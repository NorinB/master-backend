use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bson::{doc, serde_helpers::deserialize_hex_string_from_object_id};
use mongodb::{
    options::{CreateCollectionOptions, ValidationAction, ValidationLevel},
    results::{DeleteResult, InsertOneResult, UpdateResult},
    Client,
};
use serde::{Deserialize, Serialize};

use crate::database::{
    document::{Document, DocumentBase},
    validator::Validator,
};

const ACTIVE_MEMBER_COLLECTION_NAME: &str = "active_member";
const ACTIVE_MEMBER_DOCUMENT_NAME: &str = "Active Member";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActiveMember {
    #[serde(
        deserialize_with = "deserialize_hex_string_from_object_id",
        rename = "_id"
    )]
    pub _id: String,
    pub user_id: String,
    pub board_id: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateActiveMember {
    pub user_id: String,
    pub board_id: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateActiveMember {
    pub board_id: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

impl Document<ActiveMember, CreateActiveMember, UpdateActiveMember> for ActiveMember {
    async fn create_collection(client: &Client) -> Result<(), Response> {
        let create_collection_opts = ActiveMember::get_validation_options().ok();
        DocumentBase::create_collection(
            client,
            ACTIVE_MEMBER_COLLECTION_NAME,
            create_collection_opts,
            ACTIVE_MEMBER_DOCUMENT_NAME,
        )
        .await
    }

    async fn create_document(
        client: &Client,
        insert_doc: CreateActiveMember,
    ) -> Result<InsertOneResult, Response> {
        DocumentBase::create_document::<CreateActiveMember>(
            client,
            ACTIVE_MEMBER_COLLECTION_NAME,
            insert_doc,
            ACTIVE_MEMBER_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<DeleteResult, Response> {
        DocumentBase::delete_document::<ActiveMember>(
            client,
            ACTIVE_MEMBER_COLLECTION_NAME,
            query_doc,
            ACTIVE_MEMBER_DOCUMENT_NAME,
        )
        .await
    }

    async fn update_document(
        client: &Client,
        query_doc: bson::Document,
        update_document: UpdateActiveMember,
    ) -> Result<UpdateResult, Response> {
        let mut update_fields = doc! {};
        if let Some(board_id) = update_document.board_id {
            update_fields.insert("boardId", board_id);
        }
        if let Some(x) = update_document.x {
            update_fields.insert("x", x);
        }
        if let Some(y) = update_document.y {
            update_fields.insert("y", y);
        }
        let update_doc = doc! {
            "$set": update_fields
        };
        DocumentBase::update_document::<ActiveMember>(
            client,
            ACTIVE_MEMBER_COLLECTION_NAME,
            query_doc,
            update_doc,
            ACTIVE_MEMBER_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_collection(client: &Client) -> Result<(), Response> {
        DocumentBase::delete_collection::<ActiveMember>(
            client,
            ACTIVE_MEMBER_COLLECTION_NAME,
            ACTIVE_MEMBER_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<Option<ActiveMember>, Response> {
        DocumentBase::get_document::<ActiveMember>(
            client,
            ACTIVE_MEMBER_COLLECTION_NAME,
            query_doc,
            ACTIVE_MEMBER_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_multiple_documents(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<mongodb::Cursor<ActiveMember>, Response> {
        DocumentBase::get_multiple_documents::<ActiveMember>(
            client,
            ACTIVE_MEMBER_COLLECTION_NAME,
            query_doc,
            ACTIVE_MEMBER_DOCUMENT_NAME,
        )
        .await
    }
}

impl ActiveMember {
    pub async fn get_existing_active_member_by_user_id(
        user_id: String,
        database_client: &mongodb::Client,
    ) -> Result<ActiveMember, Response> {
        let query_doc = doc! {
            "userId": user_id,
        };
        let element_result = ActiveMember::get_document(database_client, query_doc).await;
        match element_result {
            Ok(active_member_option) => match active_member_option {
                Some(active_member) => Ok(active_member),
                None => {
                    Err((StatusCode::NOT_FOUND, "Active Member does not exist").into_response())
                }
            },
            Err(error_response) => Err(error_response),
        }
    }
}

impl Validator for ActiveMember {
    fn get_validation_options(
    ) -> Result<mongodb::options::CreateCollectionOptions, Box<dyn std::error::Error>> {
        let validator = doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "Active Member Validation",
                "required": vec!["_id", "user_id", "board_id", "x", "y"],
                "properties": doc! {
                    "_id": doc! {
                        "bsonType": "string",
                        "description": "ID of this active member"
                    },
                    "userId": doc! {
                        "bsonType": "string",
                        "description": "ID of the user, this active member is reflecting"
                    },
                    "boardId": doc! {
                        "bsonType": "string",
                        "description": "ID of the board, this member is working on"
                    },
                    "x": doc! {
                        "bsonType": "double",
                        "description": "X Coordinate of the active member to display the cursor"
                    },
                    "y": doc! {
                        "bsonType": "double",
                        "description": "Y Coordinate of the active member to display the cursor"
                    },
                }
            }
        };

        let validation_opts = CreateCollectionOptions::builder()
            .validator(validator)
            .validation_action(Some(ValidationAction::Error))
            .validation_level(Some(ValidationLevel::Moderate))
            .build();

        Ok(validation_opts)
    }
}
