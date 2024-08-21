use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerMessage {
    pub message_type: String,
    pub status: String,
    pub body: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponseBody {
    pub message: String,
    pub body: String,
}

impl ServerMessage {
    pub fn new(message_type: String, status: String, body: String) -> Self {
        Self {
            message_type,
            status,
            body,
        }
    }

    pub fn event(message_type: String, body: String) -> Self {
        Self {
            message_type,
            status: "OK".to_string(),
            body,
        }
    }

    pub fn ok_response(message_type: String, body: String) -> Self {
        Self {
            message_type: format!("response_{}", message_type),
            status: "OK".to_string(),
            body,
        }
    }

    pub fn error_response(message_type: String, body: String) -> Self {
        Self {
            message_type: format!("response_{}", message_type),
            status: "ERROR".to_string(),
            body,
        }
    }
}
