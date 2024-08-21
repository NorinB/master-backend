use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserPayload {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserResponsePayload {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginUserPayload {
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: String,
    pub device_type: String,
    pub client_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginUserResponsePayload {
    pub user_id: String,
    pub name: String,
    pub email: String,
}
