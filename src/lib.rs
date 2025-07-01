mod api;
mod articles;
mod db;
mod error;
mod github_render;
mod group;
mod model;
mod repo;

use std::path::Path;

use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

pub async fn run() {
    tracing_subscriber::fmt()
        // .with_target(false)
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .with_env_filter(EnvFilter::new("gitnote=debug,tower_http=debug"))
        .init();

    let app = api::App::new(
        db::init_db_from_env().await,
        github_render::GithubAPiRenderer::default(),
        repo::GitBareRepository::new("gitnote.git"),
    );

    ensure_bare_repo();
    api::run_server(app).await;
}

pub fn ensure_bare_repo() {
    let repo_name = std::env::var("REPO_NAME").unwrap();

    let path = Path::new(repo_name.as_str());

    // 如果路径已存在且是一个 Git 仓库，什么也不做
    if !(path.exists() && path.join("HEAD").exists()) {
        eprintln!("You must first initialize the bare repository.");
        std::process::exit(1)
    }
}
