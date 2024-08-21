use rxrust::{observable::ObservableItem, subscription::Subscription};
use std::{sync::Arc, time::Duration};
use tracing::warn;

use mongodb::Client;
use tokio::sync::{Mutex, MutexGuard};
use tracing::{error, info, info_span, Instrument};
use wtransport::{
    endpoint::{endpoint_side::Server, IncomingSession},
    error::{ConnectionError, StreamReadError, StreamWriteError},
    Endpoint, Identity, RecvStream, SendStream, ServerConfig,
};

use crate::{
    database::collections::board::Board,
    services::webtransport::messages::base::WebTransportClientBaseMessage, AppState,
};

use super::{
    context::{
        active_member::ActiveMemberContext, base::EventCategory, board::BoardContext,
        client::ClientContext, element::ElementContext,
    },
    messages::{
        active_member::ActiveMemberMessage,
        board::BoardMessage,
        category::{WebTransportMainCategoryHandler, WebTransportMessageMainCategory},
        element::ElementMessage,
        init::InitMessage,
        server::ServerMessage,
    },
};

pub struct WebTransportServer {
    endpoint: Endpoint<Server>,
    pub local_port: u16,
    state: AppState,
}

impl WebTransportServer {
    const PORT: u16 = 3031;

    pub fn new(state: AppState, identity: Identity) -> anyhow::Result<Self> {
        let local_port = Self::PORT;
        let config = Self::build_config(&identity);
        let endpoint = Endpoint::server(config)?;
        Ok(Self {
            endpoint,
            local_port,
            state,
        })
    }

    fn build_config(identity: &Identity) -> ServerConfig {
        ServerConfig::builder()
            .with_bind_default(Self::PORT)
            .with_identity(identity)
            .keep_alive_interval(Some(Duration::from_secs(3)))
            .build()
    }

    pub fn local_port(&self) -> u16 {
        self.endpoint.local_addr().unwrap().port()
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        info!("WebTransport server running on port: {}", self.local_port());

        for id in 0.. {
            let incoming_session = self.endpoint.accept().await;
            let client = self.state.database_client.clone();
            let board_context = self.state.board_context.clone();
            let element_context = self.state.element_context.clone();
            let client_context = self.state.client_context.clone();
            let active_member_context = self.state.active_member_context.clone();
            tokio::spawn(async move {
                {
                    let board_context = board_context.clone();
                    let element_context = element_context.clone();
                    let client_context = client_context.clone();
                    let active_member_context = active_member_context.clone();
                    let _ = WebTransportServer::handle_incoming_session(
                        board_context,
                        element_context,
                        client_context,
                        active_member_context,
                        client,
                        incoming_session,
                    )
                    .await
                    .instrument(info_span!("Connection", id));
                }
            });
        }

        Ok(())
    }

