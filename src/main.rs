use axum::{routing::get, Router};
use dotenvy::dotenv;
use mongodb::bson::doc;
use mongodb::{options::ClientOptions, Client};

mod database;
mod services;
use crate::database::config::DatabaseConfig;
use crate::services::rest::root;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().expect("Failed to load .env file");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .init();

    let database_config = DatabaseConfig::new();
    let mut client_options = ClientOptions::parse(database_config.uri).await.unwrap();
    client_options.connect_timeout = database_config.connection_timeout;
    client_options.max_pool_size = database_config.max_pool_size;
    client_options.min_pool_size = database_config.min_pool_size;
    let client = Client::with_options(client_options).unwrap();

    client
        .database("admin")
        .run_command(doc! { "ping": 1 }, None)
        .await?;
    println!("Connection to MongoDB successful!");

    let app = Router::new().route("/", get(root)).with_state(client);
    let address = "0.0.0.0:3030";
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("Failed to bind address!");
    println!("Server is running on: {}", address);
    axum::serve(listener, app)
        .await
        .expect("Failed to start server!");

    Ok(())
}
