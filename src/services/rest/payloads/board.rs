use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoardRequestPayload {
    pub name: String,
    pub host: String,
}
