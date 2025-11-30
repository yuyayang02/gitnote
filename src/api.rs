mod git_sync;
mod query;

use axum::Router;
use tower_http::trace::TraceLayer;
use tracing::instrument;

use crate::{
    error::{Error, Result},
    git_sync::{PersistMode, Persistable, PushKind},
    state::AppState,
    storage::Querier,
};

/// 设置应用的路由。
///
/// 将 `/api` 下的 Git 仓库接口和查询接口组合在一起，并绑定应用状态。
pub fn setup_route(app: AppState) -> Router {
    Router::new()
        .nest("/api", git_sync::setup_route().merge(query::setup_route()))
        .with_state(app)
}

/// 启动 HTTP 服务，并使用给定的路由处理请求。
///
/// 在 `0.0.0.0:3000` 上监听 TCP 连接，并打印启动日志。
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

/// 启动 HTTP 服务，自动设置路由和中间件。
///
/// 1. 生成路由
/// 2. 添加日志和追踪中间件
/// 3. 启动服务器
pub async fn run_server(app: AppState) {
    let router = setup_route(app);
    let router = add_middlewares(router);
    run_server_with_router(router).await
}

/// 为路由添加中间件，包括请求追踪和失败日志记录。
///
/// 日志记录会在请求失败时输出错误信息。
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
                // 空实现，关闭请求日志
            }),
    )
}
