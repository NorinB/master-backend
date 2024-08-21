use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActiveMemberPayload {
    pub user_id: String,
    pub board_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeActiveBoardPayload {
    pub user_id: String,
    pub new_board_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePostionPayload {
    pub user_id: String,
    pub board_id: String,
    pub x: f32,
    pub y: f32,
}
