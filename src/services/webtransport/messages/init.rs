use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitMessage {
    pub message_type: String,
    pub event_category: String,
    pub context_id: String,
}
