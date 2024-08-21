use std::{str::FromStr, sync::Arc};

use bson::{
    doc,
    oid::ObjectId,
    serde_helpers::{
        deserialize_bson_datetime_from_rfc3339_string, serialize_bson_datetime_as_rfc3339_string,
    },
    DateTime,
};
use futures::TryStreamExt;
use mongodb::{results::UpdateResult, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    database::{
        collections::element::{CreateElement, Element, UpdateElement},
        document::Document,
    },
    services::webtransport::context::element::{ElementContext, ElementEvent, ElementEventType},
};

use super::{
    base::WebTransportBaseMessageHandler,
    category::WebTransportMainCategoryHandler,
    server::{ErrorResponseBody, ServerMessage},
};

pub struct ElementMessage {}

impl WebTransportMainCategoryHandler<ElementContext> for ElementMessage {
    async fn handle_with_corresponding_message(
        message_subcategory: &str,
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ElementContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        match message_subcategory {
            "createelement" => {
                CreateElementMessage::handle_message(message, database_client, context).await
            }
            "removeelement" => {
                RemoveElementMessage::handle_message(message, database_client, context).await
            }
            "lockelement" => {
                LockElementMessage::handle_message(message, database_client, context).await
            }
            "unlockelement" => {
                UnlockElementMessage::handle_message(message, database_client, context).await
            }
            "lockelements" => {
                LockElementsMessage::handle_message(message, database_client, context).await
            }
            "unlockelements" => {
                UnlockElementsMessage::handle_message(message, database_client, context).await
            }
            "updateelement" => {
                UpdateElementMessage::handle_message(message, database_client, context).await
            }
            "moveelements" => {
                MoveElementsMessage::handle_message(message, database_client, context).await
            }
            _ => Err(ServerMessage::error_response(
                "unknownelementcategory".to_string(),
                "Element has no such subcategory".to_string(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementCreatedEventPayload {
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
    #[serde(serialize_with = "serialize_bson_datetime_as_rfc3339_string")]
    pub created_at: DateTime,
    pub text: String,
    pub element_type: String,
    pub board_id: String,
    pub color: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateElementMessage {
    #[serde(rename = "_id")]
    pub _id: String,
    pub selected: bool,
    pub user_id: String,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementCreatedMessage {
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
    #[serde(serialize_with = "serialize_bson_datetime_as_rfc3339_string")]
    pub created_at: DateTime,
    pub text: String,
    pub element_type: String,
    pub board_id: String,
    pub color: String,
}

impl WebTransportBaseMessageHandler<ElementContext> for CreateElementMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ElementContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<CreateElementMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "createelement".to_string(),
                    "Create Element Message is invalid".to_string(),
                ));
            }
        };
        let create_element = CreateElement {
            _id: body._id.clone(),
            board_id: body.board_id.clone(),
            selected: body.selected,
            locked_by: body.locked_by,
            rotation: body.rotation,
            scale_x: body.scale_x,
            scale_y: body.scale_y,
            z_index: body.z_index,
            x: body.x,
            y: body.y,
            element_type: body.element_type.clone(),
            text: body.text.clone(),
            created_at: body.created_at,
            color: body.color,
        };
        match Element::create_document(&database_client, create_element.clone()).await {
            Ok(result) => {
                let inserted_id = result.inserted_id.as_object_id().unwrap().to_hex();
                let mut context_guard = context.lock().await;
                context_guard
                    .emit_element_event(
                        body.board_id,
                        ElementEvent {
                            event_type: ElementEventType::Created,
                            body: serde_json::to_string(&ElementCreatedEventPayload {
                                _id: inserted_id.clone(),
                                user_id: body.user_id.clone(),
                                selected: create_element.selected,
                                locked_by: create_element.locked_by.clone(),
                                x: create_element.x,
                                y: create_element.y,
                                rotation: create_element.rotation,
                                scale_x: create_element.scale_x,
                                scale_y: create_element.scale_y,
                                z_index: create_element.z_index,
                                created_at: create_element.created_at,
                                text: create_element.text.clone(),
                                element_type: create_element.element_type.clone(),
                                board_id: create_element.board_id.clone(),
                                color: create_element.color.clone(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(context_guard);
                Ok(ServerMessage::ok_response(
                    "createelement".to_string(),
                    serde_json::to_string(&ElementCreatedMessage {
                        _id: inserted_id.clone(),
                        user_id: body.user_id,
                        selected: create_element.selected,
                        locked_by: create_element.locked_by,
                        x: create_element.x,
                        y: create_element.y,
                        rotation: create_element.rotation,
                        scale_x: create_element.scale_x,
                        scale_y: create_element.scale_y,
                        z_index: create_element.z_index,
                        created_at: create_element.created_at,
                        text: create_element.text,
                        element_type: create_element.element_type,
                        board_id: create_element.board_id,
                        color: create_element.color,
                    })
                    .unwrap(),
                ))
            }
            Err(_) => Err(ServerMessage::error_response(
                "createelement".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "Element could not be created".to_string(),
                    body: body._id,
                })
                .unwrap(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementRemovedEventPayload {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveElementMessage {
    #[serde(rename = "_id")]
    pub _id: String,
    pub board_id: String,
    pub user_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementRemovedMessage {
    #[serde(rename = "_id")]
    _id: String,
}

impl WebTransportBaseMessageHandler<ElementContext> for RemoveElementMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ElementContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<RemoveElementMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "removeelement".to_string(),
                    "Remove Element Message is invalid".to_string(),
                ))
            }
        };
        match Element::delete_document(
            &database_client,
            doc! { "_id": ObjectId::from_str(body._id.as_str()).unwrap() },
        )
        .await
        {
            Ok(result) => match result.deleted_count {
                0 => Err(ServerMessage::error_response(
                    "removeelement".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "No Element found to delete".to_string(),
                        body: body._id,
                    })
                    .unwrap(),
                )),
                _ => {
                    let mut context_guard = context.lock().await;
                    context_guard
                        .emit_element_event(
                            body.board_id.clone(),
                            ElementEvent {
                                event_type: ElementEventType::Removed,
                                body: serde_json::to_string(&ElementRemovedEventPayload {
                                    _id: body._id.clone(),
                                    user_id: body.user_id.clone(),
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(context_guard);
                    Ok(ServerMessage::ok_response(
                        "removeelement".to_string(),
                        serde_json::to_string(&ElementRemovedMessage { _id: body._id }).unwrap(),
                    ))
                }
            },
            Err(_) => Err(ServerMessage::error_response(
                "removeelement".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "Element could not be deleted".to_string(),
                    body: body._id,
                })
                .unwrap(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementLockedEventPayload {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockElementMessage {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
    pub board_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementLockedMessage {
    #[serde(rename = "_id")]
    _id: String,
    user_id: String,
}

impl WebTransportBaseMessageHandler<ElementContext> for LockElementMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ElementContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<LockElementMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "lockelement".to_string(),
                    "Lock Element Message is invalid".to_string(),
                ))
            }
        };
        let query_doc = doc! {
            "_id": ObjectId::from_str(body._id.as_str()).unwrap()
        };
        let found_element_result = Element::get_document(&database_client, query_doc.clone()).await;
        match found_element_result {
            Ok(element) => match element {
                Some(element) => {
                    if let Some(locked_by) = element.locked_by {
                        if locked_by != body.user_id {
                            return Err(ServerMessage::error_response(
                                "lockelement".to_string(),
                                serde_json::to_string(&ErrorResponseBody {
                                    message: "Element already locked by someone else".to_string(),
                                    body: body._id,
                                })
                                .unwrap(),
                            ));
                        } else {
                            return Err(ServerMessage::error_response(
                                "lockelement".to_string(),
                                serde_json::to_string(&ErrorResponseBody {
                                    message: "Element already locked by yourself".to_string(),
                                    body: body._id,
                                })
                                .unwrap(),
                            ));
                        }
                    }
                }
                None => {
                    return Err(ServerMessage::error_response(
                        "lockelement".to_string(),
                        serde_json::to_string(&ErrorResponseBody {
                            message: "Element not found".to_string(),
                            body: body._id,
                        })
                        .unwrap(),
                    ));
                }
            },
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "lockelement".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Error during Element existing check".to_string(),
                        body: body._id,
                    })
                    .unwrap(),
                ));
            }
        };
        let update_result = Element::update_document(
            &database_client,
            query_doc,
            UpdateElement {
                selected: None,
                locked_by: Some(Some(body.user_id.clone())),
                x: None,
                y: None,
                rotation: None,
                scale_x: None,
                scale_y: None,
                z_index: None,
                text: None,
                color: None,
            },
        )
        .await;
        match update_result {
            Ok(result) => match result.modified_count {
                0 => Err(ServerMessage::error_response(
                    "lockelement".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "No Element found to lock".to_string(),
                        body: body._id,
                    })
                    .unwrap(),
                )),
                _ => {
                    let mut context_guard = context.lock().await;
                    context_guard
                        .emit_element_event(
                            body.board_id.clone(),
                            ElementEvent {
                                event_type: ElementEventType::Locked,
                                body: serde_json::to_string(&ElementLockedEventPayload {
                                    _id: body._id.clone(),
                                    user_id: body.user_id.clone(),
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(context_guard);
                    Ok(ServerMessage::ok_response(
                        "lockelement".to_string(),
                        serde_json::to_string(&ElementLockedMessage {
                            _id: body._id,
                            user_id: body.user_id,
                        })
                        .unwrap(),
                    ))
                }
            },
            Err(_) => Err(ServerMessage::error_response(
                "lockelement".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "Element could not be locked".to_string(),
                    body: body._id,
                })
                .unwrap(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementUnlockedEventPayload {
    #[serde(rename = "_id")]
    pub _id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockElementMessage {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
    pub board_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementUnlockedMessage {
    #[serde(rename = "_id")]
    _id: String,
}

impl WebTransportBaseMessageHandler<ElementContext> for UnlockElementMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ElementContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<UnlockElementMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "unlockelement".to_string(),
                    "Unlock Element Message is invalid".to_string(),
                ))
            }
        };
        let query_doc = doc! {
            "_id": ObjectId::from_str(body._id.as_str()).unwrap()
        };
        let found_element_result = Element::get_document(&database_client, query_doc.clone()).await;
        match found_element_result {
            Ok(element) => match element {
                Some(element) => match element.locked_by {
                    Some(locked_by) => {
                        if locked_by != body.user_id {
                            return Err(ServerMessage::error_response(
                                "unlockelement".to_string(),
                                serde_json::to_string(&ErrorResponseBody {
                                    message: "Element currently locked by someone else".to_string(),
                                    body: body._id,
                                })
                                .unwrap(),
                            ));
                        }
                    }
                    None => {
                        return Err(ServerMessage::error_response(
                            "unlockelement".to_string(),
                            serde_json::to_string(&ErrorResponseBody {
                                message: "Element already unlocked".to_string(),
                                body: body._id,
                            })
                            .unwrap(),
                        ));
                    }
                },
                None => {
                    return Err(ServerMessage::error_response(
                        "unlockelement".to_string(),
                        serde_json::to_string(&ErrorResponseBody {
                            message: "Element not found".to_string(),
                            body: body._id,
                        })
                        .unwrap(),
                    ));
                }
            },
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "unlockelement".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Error during Element existing check".to_string(),
                        body: body._id,
                    })
                    .unwrap(),
                ));
            }
        };
        let update_result = Element::update_document(
            &database_client,
            query_doc,
            UpdateElement {
                selected: None,
                locked_by: Some(None),
                x: None,
                y: None,
                rotation: None,
                scale_x: None,
                scale_y: None,
                z_index: None,
                text: None,
                color: None,
            },
        )
        .await;
        match update_result {
            Ok(result) => match result.modified_count {
                0 => Err(ServerMessage::error_response(
                    "unlockelement".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "No Element found to unlock".to_string(),
                        body: body._id,
                    })
                    .unwrap(),
                )),
                _ => {
                    let mut context_guard = context.lock().await;
                    context_guard
                        .emit_element_event(
                            body.board_id.clone(),
                            ElementEvent {
                                event_type: ElementEventType::Unlocked,
                                body: serde_json::to_string(&ElementUnlockedEventPayload {
                                    _id: body._id.clone(),
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(context_guard);
                    Ok(ServerMessage::ok_response(
                        "unlockelement".to_string(),
                        serde_json::to_string(&ElementUnlockedMessage { _id: body._id }).unwrap(),
                    ))
                }
            },
            Err(_) => Err(ServerMessage::error_response(
                "unlockelement".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "Element could not be unlocked".to_string(),
                    body: body._id,
                })
                .unwrap(),
            )),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockElementsMessage {
    pub ids: Vec<String>,
    pub user_id: String,
    pub board_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementsLockedMessage {
    ids: Vec<String>,
    user_id: String,
}

impl WebTransportBaseMessageHandler<ElementContext> for LockElementsMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ElementContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<LockElementsMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "lockelements".to_string(),
                    "Lock Elements Message is invalid".to_string(),
                ))
            }
        };
        let query_doc = doc! {
            "_id": doc! { "$in": body.ids.iter().map(|id| ObjectId::from_str(id.as_str()).unwrap()).collect::<Vec<ObjectId>>() }
        };
        let found_element_result =
            Element::get_multiple_documents(&database_client, query_doc.clone()).await;
        let found_elements = match found_element_result {
            Ok(element_cursor) => {
                let retrieved_elements = element_cursor.try_collect::<Vec<Element>>().await;
                match retrieved_elements {
                    Ok(retrieved_elements) => match retrieved_elements.len() {
                        0 => {
                            return Err(ServerMessage::error_response(
                                "lockelements".to_string(),
                                serde_json::to_string(&ErrorResponseBody {
                                    message: "No Elements found".to_string(),
                                    body: serde_json::to_string(&body.ids).unwrap(),
                                })
                                .unwrap(),
                            ))
                        }
                        _ => retrieved_elements,
                    },
                    Err(_) => {
                        return Err(ServerMessage::error_response(
                            "lockelements".to_string(),
                            serde_json::to_string(&ErrorResponseBody {
                                message: "Found Elements could not be retrieved".to_string(),
                                body: serde_json::to_string(&body.ids).unwrap(),
                            })
                            .unwrap(),
                        ))
                    }
                }
            }
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "lockelements".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Error during Element check".to_string(),
                        body: serde_json::to_string(&body.ids).unwrap(),
                    })
                    .unwrap(),
                ))
            }
        };
        if found_elements
            .iter()
            .any(|element| match &element.locked_by {
                Some(locked_by) => *locked_by != body.user_id,
                None => false,
            })
        {
            return Err(ServerMessage::error_response(
                "lockelements".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "Some Element is locked by another user".to_string(),
                    body: serde_json::to_string(&body.ids).unwrap(),
                })
                .unwrap(),
            ));
        }
        let mut updated_document_results: Vec<UpdateResult> = vec![];
        for element in found_elements.iter() {
            let query_doc = doc! {
                "_id": element._id.clone(),
            };
            match Element::update_document(
                &database_client,
                query_doc,
                UpdateElement {
                    selected: None,
                    locked_by: Some(Some(body.user_id.clone())),
                    x: None,
                    y: None,
                    rotation: None,
                    scale_x: None,
                    scale_y: None,
                    z_index: None,
                    text: None,
                    color: None,
                },
            )
            .await
            {
                Ok(update_result) => match update_result.modified_count {
                    0 => {
                        return Err(ServerMessage::error_response(
                            "lockelements".to_string(),
                            serde_json::to_string(&ErrorResponseBody {
                                message: format!("Lock of Element with ID {} failed", element._id),
                                body: serde_json::to_string(&body.ids).unwrap(),
                            })
                            .unwrap(),
                        ))
                    }
                    _ => {
                        updated_document_results.push(update_result);
                    }
                },
                Err(_) => {
                    return Err(ServerMessage::error_response(
                        "lockelements".to_string(),
                        serde_json::to_string(&ErrorResponseBody {
                            message: "Error during locking of elements".to_string(),
                            body: serde_json::to_string(&body.ids).unwrap(),
                        })
                        .unwrap(),
                    ))
                }
            }
        }
        match updated_document_results.len() {
            0 => Err(ServerMessage::error_response(
                "lockelements".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "No Element found to lock".to_string(),
                    body: serde_json::to_string(&body.ids).unwrap(),
                })
                .unwrap(),
            )),
            _ => {
                for element_id in body.ids.iter() {
                    let mut sub_context = context.lock().await;
                    sub_context
                        .emit_element_event(
                            body.board_id.to_string(),
                            ElementEvent {
                                event_type: ElementEventType::Locked,
                                body: serde_json::to_string(&ElementLockedEventPayload {
                                    _id: element_id.clone(),
                                    user_id: body.user_id.clone(),
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(sub_context);
                }
                Ok(ServerMessage::ok_response(
                    "lockelements".to_string(),
                    serde_json::to_string(&ElementsLockedMessage {
                        ids: body.ids,
                        user_id: body.user_id,
                    })
                    .unwrap(),
                ))
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockElementsMessage {
    pub ids: Vec<String>,
    pub user_id: String,
    pub board_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementsUnlockedMessage {
    ids: Vec<String>,
}

impl WebTransportBaseMessageHandler<ElementContext> for UnlockElementsMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ElementContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<UnlockElementsMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "unlockelements".to_string(),
                    "Unlock Elements Message is invalid".to_string(),
                ))
            }
        };
        let query_doc = doc! {
            "_id": doc! { "$in": body.ids.iter().map(|id| ObjectId::from_str(id.as_str()).unwrap()).collect::<Vec<ObjectId>>() }
        };
        let found_element_result =
            Element::get_multiple_documents(&database_client, query_doc.clone()).await;
        let found_elements = match found_element_result {
            Ok(element_cursor) => {
                let retrieved_elements = element_cursor.try_collect::<Vec<Element>>().await;
                match retrieved_elements {
                    Ok(retrieved_elements) => match retrieved_elements.len() {
                        0 => {
                            return Err(ServerMessage::error_response(
                                "unlockelements".to_string(),
                                serde_json::to_string(&ErrorResponseBody {
                                    message: "No Elements found".to_string(),
                                    body: serde_json::to_string(&body.ids).unwrap(),
                                })
                                .unwrap(),
                            ))
                        }
                        _ => retrieved_elements,
                    },
                    Err(_) => {
                        return Err(ServerMessage::error_response(
                            "unlockelements".to_string(),
                            serde_json::to_string(&ErrorResponseBody {
                                message: "Found Elements could not be retrieved".to_string(),
                                body: serde_json::to_string(&body.ids).unwrap(),
                            })
                            .unwrap(),
                        ))
                    }
                }
            }
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "unlockelements".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Error during Elements check".to_string(),
                        body: serde_json::to_string(&body.ids).unwrap(),
                    })
                    .unwrap(),
                ))
            }
        };
        if found_elements
            .iter()
            .any(|element| match &element.locked_by {
                Some(locked_by) => *locked_by != body.user_id,
                None => false,
            })
        {
            return Err(ServerMessage::error_response(
                "unlockelements".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "Some element is locked by another user".to_string(),
                    body: serde_json::to_string(&body.ids).unwrap(),
                })
                .unwrap(),
            ));
        }
        let mut updated_document_results: Vec<UpdateResult> = vec![];
        for element in found_elements.iter() {
            let query_doc = doc! {
                "_id": ObjectId::from_str(element._id.as_str()).unwrap(),
            };
            match Element::update_document(
                &database_client,
                query_doc,
                UpdateElement {
                    selected: None,
                    locked_by: Some(None),
                    x: None,
                    y: None,
                    rotation: None,
                    scale_x: None,
                    scale_y: None,
                    z_index: None,
                    text: None,
                    color: None,
                },
            )
            .await
            {
                Ok(update_result) => match update_result.modified_count {
                    0 => {
                        return Err(ServerMessage::error_response(
                            "unlockelements".to_string(),
                            serde_json::to_string(&ErrorResponseBody {
                                message: format!(
                                    "Unlock of Element with ID {} failed",
                                    element._id
                                ),
                                body: serde_json::to_string(&body.ids).unwrap(),
                            })
                            .unwrap(),
                        ));
                    }
                    _ => {
                        updated_document_results.push(update_result);
                    }
                },
                Err(_) => {
                    return Err(ServerMessage::error_response(
                        "unlockelements".to_string(),
                        serde_json::to_string(&ErrorResponseBody {
                            message: "Error during unlocking of elements".to_string(),
                            body: serde_json::to_string(&body.ids).unwrap(),
                        })
                        .unwrap(),
                    ))
                }
            }
        }
        match updated_document_results.len() {
            0 => Err(ServerMessage::error_response(
                "unlockelements".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "No Element found to unlock".to_string(),
                    body: serde_json::to_string(&body.ids).unwrap(),
                })
                .unwrap(),
            )),
            _ => {
                for element_id in body.ids.iter() {
                    let mut sub_context = context.lock().await;
                    sub_context
                        .emit_element_event(
                            body.board_id.to_string(),
                            ElementEvent {
                                event_type: ElementEventType::Unlocked,
                                body: serde_json::to_string(&ElementUnlockedEventPayload {
                                    _id: element_id.clone(),
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(sub_context);
                }
                Ok(ServerMessage::ok_response(
                    "unlockelements".to_string(),
                    serde_json::to_string(&ElementsUnlockedMessage { ids: body.ids }).unwrap(),
                ))
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedElementEventPayload {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
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
pub struct UpdateElementMessage {
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementUpdatedMessage {
    pub id: String,
}

impl WebTransportBaseMessageHandler<ElementContext> for UpdateElementMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ElementContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<UpdateElementMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "updateelement".to_string(),
                    "Update Element Message is invalid".to_string(),
                ))
            }
        };
        let query_doc = doc! {
            "_id": ObjectId::from_str(body._id.as_str()).unwrap(),
        };
        let found_element_result = Element::get_document(&database_client, query_doc.clone()).await;
        match found_element_result {
            Ok(element) => match element {
                Some(element) => match element.locked_by {
                    Some(locked_by) => {
                        if locked_by != body.user_id {
                            return Err(ServerMessage::error_response(
                                "updateelement".to_string(),
                                serde_json::to_string(&ErrorResponseBody {
                                    message: "Element currently locked by someone else".to_string(),
                                    body: serde_json::to_string(&ElementUpdatedMessage {
                                        id: body._id,
                                    })
                                    .unwrap(),
                                })
                                .unwrap(),
                            ));
                        }
                    }
                    None => {
                        return Err(ServerMessage::error_response(
                            "updateelement".to_string(),
                            serde_json::to_string(&ErrorResponseBody {
                                message: "Element needs to be locked first".to_string(),
                                body: serde_json::to_string(&ElementUpdatedMessage {
                                    id: body._id,
                                })
                                .unwrap(),
                            })
                            .unwrap(),
                        ));
                    }
                },
                None => {
                    return Err(ServerMessage::error_response(
                        "updateelement".to_string(),
                        serde_json::to_string(&ErrorResponseBody {
                            message: format!("No Element found with ID: {}", body._id),
                            body: serde_json::to_string(&ElementUpdatedMessage { id: body._id })
                                .unwrap(),
                        })
                        .unwrap(),
                    ))
                }
            },
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "updateelement".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Error during Element fetching".to_string(),
                        body: serde_json::to_string(&ElementUpdatedMessage { id: body._id })
                            .unwrap(),
                    })
                    .unwrap(),
                ));
            }
        };
        let update_result = Element::update_document(
            &database_client,
            query_doc,
            UpdateElement {
                selected: None,
                locked_by: None,
                x: body.x,
                y: body.y,
                rotation: body.rotation,
                scale_x: body.scale_x,
                scale_y: body.scale_y,
                z_index: body.z_index,
                text: body.text.clone(),
                color: body.color.clone(),
            },
        )
        .await;
        match update_result {
            Ok(result) => match result.modified_count {
                0 => Err(ServerMessage::error_response(
                    "updateelement".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "No Element found to update".to_string(),
                        body: serde_json::to_string(&ElementUpdatedMessage { id: body._id })
                            .unwrap(),
                    })
                    .unwrap(),
                )),
                _ => {
                    let mut sub_context = context.lock().await;
                    sub_context
                        .emit_element_event(
                            body.board_id.clone(),
                            ElementEvent {
                                event_type: ElementEventType::Updated,
                                body: serde_json::to_string(&UpdatedElementEventPayload {
                                    user_id: body.user_id.clone(),
                                    _id: body._id.clone(),
                                    text: body.text.clone(),
                                    z_index: body.z_index,
                                    scale_x: body.scale_x,
                                    scale_y: body.scale_y,
                                    rotation: body.rotation,
                                    x: body.x,
                                    y: body.y,
                                    color: body.color,
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(sub_context);
                    Ok(ServerMessage::ok_response(
                        "updateelement".to_string(),
                        serde_json::to_string(&ElementUpdatedMessage { id: body._id }).unwrap(),
                    ))
                }
            },
            Err(_) => Err(ServerMessage::error_response(
                "updateelement".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "Could not update Element".to_string(),
                    body: serde_json::to_string(&ElementUpdatedMessage { id: body._id }).unwrap(),
                })
                .unwrap(),
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementMovedEventPayload {
    #[serde(rename = "_id")]
    pub _id: String,
    pub user_id: String,
    pub x_offset: f32,
    pub y_offset: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveElementsMessage {
    pub ids: Vec<String>,
    pub user_id: String,
    pub board_id: String,
    pub x_offset: f32,
    pub y_offset: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementsMovedMessage {
    pub ids: Vec<String>,
}

impl WebTransportBaseMessageHandler<ElementContext> for MoveElementsMessage {
    async fn handle_message(
        message: Value,
        database_client: Client,
        context: Arc<Mutex<ElementContext>>,
    ) -> Result<ServerMessage, ServerMessage> {
        let body = match serde_json::from_value::<MoveElementsMessage>(message) {
            Ok(parsed_message) => parsed_message,
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "moveelements".to_string(),
                    "Move Elements Message is invalid".to_string(),
                ))
            }
        };
        let query_doc = doc! {
            "_id": doc! { "$in": body.ids.iter().map(|id| ObjectId::from_str(id.as_str()).unwrap()).collect::<Vec<ObjectId>>() }
        };
        let found_element_result =
            Element::get_multiple_documents(&database_client, query_doc.clone()).await;
        let found_elements = match found_element_result {
            Ok(element_cursor) => {
                let retrieved_elements = element_cursor.try_collect::<Vec<Element>>().await;
                match retrieved_elements {
                    Ok(retrieved_elements) => match retrieved_elements.len() {
                        0 => {
                            return Err(ServerMessage::error_response(
                                "moveelements".to_string(),
                                serde_json::to_string(&ErrorResponseBody {
                                    message: "No Elements found".to_string(),
                                    body: serde_json::to_string(&body.ids).unwrap(),
                                })
                                .unwrap(),
                            ));
                        }
                        _ => retrieved_elements,
                    },
                    Err(_) => {
                        return Err(ServerMessage::error_response(
                            "moveelements".to_string(),
                            serde_json::to_string(&ErrorResponseBody {
                                message: "Found Elements could not be retrieved".to_string(),
                                body: serde_json::to_string(&body.ids).unwrap(),
                            })
                            .unwrap(),
                        ));
                    }
                }
            }
            Err(_) => {
                return Err(ServerMessage::error_response(
                    "moveelements".to_string(),
                    serde_json::to_string(&ErrorResponseBody {
                        message: "Error during fetching of Elements".to_string(),
                        body: serde_json::to_string(&body.ids).unwrap(),
                    })
                    .unwrap(),
                ));
            }
        };
        if found_elements
            .iter()
            .any(|element| match &element.locked_by {
                Some(locked_by) => *locked_by != body.user_id,
                None => false,
            })
        {
            return Err(ServerMessage::error_response(
                "moveelements".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "Some Element is locked by someone else".to_string(),
                    body: serde_json::to_string(&body.ids).unwrap(),
                })
                .unwrap(),
            ));
        }
        let mut updated_document_results: Vec<UpdateResult> = vec![];
        for element in found_elements.iter() {
            let query_doc = doc! {
                "_id": ObjectId::from_str(element._id.as_str()).unwrap(),
            };
            match Element::update_document(
                &database_client,
                query_doc,
                UpdateElement {
                    selected: None,
                    locked_by: Some(Some(body.user_id.clone())),
                    x: Some(element.x + body.x_offset),
                    y: Some(element.y + body.y_offset),
                    rotation: None,
                    scale_x: None,
                    scale_y: None,
                    z_index: None,
                    text: None,
                    color: None,
                },
            )
            .await
            {
                Ok(update_result) => match update_result.modified_count {
                    0 => {
                        return Err(ServerMessage::error_response(
                            "moveelements".to_string(),
                            serde_json::to_string(&ErrorResponseBody {
                                message: format!("Move of Element with ID {} failed", element._id),
                                body: serde_json::to_string(&body.ids).unwrap(),
                            })
                            .unwrap(),
                        ));
                    }
                    _ => {
                        updated_document_results.push(update_result);
                    }
                },
                Err(_) => {
                    return Err(ServerMessage::error_response(
                        "moveelements".to_string(),
                        serde_json::to_string(&ErrorResponseBody {
                            message: "Error during moving of Elements".to_string(),
                            body: serde_json::to_string(&body.ids).unwrap(),
                        })
                        .unwrap(),
                    ));
                }
            }
        }
        match updated_document_results.len() {
            0 => Err(ServerMessage::error_response(
                "moveelements".to_string(),
                serde_json::to_string(&ErrorResponseBody {
                    message: "No Element found to update".to_string(),
                    body: serde_json::to_string(&body.ids).unwrap(),
                })
                .unwrap(),
            )),
            _ => {
                for element_id in body.ids.iter() {
                    let mut sub_context = context.lock().await;
                    sub_context
                        .emit_element_event(
                            body.board_id.to_string(),
                            ElementEvent {
                                event_type: ElementEventType::Moved,
                                body: serde_json::to_string(&ElementMovedEventPayload {
                                    _id: element_id.to_string(),
                                    user_id: body.user_id.clone(),
                                    x_offset: body.x_offset,
                                    y_offset: body.y_offset,
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(sub_context);
                }
                Ok(ServerMessage::ok_response(
                    "moveelements".to_string(),
                    serde_json::to_string(&ElementsMovedMessage { ids: body.ids }).unwrap(),
                ))
            }
        }
    }
}
