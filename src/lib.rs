pub mod api;
pub mod app;
pub mod content;
pub mod error;
pub mod git;
pub mod git_repo;
pub mod render;
pub mod storage;

use std::env;

use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

use app::App;

pub async fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .with_env_filter(EnvFilter::from_env("GITNOTE_LOG"))
        .init();

    let app = App::new(
        storage::init_db_from_env().await,
        render::GithubAPiRenderer::default(),
        &repo_path(),
    );

    api::run_server(app).await
}

fn repo_path() -> String {
    env::var("GIT_REPO_PATH").expect("GIT_REPO_PATH not set")
}
