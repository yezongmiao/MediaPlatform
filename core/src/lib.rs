pub mod api;
pub mod assets;
pub mod assistant;
pub mod cli;
pub mod context;
pub mod error;
pub mod frontmatter;
pub mod library;
pub mod llm;
pub mod media_gen;
pub mod models;
pub mod seed;
pub mod store;
pub mod ui;

use std::sync::Arc;

pub async fn serve(
    listener: tokio::net::TcpListener,
    state: Arc<api::AppState>,
) -> std::io::Result<()> {
    axum::serve(listener, api::router(state)).await
}
