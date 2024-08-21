use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCreatedOrUpdatedPayload {
    pub client_id: String,
    pub user_id: String,
    pub device_type: String,
}
