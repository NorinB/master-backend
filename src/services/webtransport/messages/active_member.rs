use std::sync::Arc;

use bson::doc;
use mongodb::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    database::{
        collections::active_member::{ActiveMember, CreateActiveMember, UpdateActiveMember},
        document::Document,
    },
    services::webtransport::context::active_member::{
        ActiveMemberContext, ActiveMemberEvent, ActiveMemberEventType,
    },
};

use super::{
    base::WebTransportBaseMessageHandler, category::WebTransportMainCategoryHandler,
    server::ServerMessage,
};

pub struct ActiveMemberMessage {}

impl WebTransportMainCategoryHandler<ActiveMemberContext> for ActiveMemberMessage {
    async fn handle_with_corresponding_message(
        message_subcategory: &str,
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ActiveMemberContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        match message_subcategory {
            "createactivemember" => {
                CreateActiveMemberMessage::handle_message(message, database_client, context).await
            }
            "removeactivemember" => {
                RemoveActiveMemberMessage::handle_message(message, database_client, context).await
            }
            "changeactiveboard" => {
                ChangeActiveBoardMessage::handle_message(message, database_client, context).await
            }
            "updateposition" => {
                UpdatePositionMessage::handle_message(message, database_client, context).await
            }
            _ => Err(ServerMessage::error_response(
                "unknownactivemembercategory".to_string(),
                "Active Member has no such subcategory".to_string(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedActiveMemberEventPayload {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
    pub board_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActiveMemberMessage {
    pub user_id: String,
    pub board_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedActiveMemberMessage {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
    pub board_id: String,
}

impl WebTransportBaseMessageHandler<ActiveMemberContext> for CreateActiveMemberMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ActiveMemberContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<CreateActiveMemberMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "createactivemember".to_string(),
                    "Create Active Member Message is invalid".to_string(),
                ))
            }
        };
        let create_active_member_result = ActiveMember::create_document(
            &database_client,
            CreateActiveMember {
                user_id: body.user_id.clone(),
                board_id: body.board_id.clone(),
                x: 0.0,
                y: 0.0,
            },
        )
        .await;
        match create_active_member_result {
            Ok(result) => {
                let inserted_id = result.inserted_id.as_object_id().unwrap().to_hex();
                let mut sub_context = context.lock().await;
                sub_context
                    .emit_active_member_event(
                        body.board_id.clone(),
                        ActiveMemberEvent {
                            event_type: ActiveMemberEventType::Created,
                            body: serde_json::to_string(&CreatedActiveMemberEventPayload {
                                _id: inserted_id.clone(),
                                board_id: body.board_id.clone(),
                                user_id: body.user_id.clone(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
                Ok(ServerMessage::ok_response(
                    "createactivemember".to_string(),
                    serde_json::to_string(&CreatedActiveMemberMessage {
                        _id: inserted_id,
                        board_id: body.board_id,
                        user_id: body.user_id,
                    })
                    .unwrap(),
                ))
            }
            Err(_) => Err(ServerMessage::error_response(
                "createactivemember".to_string(),
                "Error during creating active member".to_string(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovedActiveMemberEventPayload {
    pub user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveActiveMemberMessage {
    pub user_id: String,
    pub board_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovedActiveMemberMessage {
    pub user_id: String,
}

impl WebTransportBaseMessageHandler<ActiveMemberContext> for RemoveActiveMemberMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ActiveMemberContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<RemoveActiveMemberMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "removeactivemember".to_string(),
                    "Remove Active Member Message is invalid".to_string(),
                ))
            }
        };
        let query_doc = doc! {
           "userId": body.user_id.clone(),
        };
        let delete_active_member_result =
            ActiveMember::delete_document(&database_client, query_doc).await;
        match delete_active_member_result {
            Ok(result) => match result.deleted_count {
                0 => Err(ServerMessage::error_response(
                    "removeactivemember".to_string(),
                    "No Active Member found to delete".to_string(),
                )),
                _ => {
                    let mut sub_context = context.lock().await;
                    sub_context
                        .emit_active_member_event(
                            body.board_id.clone(),
                            ActiveMemberEvent {
                                event_type: ActiveMemberEventType::Removed,
                                body: serde_json::to_string(&RemovedActiveMemberEventPayload {
                                    user_id: body.user_id,
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(sub_context);
                    Ok(ServerMessage::ok_response(
                        "removeactivemember".to_string(),
                        format!("{}", result.deleted_count),
                    ))
                }
            },
            Err(_) => Err(ServerMessage::error_response(
                "removeactivemember".to_string(),
                "Error during removing of active member".to_string(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedActiveBoardEventPayload {
    pub user_id: String,
    pub new_board_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeActiveBoardMessage {
    pub user_id: String,
    pub new_board_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedActiveBoardMessage {
    pub user_id: String,
    pub new_board_id: String,
}

impl WebTransportBaseMessageHandler<ActiveMemberContext> for ChangeActiveBoardMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ActiveMemberContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<ChangeActiveBoardMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "changeactiveboard".to_string(),
                    "Change Active Board Message is invalid".to_string(),
                ))
            }
        };
        let query_doc = doc! {
           "userId": body.user_id.clone(),
        };
        let active_member = match ActiveMember::get_existing_active_member_by_user_id(
            body.user_id.clone(),
            &database_client,
        )
        .await
        {
            Ok(active_member) => active_member,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "changeactiveboard".to_string(),
                    "Error during fetching of active member".to_string(),
                ))
            }
        };
        let update_result = ActiveMember::update_document(
            &database_client,
            query_doc,
            UpdateActiveMember {
                board_id: Some(body.new_board_id.clone()),
                x: Some(0.0),
                y: Some(0.0),
            },
        )
        .await;
        match update_result {
            Ok(result) => match result.modified_count {
                0 => Err(ServerMessage::error_response(
                    "changeactiveboard".to_string(),
                    "No active member found to update".to_string(),
                )),
                _ => {
                    let mut sub_context = context.lock().await;
                    sub_context
                        .emit_active_member_event(
                            active_member.board_id,
                            ActiveMemberEvent {
                                event_type: ActiveMemberEventType::Removed,
                                body: serde_json::to_string(&RemovedActiveMemberEventPayload {
                                    user_id: body.user_id.clone(),
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    sub_context
                        .emit_active_member_event(
                            body.new_board_id.clone(),
                            ActiveMemberEvent {
                                event_type: ActiveMemberEventType::Created,
                                body: serde_json::to_string(&CreatedActiveMemberEventPayload {
                                    _id: active_member._id,
                                    user_id: body.user_id.clone(),
                                    board_id: body.new_board_id.clone(),
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(sub_context);
                    Ok(ServerMessage::ok_response(
                        "changeactiveboard".to_string(),
                        serde_json::to_string(&ChangedActiveBoardMessage {
                            user_id: body.user_id,
                            new_board_id: body.new_board_id,
                        })
                        .unwrap(),
                    ))
                }
            },
            Err(_) => Err(ServerMessage::error_response(
                "changeactiveboard".to_string(),
                "Error during change of board of active member".to_string(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedPositionEventPayload {
    pub user_id: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePositionMessage {
    pub user_id: String,
    pub board_id: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedPositionMessage {
    pub user_id: String,
    pub x: f32,
    pub y: f32,
}

impl WebTransportBaseMessageHandler<ActiveMemberContext> for UpdatePositionMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ActiveMemberContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<UpdatePositionMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "updateposition".to_string(),
                    "Update Position Message is invalid".to_string(),
                ))
            }
        };
        let query_doc = doc! {
            "userId": body.user_id.clone(),
        };
        let update_result = ActiveMember::update_document(
            &database_client,
            query_doc,
            UpdateActiveMember {
                x: Some(body.x),
                y: Some(body.y),
                board_id: None,
            },
        )
        .await;
        match update_result {
            Ok(result) => match result.modified_count {
                0 => Err(ServerMessage::error_response(
                    "updateposition".to_string(),
                    "No active member found to update".to_string(),
                )),
                _ => {
                    let mut sub_context = context.lock().await;
                    sub_context
                        .emit_active_member_event(
                            body.board_id.clone(),
                            ActiveMemberEvent {
                                event_type: ActiveMemberEventType::PositionUpdated,
                                body: serde_json::to_string(&UpdatedPositionEventPayload {
                                    user_id: body.user_id.clone(),
                                    x: body.x,
                                    y: body.y,
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(sub_context);
                    Ok(ServerMessage::ok_response(
                        "updateposition".to_string(),
                        serde_json::to_string(&UpdatedPositionMessage {
                            user_id: body.user_id,
                            x: body.x,
                            y: body.y,
                        })
                        .unwrap(),
                    ))
                }
            },
            Err(_) => Err(ServerMessage::error_response(
                "updateposition".to_string(),
                "Error during updating of position of active member".to_string(),
            )),
        }
    }
}