    async fn handle_incoming_session(
        board_context: Arc<Mutex<BoardContext>>,
        element_context: Arc<Mutex<ElementContext>>,
        client_context: Arc<Mutex<ClientContext>>,
        active_member_context: Arc<Mutex<ActiveMemberContext>>,
        database_client: Client,
        incoming_session: IncomingSession,
    ) -> Result<(), ()> {
        info!("Waiting for session request...");

        let session_request = match incoming_session.await {
            Ok(session_request) => session_request,
            Err(err) => {
                error!("{}", err);
                error!("Error during incoming session await");
                return Err(());
            }
        };

        info!(
            "New session: Authority: '{}', Path: '{}'",
            session_request.authority(),
            session_request.path(),
        );

        let connection = match session_request.accept().await {
            Ok(connection) => connection,
            Err(_) => {
                error!("Error during session request acception");
                return Err(());
            }
        };

        info!("Waiting for data from client...");

        loop {
            info!("Waiting for new Connection...");
            let stream = match connection.accept_bi().await {
                Ok(stream) => (
                    Arc::new(Mutex::new(stream.0)),
                    Arc::new(Mutex::new(stream.1)),
                ),
                Err(err) => {
                    match err {
                        ConnectionError::TimedOut => {
                            error!("Connection timed out");
                        }
                        ConnectionError::ConnectionClosed(connection_close) => {
                            error!("Connection closed, {:?}", connection_close);
                        }
                        ConnectionError::LocallyClosed => {
                            error!("Connection locally closed");
                        }
                        _ => {
                            error!("Connection acception error");
                        }
                    };
                    return Err(());
                }
            };
            let database_client = database_client.clone();
            info!("Accepted BI stream");
            info!("Awaiting first message");
            let mut buffer = vec![0; 65536].into_boxed_slice();
            let init_connection_bytes = stream.1.lock().await.read(&mut buffer).await;
            info!("Got first message");
            let init_connection_length = match init_connection_bytes {
                Ok(bytes_read) => match bytes_read {
                    Some(bytes_read) => bytes_read,
                    None => {
                        let message = "Error during Init Message Byte Reading".to_string();
                        error!("{}", message.clone());
                        return Err(());
                    }
                },
                Err(_) => {
                    let message = "Error during reading of init connection bytes".to_string();
                    error!("{}", message.clone());
                    return Err(());
                }
            };
            info!("Init connection bytes have been read");
            let message = match std::str::from_utf8(&buffer[..init_connection_length]) {
                Ok(message) => message,
                Err(_) => {
                    let message =
                        "Error during String conversion of init message Bytes!".to_string();
                    error!("{}", message.clone());
                    return Err(());
                }
            };
            let mut board_context_guard = board_context.lock().await;
            let mut element_context_guard = element_context.lock().await;
            let mut client_context_guard = client_context.lock().await;
            let mut active_member_context_guard = active_member_context.lock().await;
            let (subject_id, event_category) =
                match WebTransportServer::init_with_id_and_event_category(
                    &mut board_context_guard,
                    &mut element_context_guard,
                    &mut client_context_guard,
                    &mut active_member_context_guard,
                    database_client.clone(),
                    message,
                )
                .await
                {
                    Ok(board_id) => board_id,
                    Err(message) => {
                        error!("{}", message.clone());
                        return Err(());
                    }
                };
            drop(board_context_guard);
            drop(element_context_guard);
            drop(client_context_guard);
            drop(active_member_context_guard);
            let _ = stream
                .0
                .lock()
                .await
                .write_all(
                    serde_json::to_string(&ServerMessage::new(
                        "success".to_string(),
                        "OK".to_string(),
                        "initialized".to_string(),
                    ))
                    .unwrap()
                    .as_bytes(),
                )
                .await;
            match event_category {
                EventCategory::Board => {
                    let context = board_context.clone();
                    let mut board_context_guard = context.lock().await;
                    let copied_send_stream = stream.0.clone();
                    let subscription = board_context_guard
                        .get_or_create_subject(subject_id.clone())
                        .subject
                        .clone()
                        .subscribe(move |event| {
                            let another_copy_of_stream = copied_send_stream.clone();
                            tokio::spawn(async move {
                                WebTransportServer::send_message_to_stream(
                                    another_copy_of_stream.lock().await,
                                    ServerMessage::event(event.event_type.to_string(), event.body),
                                )
                                .await;
                            });
                        });
                    drop(board_context_guard);
                    let cloned_board_context = board_context.clone();
                    let cloned_element_context = element_context.clone();
                    let cloned_active_member_context = active_member_context.clone();
                    tokio::spawn(async move {
                        match WebTransportServer::handle_stream(
                            database_client,
                            (stream.0, stream.1),
                            subscription,
                            cloned_board_context,
                            cloned_element_context,
                            cloned_active_member_context,
                        )
                        .await
                        {
                            Ok(_) => {
                                warn!("Connection closed");
                            }
                            Err(_) => {
                                error!("Error during handling of Bi-Stream");
                            }
                        }
                    });
                }
                EventCategory::Element => {
                    let context = element_context.clone();
                    let mut element_context_guard = context.lock().await;
                    let copied_send_stream = stream.0.clone();
                    let subscription = element_context_guard
                        .get_or_create_subject(subject_id.clone())
                        .subject
                        .clone()
                        .subscribe(move |event| {
                            let another_copy_of_stream = copied_send_stream.clone();
                            tokio::spawn(async move {
                                WebTransportServer::send_message_to_stream(
                                    another_copy_of_stream.lock().await,
                                    ServerMessage::event(event.event_type.to_string(), event.body),
                                )
                                .await;
                            });
                        });
                    drop(element_context_guard);
                    let cloned_board_context = board_context.clone();
                    let cloned_element_context = element_context.clone();
                    let cloned_active_member_context = active_member_context.clone();
                    tokio::spawn(async move {
                        match WebTransportServer::handle_stream(
                            database_client,
                            (stream.0, stream.1),
                            subscription,
                            cloned_board_context,
                            cloned_element_context,
                            cloned_active_member_context,
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(_) => {
                                error!("Error during handling of Bi-Stream");
                            }
                        }
                    });
                }
                EventCategory::Client => {
                    let context = client_context.clone();
                    let mut client_context_guard = context.lock().await;
                    let copied_send_stream = stream.0.clone();
                    let subscription = client_context_guard
                        .get_or_create_subject(subject_id.clone())
                        .subject
                        .clone()
                        .subscribe(move |event| {
                            let another_copy_of_stream = copied_send_stream.clone();
                            tokio::spawn(async move {
                                WebTransportServer::send_message_to_stream(
                                    another_copy_of_stream.lock().await,
                                    ServerMessage::event(event.event_type.to_string(), event.body),
                                )
                                .await;
                            });
                        });
                    drop(client_context_guard);
                    let cloned_board_context = board_context.clone();
                    let cloned_element_context = element_context.clone();
                    let cloned_active_member_context = active_member_context.clone();
                    tokio::spawn(async move {
                        match WebTransportServer::handle_stream(
                            database_client,
                            (stream.0, stream.1),
                            subscription,
                            cloned_board_context,
                            cloned_element_context,
                            cloned_active_member_context,
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(_) => {
                                error!("Error during handling of Bi-Stream");
                            }
                        }
                    });
                }
                EventCategory::ActiveMember => {
                    let context = active_member_context.clone();
                    let mut active_member_context_guard = context.lock().await;
                    let copied_send_stream = stream.0.clone();
                    let subscription = active_member_context_guard
                        .get_or_create_subject(subject_id.clone())
                        .subject
                        .clone()
                        .subscribe(move |event| {
                            let another_copy_of_stream = copied_send_stream.clone();
                            tokio::spawn(async move {
                                WebTransportServer::send_message_to_stream(
                                    another_copy_of_stream.lock().await,
                                    ServerMessage::event(
                                        event.event_type.to_string(),
                                        event.body.to_string(),
                                    ),
                                )
                                .await;
                            });
                        });
                    drop(active_member_context_guard);
                    let cloned_board_context = board_context.clone();
                    let cloned_element_context = element_context.clone();
                    let cloned_active_member_context = active_member_context.clone();
                    tokio::spawn(async move {
                        match WebTransportServer::handle_stream(
                            database_client,
                            (stream.0, stream.1),
                            subscription,
                            cloned_board_context,
                            cloned_element_context,
                            cloned_active_member_context,
                        )
                        .await
                        {
                            Ok(_) => {
                                warn!("Connection closed");
                            }
                            Err(_) => {
                                error!("Error during handling of Bi-Stream");
                            }
                        }
                    });
                }
            };
        }
    }

