use std::fmt::Display;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bson::serde_helpers::deserialize_hex_string_from_object_id;
use mongodb::{
    bson::doc,
    options::{CreateCollectionOptions, ValidationAction, ValidationLevel},
    results::{DeleteResult, InsertOneResult, UpdateResult},
    Cursor,
};
use serde::{Deserialize, Serialize};

use crate::database::{
    document::{Document, DocumentBase},
    validator::Validator,
};

const CLIENT_COLLECTION_NAME: &str = "client";
const CLIENT_DOCUMENT_NAME: &str = "Client";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum DeviceType {
    Web,
    Android,
    Ios,
    Other,
}

impl Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enum_as_string = match self {
            DeviceType::Web => "Web".to_string(),
            DeviceType::Android => "Android".to_string(),
            DeviceType::Ios => "IOS".to_string(),
            DeviceType::Other => "Other".to_string(),
        };
        write!(f, "{}", enum_as_string)
    }
}

impl DeviceType {
    pub fn to_enum(enum_string: String) -> Self {
        match enum_string.as_str() {
            "Web" => DeviceType::Web,
            "Android" => DeviceType::Android,
            "IOS" => DeviceType::Ios,
            _ => DeviceType::Other,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    #[serde(
        deserialize_with = "deserialize_hex_string_from_object_id",
        rename = "_id"
    )]
    pub _id: String,
    pub client_id: String,
    pub user_id: String,
    pub device_type: DeviceType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateClient {
    pub client_id: String,
    pub user_id: String,
    pub device_type: DeviceType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClient {
    pub client_id: Option<String>,
    pub device_type: Option<DeviceType>,
}

impl Document<Client, CreateClient, UpdateClient> for Client {
    async fn create_collection(client: &mongodb::Client) -> Result<(), Response> {
        let create_collection_opts = Client::get_validation_options().ok();
        DocumentBase::create_collection(
            client,
            CLIENT_COLLECTION_NAME,
            create_collection_opts,
            CLIENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn create_document(
        client: &mongodb::Client,
        insert_doc: CreateClient,
    ) -> Result<InsertOneResult, Response> {
        DocumentBase::create_document::<CreateClient>(
            client,
            CLIENT_COLLECTION_NAME,
            insert_doc,
            CLIENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_document(
        client: &mongodb::Client,
        query_doc: bson::Document,
    ) -> Result<DeleteResult, Response> {
        DocumentBase::delete_document::<Client>(
            client,
            CLIENT_COLLECTION_NAME,
            query_doc,
            CLIENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn update_document(
        client: &mongodb::Client,
        query_doc: bson::Document,
        update_document: UpdateClient,
    ) -> Result<UpdateResult, Response> {
        let mut update_fields = doc! {};
        if let Some(device_type) = update_document.device_type {
            update_fields.insert("deviceType", device_type.to_string());
        }
        if let Some(client_id) = update_document.client_id {
            update_fields.insert("clientId", client_id);
        }
        let update_doc = doc! {
            "$set": update_fields
        };
        DocumentBase::update_document::<Client>(
            client,
            CLIENT_COLLECTION_NAME,
            query_doc,
            update_doc,
            CLIENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_collection(client: &mongodb::Client) -> Result<(), Response> {
        DocumentBase::delete_collection::<Client>(
            client,
            CLIENT_COLLECTION_NAME,
            CLIENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_document(
        client: &mongodb::Client,
        query_doc: bson::Document,
    ) -> Result<Option<Client>, Response> {
        DocumentBase::get_document::<Client>(
            client,
            CLIENT_COLLECTION_NAME,
            query_doc,
            CLIENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_multiple_documents(
        client: &mongodb::Client,
        query_doc: bson::Document,
    ) -> Result<Cursor<Client>, Response> {
        DocumentBase::get_multiple_documents::<Client>(
            client,
            CLIENT_COLLECTION_NAME,
            query_doc,
            CLIENT_DOCUMENT_NAME,
        )
        .await
    }
}

impl Client {
    pub async fn get_existing_client(
        user_id: String,
        database_client: &mongodb::Client,
    ) -> Result<Client, Response> {
        let query_doc = doc! {
            "userId": user_id,
        };
        let element_result = Client::get_document(database_client, query_doc).await;
        match element_result {
            Ok(client_option) => match client_option {
                Some(client) => Ok(client),
                None => Err((StatusCode::NOT_FOUND, "Client does not exist").into_response()),
            },
            Err(error_response) => Err(error_response),
        }
    }
}

impl Validator for Client {
    fn get_validation_options(
    ) -> Result<mongodb::options::CreateCollectionOptions, Box<dyn std::error::Error>> {
        let validator = doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "Client Validation",
                "required": vec! ["_id", "user_id", "device_type"],
                "properties": doc! {
                    "_id": doc! {
                        "bsonType": "string",
                        "description": "ID of the client"
                    },
                    "client_id": doc! {
                        "bsonType": "string",
                        "description": "ID of the client device, created by the client"
                    },
                    "user_id": doc! {
                        "bsonType": "string",
                        "description": "ID of the user this client is associated with"
                    },
                    "device_type": doc! {
                        "enum": vec!["Web", "Android", "IOS"],
                        "description": "Type of the device associated with this client"
                    }
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
