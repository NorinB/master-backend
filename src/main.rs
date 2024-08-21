use std::process::exit;
use std::sync::Arc;

use dotenvy::dotenv;
use mongodb::bson::doc;
use mongodb::{options::ClientOptions, Client};
use services::webtransport::context::active_member::ActiveMemberContext;
use services::webtransport::context::board::BoardContext;
use services::webtransport::context::client::ClientContext;
use services::webtransport::context::element::ElementContext;
use tokio::sync::Mutex;
use tracing::{error, info};
use utils::element_types::generate_elements;
use wtransport::tls::Sha256DigestFmt;
use wtransport::Identity;

mod database {
    pub mod config;
    pub mod document;
    pub mod validator;
    pub mod collections {
        pub mod active_member;
        pub mod board;
        pub mod client;
        pub mod element;
        pub mod element_type;
        pub mod user;
    }
}
mod services {
    pub mod webtransport {
        pub mod messages {
            pub mod active_member;
            pub mod base;
            pub mod board;
            pub mod category;
            pub mod client;
            pub mod element;
            pub mod init;
            pub mod server;
        }
        pub mod context {
            pub mod active_member;
            pub mod base;
            pub mod board;
            pub mod client;
            pub mod element;
        }
        pub mod server;
    }
    pub mod rest {
        pub mod server;
        pub mod endpoints {
            pub mod active_member;
            pub mod board;
            pub mod client;
            pub mod element;
            pub mod element_type;
            pub mod ping;
            pub mod user;
        }
        pub mod payloads {
            pub mod active_member;
            pub mod board;
            pub mod client;
            pub mod element;
            pub mod element_type;
            pub mod user;
        }
    }
}
mod utils {
    pub mod check_request_body;
    pub mod element_types;
    pub mod generate_certificate;
    pub mod logging;
}
use crate::database::config::DatabaseConfig;
use crate::services::rest::server::RestServer;
use crate::services::webtransport::server::WebTransportServer;
use crate::utils::{generate_certificate::generate_certificate, logging::init_logging};

#[derive(Clone)]
pub struct AppState {
    database_client: Client,
    board_context: Arc<Mutex<BoardContext>>,
    element_context: Arc<Mutex<ElementContext>>,
    client_context: Arc<Mutex<ClientContext>>,
    active_member_context: Arc<Mutex<ActiveMemberContext>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().expect("Failed to load .env file");

    init_logging();

    let database_config = DatabaseConfig::new();
    let mut client_options = ClientOptions::parse(database_config.uri).await.unwrap();
    client_options.connect_timeout = database_config.connection_timeout;
    client_options.max_pool_size = database_config.max_pool_size;
    client_options.min_pool_size = database_config.min_pool_size;
    client_options.compressors = database_config.compressors;
    let client = Client::with_options(client_options).unwrap();

    client
        .database("admin")
        .run_command(doc! { "ping": 1 }, None)
        .await?;
    info!("Connection to MongoDB successful!");
    client
        .database("master")
        .run_command(doc! {"ping": 1}, None)
        .await?;
    info!("master Database ready");

    if !std::path::Path::new("certificates/key.pem").is_file() {
        info!("Generiere Zeritifikat");
        let _ = generate_certificate().await;
    }
    let identity = Identity::load_pemfiles(
        std::path::Path::new("certificates/cert.pem"),
        std::path::Path::new("certificates/key.pem"),
    )
    .await?;
    info!(
        "Certificate hash: {}",
        identity.certificate_chain().as_slice()[0]
            .hash()
            .fmt(Sha256DigestFmt::BytesArray)
    );

    match generate_elements(&client).await {
        Ok(_) => {}
        Err(error_message) => {
            error!("Error during Element Type creation: {}", error_message);
            exit(1);
        }
    };

    let state = AppState {
        database_client: client,
        board_context: Arc::new(Mutex::new(BoardContext::new())),
        element_context: Arc::new(Mutex::new(ElementContext::new())),
        client_context: Arc::new(Mutex::new(ClientContext::new())),
        active_member_context: Arc::new(Mutex::new(ActiveMemberContext::new())),
    };

    let webtransport_server = WebTransportServer::new(state.clone(), identity)?;
    let rest_server = RestServer::new(state).await?;
    info!(
        "Servers are running. REST on: http://127.0.0.1:{}, WebTransport on: https://127.0.0.1:{}",
        rest_server.local_port, webtransport_server.local_port
    );

    tokio::select! {
        result = rest_server.serve() => {
            error!("HTTP server: {:?}", result);
        }
        result = webtransport_server.serve() => {
            error!("WebTransport server: {:?}", result);
        }
    }

    Ok(())
}
