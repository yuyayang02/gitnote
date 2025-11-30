pub mod api;
pub mod content;
pub mod error;
pub mod git_client;
pub mod git_sync;
pub mod render;
pub mod state;
pub mod storage;

use std::env;

use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

pub const REPO_PATH: &str = env!("REPO_PATH");

pub async fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .with_env_filter(EnvFilter::from_env("GITNOTE_LOG"))
        .init();

    let app = {
        let db = storage::init_db_from_env().await;
        state::AppState::new(db, render::GithubAPiRenderer::default(), REPO_PATH)
    };

    api::run_server(app).await
}
