use std::path::Path;

use axum::{Json, Router, extract::State, routing::post};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{
    api::App, articles::ArticleBuilder, error::Result, group::Group, model::ArticleModel,
    repo::RepoEntry,
};

pub fn setup_route() -> Router<App> {
    Router::new()
        .route("/repo/rebuild", post(git_repo_rebuild))
        .route("/repo/update", post(git_repo_update))
}

#[derive(Debug, Deserialize)]
pub struct GitRefUpdate {
    // pub refname: String,
    pub oldrev: String,
    pub newrev: String,
}

async fn git_repo_rebuild(State(app): State<App>) -> Result<StatusCode> {
    let repo = app.repo.open()?;

    let entries = repo.rebuild_all()?;
    let mut tx = app.db.begin().await?;

    // 清除所有数据表
    ArticleModel::reset_all(&mut tx).await?;

    match process_repo_entries(&mut tx, entries, &app).await {
        Ok(_) => {
            tx.commit().await?;
            Ok(StatusCode::OK)
        }

        Err(e) => {
            tx.rollback().await.ok();
            tracing::error!(?e);
            Err(e)
        }
    }
}

async fn git_repo_update(
    State(app): State<App>,
    Json(data): Json<GitRefUpdate>,
) -> Result<(StatusCode, String)> {
    let repo = app.repo.open()?;

    let entries = repo.diff_commits_from_str(data.oldrev, data.newrev)?;

    // 构造最终输出字符串：将 RepoEntry 项逐个格式化为字符串后连接成完整输出
    let resp_str = entries
        .iter() // 遍历 RepoEntry 的迭代器（原始顺序）
        .rev() // 反转迭代顺序，使得较新的条目显示在最下方（由旧到新）
        .map(|s| s.to_string()) // 将每个 RepoEntry 格式化为 String（通过实现的 Display trait）
        .collect::<Vec<_>>() // 收集为一个 String 向量（每行一个）
        .join("\n"); // 使用换行符将每个字符串拼接为最终输出

    // 开启db事务
    let mut tx = app.db.begin().await?;

    match process_repo_entries(&mut tx, entries, &app).await {
        Ok(_) => {
            tx.commit().await?;

            Ok((StatusCode::OK, resp_str))
        }

        Err(e) => {
            tx.rollback().await.ok();
            tracing::error!(?e);
            Err(e)
        }
    }
}

async fn process_repo_entries<'c>(
    mut tx: &mut sqlx::PgTransaction<'c>,
    entries: Vec<RepoEntry>,
    app: &App,
) -> Result<StatusCode> {
    fn slug_from_filename(filename: String) -> String {
        Path::new(&filename)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or(filename)
    }

    for entry in entries {
        match entry {
            RepoEntry::GitNote { group, content } => {
                let group = Group::new(group.to_string_lossy(), content)?;
                ArticleModel::update_group(&mut tx, &group).await?;
            }

            RepoEntry::RemoveGitNote { group } => {
                let group = Group::new_with_meta(group.to_string_lossy(), Default::default());
                ArticleModel::update_group(&mut tx, &group).await?;
            }

            RepoEntry::File {
                group,
                name,
                datetime,
                content,
            } => {
                let article =
                    ArticleBuilder::new(group.to_string_lossy(), slug_from_filename(name), content)
                        .build_with_renderer(&app.renderer)
                        .await?;

                ArticleModel::upsert(&mut tx, article, datetime).await?;
            }

            RepoEntry::RemoveFile { group, name } => {
                ArticleModel::remove(&mut tx, group.to_string_lossy(), slug_from_filename(name))
                    .await?;
            }
        }
    }

    Ok(StatusCode::OK)
}
