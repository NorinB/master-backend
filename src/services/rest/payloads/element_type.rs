use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateElementTypePayload {
    pub name: String,
    pub path: String,
}
