use std::sync::Arc;

use mongodb::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;

use super::server::ServerMessage;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebTransportClientBaseMessage {
    pub message_type: String,
    pub body: Value,
}

pub trait WebTransportBaseMessageHandler<Context> {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<Context>>,
    ) -> Result<ServerMessage, ServerMessage>;
}
