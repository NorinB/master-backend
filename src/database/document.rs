use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use mongodb::{
    options::CreateCollectionOptions,
    results::{DeleteResult, InsertOneResult, UpdateResult},
    Client, Cursor,
};
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

use super::config::DATABASE_NAME;

pub struct DocumentBase {}

impl DocumentBase {
    pub async fn create_collection(
        client: &Client,
        collection_name: &str,
        create_collection_opts: Option<CreateCollectionOptions>,
        document_name: &str,
    ) -> Result<(), Response> {
        let result = client
            .database(DATABASE_NAME())
            .create_collection(collection_name, create_collection_opts)
            .await;
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error during {} collection creaton", document_name),
            )
                .into_response()),
        }
    }

    pub async fn create_document<CreateDocument>(
        client: &Client,
        collection_name: &str,
        insert_doc: CreateDocument,
        document_name: &str,
    ) -> Result<InsertOneResult, Response>
    where
        CreateDocument: Serialize,
    {
        let result = client
            .database(DATABASE_NAME())
            .collection::<CreateDocument>(collection_name)
            .insert_one(insert_doc, None)
            .await;
        match result {
            Ok(result) => Ok(result),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error during {} creation", document_name),
            )
                .into_response()),
        }
    }

    pub async fn delete_document<BaseDocument>(
        client: &Client,
        collection_name: &str,
        query_doc: bson::Document,
        document_name: &str,
    ) -> Result<DeleteResult, Response>
    where
        BaseDocument: Serialize,
    {
        let result = client
            .database(DATABASE_NAME())
            .collection::<BaseDocument>(collection_name)
            .delete_one(query_doc, None)
            .await;
        match result {
            Ok(result) => Ok(result),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error during {} deletion", document_name),
            )
                .into_response()),
        }
    }

    pub async fn update_document<BaseDocument>(
        client: &Client,
        collection_name: &str,
        query_doc: bson::Document,
        update_doc: bson::Document,
        document_name: &str,
    ) -> Result<UpdateResult, Response>
    where
        BaseDocument: Serialize,
    {
        let result = client
            .database(DATABASE_NAME())
            .collection::<BaseDocument>(collection_name)
            .update_one(query_doc, update_doc, None)
            .await;
        match result {
            Ok(result) => Ok(result),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error during {} update", document_name),
            )
                .into_response()),
        }
    }

    pub async fn delete_collection<BaseDocument>(
        client: &Client,
        collection_name: &str,
        document_name: &str,
    ) -> Result<(), Response>
    where
        BaseDocument: Serialize,
    {
        let result = client
            .database(DATABASE_NAME())
            .collection::<BaseDocument>(collection_name)
            .drop(None)
            .await;
        match result {
            Ok(result) => Ok(result),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error during {} collection deletion", document_name),
            )
                .into_response()),
        }
    }

    pub async fn get_document<BaseDocument>(
        client: &Client,
        collection_name: &str,
        query_doc: bson::Document,
        document_name: &str,
    ) -> Result<Option<BaseDocument>, Response>
    where
        BaseDocument: DeserializeOwned + Unpin + Sync + Send,
    {
        let result = client
            .database(DATABASE_NAME())
            .collection::<BaseDocument>(collection_name)
            .find_one(query_doc, None)
            .await;
        match result {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("{:?}", err);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error during {} fetching", document_name),
                )
                    .into_response())
            }
        }
    }

    pub async fn get_multiple_documents<BaseDocument>(
        client: &Client,
        collection_name: &str,
        query_doc: bson::Document,
        document_name: &str,
    ) -> Result<Cursor<BaseDocument>, Response>
    where
        BaseDocument: DeserializeOwned,
    {
        let result = client
            .database(DATABASE_NAME())
            .collection::<BaseDocument>(collection_name)
            .find(query_doc, None)
            .await;
        match result {
            Ok(result) => Ok(result),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error during {} fetching", document_name),
            )
                .into_response()),
        }
    }
}

#[allow(dead_code)]
pub trait Document<Base, Create, Update> {
    async fn create_collection(client: &Client) -> Result<(), Response>;
    async fn get_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<Option<Base>, Response>;
    async fn create_document(
        client: &Client,
        insert_doc: Create,
    ) -> Result<InsertOneResult, Response>;
    async fn delete_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<DeleteResult, Response>;
    async fn update_document(
        client: &Client,
        query_doc: bson::Document,
        update_document: Update,
    ) -> Result<UpdateResult, Response>;
    async fn delete_collection(client: &Client) -> Result<(), Response>;
    async fn get_multiple_documents(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<Cursor<Base>, Response>;
}
