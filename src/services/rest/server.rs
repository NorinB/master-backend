use std::net::{Ipv4Addr, SocketAddr};

use crate::{
    services::rest::endpoints::{active_member, board, client, element, element_type, ping, user},
    AppState,
};
use anyhow::Context;
use axum::{serve::Serve, Router};
use tower_http::cors::CorsLayer;
use tracing::info;

pub struct RestServer {
    serve: Serve<Router, Router>,
    pub local_port: u16,
}

impl RestServer {
    const PORT: u16 = 3030;
    pub async fn new(state: AppState) -> anyhow::Result<Self> {
        let router = Self::build_router(state);

        let listener = tokio::net::TcpListener::bind(SocketAddr::new(
            Ipv4Addr::UNSPECIFIED.into(),
            Self::PORT,
        ))
        .await
        .expect("Failed to bind address!");

        let local_port = listener
            .local_addr()
            .context("Cannot get local port")?
            .port();

        Ok(RestServer {
            serve: axum::serve(listener, router),
            local_port,
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        info!("Server running on port {}", self.local_port());

        let _ = self.serve.await.context("HTTP Server error");

        Ok(())
    }

    fn build_router(state: AppState) -> Router {
        Router::<AppState>::new()
            .merge(ping::get_routes())
            .merge(user::get_routes())
            .merge(board::get_routes())
            .merge(active_member::get_routes())
            .merge(element::get_routes())
            .merge(element_type::get_routes())
            .merge(client::get_routes())
            .with_state(state)
            .layer(CorsLayer::permissive())
    }
}
