use axum::Json;

#[derive(serde::Serialize)]
pub struct Message {
    pub message: String,
}

pub async fn root() -> Json<Message> {
    Json(Message {
        message: "Hello, World!".to_string(),
    })
}
