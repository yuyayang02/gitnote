pub mod api;
pub mod app;
pub mod content;
pub mod error;
pub mod git;
pub mod git_repo;
pub mod render;
pub mod storage;

use std::path::Path;

use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

use app::App;

pub async fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .with_env_filter(EnvFilter::from_env("GITNOTE_LOG"))
        .init();

    let git_repo_path = git_repo_path();

    let repo = git::GitBareRepository::new(git_repo_path);

    let app = App::new(
        storage::init_db_from_env().await,
        render::GithubAPiRenderer::default(),
        repo.clone(),
    );

    api::run_server(app).await
}

pub fn ensure_bare_repo() {
    let repo_name = std::env::var("GIT_REPO_PATH").expect("GIT_REPO_PATH not set");

    let path = Path::new(repo_name.as_str());

    // 如果路径已存在且是一个 Git 仓库，什么也不做

    assert!(
        path.exists() && path.join("HEAD").exists(),
        "You must first initialize the bare repository."
    )
}

pub fn git_repo_path() -> String {
    let repo_name = std::env::var("GIT_REPO_PATH").expect("GIT_REPO_PATH not set");
    let path = Path::new(repo_name.as_str());

    assert!(
        path.exists() && path.join("HEAD").exists(),
        "You must first initialize the bare repository."
    );

    repo_name
}