    async fn handle_stream(
        database_client: Client,
        stream: (Arc<Mutex<SendStream>>, Arc<Mutex<RecvStream>>),
        subscription: impl Subscription,
        board_context: Arc<Mutex<BoardContext>>,
        element_context: Arc<Mutex<ElementContext>>,
        active_member_context: Arc<Mutex<ActiveMemberContext>>,
    ) -> Result<(), String> {
        loop {
            let mut buffer = vec![0; 65536].into_boxed_slice();
            let bytes_read = stream.1.lock().await.read(&mut buffer).await;
            let bytes_read = match bytes_read {
                Ok(bytes_read) => match bytes_read {
                    Some(bytes_read) => bytes_read,
                    None => continue,
                },
                Err(error) => {
                    let message = match error {
                        StreamReadError::NotConnected => {
                            "Cannot read Stream, Stream lost connection".to_string()
                        }
                        StreamReadError::Reset(reset) => {
                            format!("Connection has been reset: {:?}", reset)
                        }
                        StreamReadError::QuicProto => {
                            "Stream could not be read because of quic protocol error".to_string()
                        }
                    };
                    subscription.unsubscribe();
                    error!("{}", message.clone());
                    return Err(message);
                }
            };
            let str_data = match std::str::from_utf8(&buffer[..bytes_read]) {
                Ok(str_data) => str_data,
                Err(_) => {
                    subscription.unsubscribe();
                    let message = "Error during parsing of incoming bytes".to_string();
                    error!("{}", message.clone());
                    return Err(message);
                }
            };
            let json_message = match serde_json::from_str::<WebTransportClientBaseMessage>(str_data)
            {
                Ok(parsed_json) => parsed_json,
                Err(_) => {
                    let message =
                        "Error during parsing of WebTransportClientBaseMessage JSON Message";
                    error!("{}", message.to_string());
                    match stream
                        .0
                        .lock()
                        .await
                        .write_all(
                            serde_json::to_string(&ServerMessage::error_response(
                                "basemessage".to_string(),
                                message.to_string(),
                            ))
                            .unwrap()
                            .as_bytes(),
                        )
                        .await
                    {
                        Ok(_) => continue,
                        Err(error) => {
                            let message = match error {
                                StreamWriteError::NotConnected => {
                                    "Cannot write Stream, Stream lost connection".to_string()
                                }
                                StreamWriteError::Stopped(stopped) => {
                                    format!("Stream writing stopped, {:?}", stopped)
                                }
                                StreamWriteError::QuicProto => {
                                    "Stream could not be written because of quic protocol error"
                                        .to_string()
                                }
                            };
                            error!("{}", message.clone());
                            subscription.unsubscribe();
                            return Err(message);
                        }
                    }
                }
            };
            info!("Recieved (bi) '{str_data}' from client");
            let response_message = Self::handle_with_corresponding_category(
                json_message.clone(),
                database_client.clone(),
                board_context.clone(),
                element_context.clone(),
                active_member_context.clone(),
            )
            .await;
            match response_message {
                Ok(message) => {
                    info!(
                        "WebTransport Antwort vom Server: type: {}, body: {}",
                        message.message_type, message.body
                    );
                    match stream
                        .0
                        .lock()
                        .await
                        .write_all(serde_json::to_string(&message).unwrap().as_bytes())
                        .await
                    {
                        Ok(_) => continue,
                        Err(error) => {
                            let message = match error {
                                StreamWriteError::NotConnected => {
                                    "Cannot write Stream, Stream lost connection".to_string()
                                }
                                StreamWriteError::Stopped(stopped) => {
                                    format!("Stream writing stopped, {:?}", stopped)
                                }
                                StreamWriteError::QuicProto => {
                                    "Stream could not be written because of quic protocol error"
                                        .to_string()
                                }
                            };
                            error!("{}", message.clone());
                            subscription.unsubscribe();
                            return Err(message);
                        }
                    }
                }
                Err(error_message) => match stream
                    .0
                    .lock()
                    .await
                    .write_all(serde_json::to_string(&error_message).unwrap().as_bytes())
                    .await
                {
                    Ok(_) => continue,
                    Err(error) => {
                        subscription.unsubscribe();
                        let message = format!("{:?}", error);
                        error!("{}", message);
                        return Err(message);
                    }
                },
            };
        }
    }

