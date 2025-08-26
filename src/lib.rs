pub mod api;
pub mod app;
pub mod content;
pub mod error;
pub mod git;
pub mod git_repo;
pub mod render;
pub mod storage;

use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

use app::App;

pub async fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .with_env_filter(EnvFilter::from_env("GITNOTE_LOG"))
        .init();

    git::init_git_repositories_from_env();

    let app = App::new(
        storage::init_db_from_env().await,
        render::GithubAPiRenderer::default(),
    );

    api::run_server(app).await
}
