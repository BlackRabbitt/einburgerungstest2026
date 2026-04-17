mod ai;
mod memory;
mod models;
mod quiz;
mod routes;
mod storage;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::ai::AiClient;
use crate::quiz::validate_questions;
use crate::routes::{AppState, app_router, load_questions};
use crate::storage::{ensure_dir, root_data_dir};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let questions = load_questions().await?;
    validate_questions(&questions)?;

    let data_root = root_data_dir();
    let exams_dir = data_root.join("exams");
    let memory_dir = data_root.join("memory");
    ensure_dir(&exams_dir).await?;
    ensure_dir(&memory_dir).await?;

    let state = AppState {
        questions: Arc::new(questions),
        exams_dir: Arc::new(exams_dir),
        memory_dir: Arc::new(memory_dir),
        ai_client: AiClient::new(
            std::env::var("OPENAI_API_KEY").ok(),
            std::env::var("OPENAI_MODEL").ok(),
        ),
    };

    let app = Router::new()
        .merge(app_router(state))
        .fallback_service(ServeDir::new(PathBuf::from("static")).append_index_html_on_directories(true))
        .layer(TraceLayer::new_for_http());

    let host = std::env::var("HOST")
        .ok()
        .and_then(|value| value.parse::<IpAddr>().ok())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);
    let address = SocketAddr::new(host, port);

    tracing::info!("listening on http://{}", address);
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
