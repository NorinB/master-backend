use axum::response::Response;
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

const ELEMENT_TYPE_COLLECTION_NAME: &str = "element-type";
const ELEMENT_TYPE_DOCUMENT_NAME: &str = "Element Type";

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ElementType {
    #[serde(
        deserialize_with = "deserialize_hex_string_from_object_id",
        rename = "_id"
    )]
    pub _id: String,
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateElementType {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateElementType {
    pub name: Option<String>,
    pub path: Option<String>,
}

impl Document<ElementType, CreateElementType, UpdateElementType> for ElementType {
    async fn create_collection(client: &Client) -> Result<(), Response> {
        let create_collection_opts = ElementType::get_validation_options().ok();
        DocumentBase::create_collection(
            client,
            ELEMENT_TYPE_COLLECTION_NAME,
            create_collection_opts,
            ELEMENT_TYPE_DOCUMENT_NAME,
        )
        .await
    }

    async fn create_document(
        client: &Client,
        insert_doc: CreateElementType,
    ) -> Result<InsertOneResult, Response> {
        DocumentBase::create_document::<CreateElementType>(
            client,
            ELEMENT_TYPE_COLLECTION_NAME,
            insert_doc,
            ELEMENT_TYPE_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<DeleteResult, Response> {
        DocumentBase::delete_document::<ElementType>(
            client,
            ELEMENT_TYPE_COLLECTION_NAME,
            query_doc,
            ELEMENT_TYPE_DOCUMENT_NAME,
        )
        .await
    }

    async fn update_document(
        client: &Client,
        query_doc: bson::Document,
        update_document: UpdateElementType,
    ) -> Result<UpdateResult, Response> {
        let mut update_fields = doc! {};
        if let Some(name) = update_document.name {
            update_fields.insert("name", name);
        }
        if let Some(path) = update_document.path {
            update_fields.insert("path", path);
        }
        let update_doc = doc! {
            "$set": update_fields
        };
        DocumentBase::update_document::<ElementType>(
            client,
            ELEMENT_TYPE_COLLECTION_NAME,
            query_doc,
            update_doc,
            ELEMENT_TYPE_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_collection(client: &Client) -> Result<(), Response> {
        DocumentBase::delete_collection::<ElementType>(
            client,
            ELEMENT_TYPE_COLLECTION_NAME,
            ELEMENT_TYPE_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<Option<ElementType>, Response> {
        DocumentBase::get_document::<ElementType>(
            client,
            ELEMENT_TYPE_COLLECTION_NAME,
            query_doc,
            ELEMENT_TYPE_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_multiple_documents(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<mongodb::Cursor<ElementType>, Response> {
        DocumentBase::get_multiple_documents::<ElementType>(
            client,
            ELEMENT_TYPE_COLLECTION_NAME,
            query_doc,
            ELEMENT_TYPE_DOCUMENT_NAME,
        )
        .await
    }
}

impl Validator for ElementType {
    fn get_validation_options(
    ) -> Result<mongodb::options::CreateCollectionOptions, Box<dyn std::error::Error>> {
        let validator = doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "ElementType Validation",
                "required": vec!["_id", "name", "path"],
                "properties": doc! {
                    "_id": doc! {
                        "bsonType": "string",
                        "description": "ID of the element type",
                    },
                    "name": doc! {
                        "bsonType": "string",
                        "description": "Name of the user"
                    },
                    "path": doc! {
                        "bsonType": "string",
                        "description": "Path of the Element",
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
