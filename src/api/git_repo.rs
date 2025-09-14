use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
};
use reqwest::StatusCode;

use crate::{
    git::{AsSummary, GitRepository},
    git_repo::GitPushPayload,
};

use super::{App, PersistMode, PushKind, RepoEntryPersist, Result};

/// 配置 Git 仓库更新相关的路由。
///
/// 将 `/repo/update` 注册为 POST 请求，用于处理 Git push 事件。
pub fn setup_route() -> Router<App> {
    Router::new().route("/repo/update", post(update))
}

/// 处理 Git push 请求。
///
/// 根据 push 类型执行不同操作：
///
/// - [`PushKind::Sync`]：对比两个 commit 的差异，并进行增量持久化，同时返回变更摘要。
/// - [`PushKind::Rebuild`]：获取目标 commit 的完整快照，重建数据。
/// - 其他类型：返回 `201 Created` 表示操作成功但没有内容返回。
///
/// 执行流程：
/// 1. 打开并 fetch 仓库
/// 2. 根据 push 类型选择增量或全量处理
/// 3. 调用 [`RepoEntryPersist::persist`] 将数据写入应用
/// 4. 返回 HTTP 响应
async fn update(State(app): State<App>, Json(data): Json<GitPushPayload>) -> Result<Response> {
    tracing::debug!(data = ?data, "git push paylaod");

    let ref_kind = data.push_kind();
    match ref_kind {
        PushKind::Sync => {
            let repo = GitRepository::open(app.repo_path())?;
            let entries = repo.diff_commits(&data.before, &data.after)?;
            entries
                .persist(app, &repo, PersistMode::Incremental)
                .await?;
            Ok((StatusCode::OK, entries.as_summary()).into_response())
        }

        PushKind::Rebuild => {
            let repo = GitRepository::open(&app.repo_path())?;
            let entries = repo.snapshot(&data.after)?;

            entries.persist(app, &repo, PersistMode::ResetAll).await?;
            Ok((StatusCode::OK, entries.as_summary()).into_response())
        }
        _ => Ok(StatusCode::CREATED.into_response()),
    }
}
