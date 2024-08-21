use std::sync::Arc;

use mongodb::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    database::collections::board::Board,
    services::webtransport::{
        context::board::{BoardContext, BoardEvent, BoardEventType},
        messages::{
            base::WebTransportBaseMessageHandler, category::WebTransportMainCategoryHandler,
            server::ServerMessage,
        },
    },
};

use super::server::ErrorResponseBody;

pub struct BoardMessage {}

impl WebTransportMainCategoryHandler<BoardContext> for BoardMessage {
    async fn handle_with_corresponding_message(
        message_subcategory: &str,
        message: Value,
        database_client: Client,
        context: Arc<Mutex<BoardContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        match message_subcategory {
            "memberadd" => {
                MemberAddMessage::handle_message(message, database_client, context).await
            }
            "memberremove" => {
                MemberRemoveMessage::handle_message(message, database_client, context).await
            }
            _ => Err(ServerMessage::error_response(
                "unknownboardcategory".to_string(),
                "Board has no such subcategory".to_string(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberAddedEventPayload {
    pub user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberAddMessage {
    pub user_id: String,
    pub board_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberAddedMessage {
    user_id: String,
}

impl WebTransportBaseMessageHandler<BoardContext> for MemberAddMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<BoardContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<MemberAddMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "memberadd".to_string(),
                    "Member Add Message is invalid".to_string(),
                ))
            }
        };
        let board = match Board::get_existing_board(body.board_id.clone(), &database_client).await {
            Ok(board) => board,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "memberadd".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Error during Current Active Member check".to_string(),
                        body: body.user_id,
                    })
                    .unwrap(),
                ));
            }
        };
        match board.allowed_members.contains(&body.user_id) {
            true => {
                return Err(ServerMessage::error_response(
                    "memberadd".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Member already part of this board".to_string(),
                        body: body.user_id,
                    })
                    .unwrap(),
                ));
            }
            false => {}
        }
        match Board::add_member(
            body.board_id.clone(),
            body.user_id.clone(),
            &database_client,
        )
        .await
        {
            Ok(_) => {
                let mut context_guard = context.lock().await;
                context_guard
                    .emit_board_event(
                        database_client.clone(),
                        body.board_id,
                        BoardEvent {
                            event_type: BoardEventType::MemberAdded,
                            body: serde_json::to_string(&MemberAddedEventPayload {
                                user_id: body.user_id.to_string(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(context_guard);
                Ok(ServerMessage::ok_response(
                    "memberadd".to_string(),
                    serde_json::to_string(&MemberAddedMessage {
                        user_id: body.user_id.to_string(),
                    })
                    .unwrap(),
                ))
            }
            Err(message) => Err(ServerMessage::error_response(
                "memberadd".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message,
                    body: body.user_id,
                })
                .unwrap(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberRemovedEventPayload {
    pub user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberRemoveMessage {
    pub user_id: String,
    pub board_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberRemovedMessage {
    user_id: String,
}

impl WebTransportBaseMessageHandler<BoardContext> for MemberRemoveMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<BoardContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<MemberRemoveMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "memberremove".to_string(),
                    "Member Remove Message is invalid".to_string(),
                ))
            }
        };
        let board = match Board::get_existing_board(body.board_id.clone(), &database_client).await {
            Ok(board) => board,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "memberremove".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Error during Current Active Member check".to_string(),
                        body: body.user_id,
                    })
                    .unwrap(),
                ));
            }
        };
        match board.allowed_members.contains(&body.user_id) {
            false => {
                return Err(ServerMessage::error_response(
                    "memberremove".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Member not part of this board".to_string(),
                        body: body.user_id,
                    })
                    .unwrap(),
                ));
            }
            true => {}
        };
        match Board::remove_member(
            body.board_id.clone(),
            body.user_id.clone(),
            &database_client,
        )
        .await
        {
            Ok(_) => {
                let mut context_guard = context.lock().await;
                context_guard
                    .emit_board_event(
                        database_client.clone(),
                        body.board_id,
                        BoardEvent {
                            event_type: BoardEventType::MemberRemoved,
                            body: serde_json::to_string(&MemberRemovedEventPayload {
                                user_id: body.user_id.to_string(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(context_guard);
                Ok(ServerMessage::ok_response(
                    "memberremove".to_string(),
                    serde_json::to_string(&MemberRemovedMessage {
                        user_id: body.user_id.to_string(),
                    })
                    .unwrap(),
                ))
            }
            Err(message) => Err(ServerMessage::error_response(
                "memberremove".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message,
                    body: body.user_id,
                })
                .unwrap(),
            )),
        }
    }
}
