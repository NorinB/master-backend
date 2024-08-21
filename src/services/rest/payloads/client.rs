use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrUpdateClientPayload {
    pub client_id: String,
    pub user_id: String,
    pub device_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrUpdateClientResponsePayload {
    pub client_id: String,
    pub user_id: String,
    pub device_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetClientReponsePayload {
    pub client_id: String,
    pub user_id: String,
    pub device_type: String,
}
