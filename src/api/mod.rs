mod git_repo;

mod query;

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::trace::TraceLayer;

use crate::{db::Db, github_render::GithubAPiRenderer, repo::GitBareRepository};

#[derive(Clone)]
pub struct App {
    db: Arc<Db>,
    renderer: GithubAPiRenderer,
    repo: Arc<GitBareRepository>,
}

impl App {
    pub fn new(db: Db, renderer: GithubAPiRenderer, repo: GitBareRepository) -> Self {
        Self {
            db: Arc::new(db),
            renderer,
            repo: Arc::new(repo),
        }
    }
}

pub async fn run_server(app: App) {
    let router = Router::new()
        .route("/repo/update", post(git_repo::git_repo_update))
        .route("/articles", get(query::articles_list))
        .route("/articles/{slug}", get(query::articles_get_one))
        .route("/tags", get(query::articles_tags))
        .route("/categories", get(query::articels_categories))
        .with_state(app);

    let router = add_middlewares(router);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on :3000");
    axum::serve(listener, router).await.unwrap();
}

fn add_middlewares(router: Router) -> Router {
    fn log_failure(
        err: tower_http::classify::ServerErrorsFailureClass,
        _latency: std::time::Duration,
        _span: &tracing::Span,
    ) {
        tracing::error!(error = %err, "request failed");
    }

    router.layer(
        TraceLayer::new_for_http()
            .on_failure(log_failure)
            .on_request(|_req: &_, _span: &tracing::Span| {
                // 空实现或省略此行即可关闭请求日志
            }),
    )
}
