mod git_repo;

mod query;

use std::sync::Arc;

use axum::Router;
use tower_http::trace::TraceLayer;
use tracing::instrument;

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

#[instrument(name = "http server", skip_all)]
pub async fn run_server(app: App) {
    let router = Router::new()
        .nest("/api", git_repo::setup_route().merge(query::setup_route()))
        .with_state(app);

    let router = add_middlewares(router);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on :3000");
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
