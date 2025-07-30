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

use crate::repo::ArchiveTagger;

pub async fn run() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .with_env_filter(EnvFilter::from_env("GITNOTE_LOG"))
        .init();

    let repo = repo::GitBareRepository::new("gitnote.git");

    let app = api::App::new(
        db::init_db_from_env().await,
        github_render::GithubAPiRenderer::default(),
        repo.clone(),
    );

    let tagger = ArchiveTagger::new(repo, (4, 30), 24 * 60 * 60);

    ensure_bare_repo();

    tokio::join!(api::run_server(app), tagger.run_scheduled_task());
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
