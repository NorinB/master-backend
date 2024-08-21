use bson::{serde_helpers::deserialize_bson_datetime_from_rfc3339_string, DateTime};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateElementPayload {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
    pub selected: bool,
    pub locked_by: Option<String>,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub z_index: i32,
    #[serde(deserialize_with = "deserialize_bson_datetime_from_rfc3339_string")]
    pub created_at: DateTime,
    pub text: String,
    pub element_type: String,
    pub board_id: String,
    pub color: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockElementPayload {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
    pub board_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockElementPayload {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
    pub board_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockMultipleElementsPayload {
    pub ids: Vec<String>,
    pub user_id: String,
    pub board_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockMultipleElementsPayload {
    pub ids: Vec<String>,
    pub user_id: String,
    pub board_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateElementPayload {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
    pub board_id: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub rotation: Option<f32>,
    pub scale_x: Option<f32>,
    pub scale_y: Option<f32>,
    pub z_index: Option<i32>,
    pub text: Option<String>,
    pub color: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveMultipleElementsPayload {
    pub ids: Vec<String>,
    pub user_id: String,
    pub board_id: String,
    pub x_offset: f32,
    pub y_offset: f32,
}