    async fn send_message_to_stream(
        mut stream: MutexGuard<'_, SendStream>,
        message: ServerMessage,
    ) {
        match stream
            .write_all(serde_json::to_string(&message).unwrap().as_bytes())
            .await
        {
            Ok(_) => (),
            Err(error) => {
                let message = match error {
                    StreamWriteError::NotConnected => {
                        "Cannot write Stream, Stream lost connection".to_string()
                    }
                    StreamWriteError::Stopped(stopped) => {
                        format!("Stream writing stopped, {:?}", stopped)
                    }
                    StreamWriteError::QuicProto => {
                        "Stream could not be written because of quic protocol error".to_string()
                    }
                };
                error!("{}", message);
            }
        }
    }

    async fn init_with_id_and_event_category<'a, 'b>(
        board_context: &'a mut BoardContext,
        element_context: &'a mut ElementContext,
        client_context: &'a mut ClientContext,
        active_member_context: &'a mut ActiveMemberContext,
        database_client: Client,
        message: &'b str,
    ) -> Result<(String, EventCategory), String> {
        let init_message = match serde_json::from_str::<InitMessage>(message) {
            Ok(init_message) => init_message,
            Err(error) => {
                info!("{:?}", error);
                return Err(
                    "Init Message couldn't be deserialized into the InitMessage struct".to_string(),
                );
            }
        };
        if init_message.message_type != *"init".to_string() {
            return Err("Init Message: `messageType` != 'init'".to_string());
        }
        let event_category =
            match EventCategory::get_category_by_string(init_message.event_category) {
                Ok(category) => category,
                Err(_) => {
                    return Err("Invalid event category".to_string());
                }
            };
        let subject_id = match event_category {
            EventCategory::Client => init_message.context_id.clone(),
            _ => match Board::get_existing_board(init_message.context_id.clone(), &database_client)
                .await
            {
                Ok(board) => board._id,
                Err(_) => {
                    return Err(format!(
                        "No Board found with the Board Id: {}",
                        init_message.context_id
                    ));
                }
            },
        };
        match event_category {
            EventCategory::Board => Ok((
                board_context.get_or_create_subject_return_board_id(subject_id),
                event_category,
            )),
            EventCategory::Client => Ok((
                client_context.get_or_create_subject_return_user_id(subject_id),
                event_category,
            )),
            EventCategory::ActiveMember => Ok((
                active_member_context.get_or_create_subject_return_board_id(subject_id),
                event_category,
            )),
            EventCategory::Element => Ok((
                element_context.get_or_create_subject_return_board_id(subject_id),
                event_category,
            )),
        }
    }

