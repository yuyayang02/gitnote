mod git_repo;
mod query;

use axum::Router;
use tower_http::trace::TraceLayer;
use tracing::instrument;

use crate::{
    app::App,
    error::{Error, Result},
    git_repo::{PersistMode, RefKind, RepoEntryPersist},
    storage::ArticleQuery,
};

pub fn setup_route(app: App) -> Router {
    Router::new()
        .nest("/api", git_repo::setup_route().merge(query::setup_route()))
        .with_state(app)
}

#[instrument(name = "http server", skip_all)]
pub async fn run_server_with_router(router: Router) {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind TCP listener on 0.0.0.0:3000");

    tracing::info!("listening on :3000");

    axum::serve(listener, router)
        .await
        .expect("Failed to start Axum server");
}

pub async fn run_server(app: App) {
    let router = setup_route(app);
    let router = add_middlewares(router);
    run_server_with_router(router).await
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
