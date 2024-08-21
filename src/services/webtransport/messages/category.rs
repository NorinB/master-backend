use std::sync::Arc;

use crate::services::webtransport::messages::server::ServerMessage;
use mongodb::Client;
use serde_json::Value;
use tokio::sync::Mutex;

pub enum WebTransportMessageMainCategory {
    Board,
    Element,
    ActiveMember,
    Unknown,
}

impl WebTransportMessageMainCategory {
    pub fn to_enum(string: &str) -> Self {
        match string {
            "board" => Self::Board,
            "element" => Self::Element,
            "activemember" => Self::ActiveMember,
            _ => Self::Unknown,
        }
    }
}

pub trait WebTransportMainCategoryHandler<Context> {
    async fn handle_with_corresponding_message(
        message_subcategory: &str,
        message: Value,
        database_client: Client,
        context: Arc<Mutex<Context>>,
    ) -> Result<ServerMessage, ServerMessage>;
}