    async fn handle_with_corresponding_category(
        json: WebTransportClientBaseMessage,
        database_client: Client,
        board_context: Arc<Mutex<BoardContext>>,
        element_context: Arc<Mutex<ElementContext>>,
        active_member_context: Arc<Mutex<ActiveMemberContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let substrings = json
            .message_type
            .split('_')
            .map(|substring| substring.to_string())
            .collect::<Vec<String>>();
        if substrings.len() <= 1 {
            return Err(ServerMessage::error_response(
                "messagetypeparsing".to_string(),
                "No actual message type provided".to_string(),
            ));
        }
        let message_category =
            WebTransportMessageMainCategory::to_enum(substrings.first().unwrap());
        let message_subcategory = substrings.get(1).unwrap().as_str();
        match message_category {
            WebTransportMessageMainCategory::Board => {
                BoardMessage::handle_with_corresponding_message(
                    message_subcategory,
                    json.body,
                    database_client,
                    board_context,
                )
                .await
            }
            WebTransportMessageMainCategory::Element => {
                ElementMessage::handle_with_corresponding_message(
                    message_subcategory,
                    json.body,
                    database_client,
                    element_context,
                )
                .await
            }
            WebTransportMessageMainCategory::ActiveMember => {
                ActiveMemberMessage::handle_with_corresponding_message(
                    message_subcategory,
                    json.body,
                    database_client,
                    active_member_context,
                )
                .await
            }
            WebTransportMessageMainCategory::Unknown => Err(ServerMessage::error_response(
                "messagecategory".to_string(),
                "Message Main Category unknown".to_string(),
            )),
        }
    }
}
