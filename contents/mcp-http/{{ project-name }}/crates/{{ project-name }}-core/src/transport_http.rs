//! Streamable HTTP transport for the MCP server.

use std::sync::Arc;

use anyhow::Result;
use axum::{Json, response::IntoResponse};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use serde_json::json;
use tower_http::cors::CorsLayer;

use crate::server::{{ ProjectName }}Server;

pub async fn serve_http(server: {{ ProjectName }}Server, port: u16) -> Result<()> {
    let addr = format!("0.0.0.0:{port}");
    tracing::info!(addr = %addr, "starting MCP streamable HTTP transport");

    let health_name = server.config.name.clone();
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    );

    let app = axum::Router::new()
        .route("/health", axum::routing::get(move || health(health_name)))
        .nest_service("/mcp", service)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        )
        .expect("failed to register SIGTERM handler");

        tokio::select! {
            _ = ctrl_c => { tracing::info!("received Ctrl+C, shutting down"); }
            _ = sigterm.recv() => { tracing::info!("received SIGTERM, shutting down"); }
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.expect("failed to listen for Ctrl+C");
        tracing::info!("received Ctrl+C, shutting down");
    }
}

async fn health(name: String) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "name": name,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
