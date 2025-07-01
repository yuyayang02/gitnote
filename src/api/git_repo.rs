use axum::{Json, extract::State};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{
    api::App, articles::ArticleBuilder, error::Result, group::Group, model::ArticleModel,
    repo::RepoEntry,
};

#[derive(Debug, Deserialize)]
pub struct GitRefUpdate {
    pub refname: String,
    pub oldrev: String,
    pub newrev: String,
}

pub async fn git_repo_update(
    State(app): State<App>,
    Json(data): Json<GitRefUpdate>,
) -> Result<StatusCode> {
    let repo = app.repo.open(data.refname)?;

    let entries = repo.diff_commits_from_str(data.oldrev, data.newrev)?;

    // 开启db事务
    let mut tx = app.db.begin().await?;

    match (|| async {
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
                    let article = ArticleBuilder::new(group.to_string_lossy(), name, content)
                        .build_with_renderer(&app.renderer)
                        .await?;

                    ArticleModel::upsert(&mut tx, article, datetime).await?;
                }

                RepoEntry::RemoveFile { group, name } => {
                    ArticleModel::remove(&mut tx, name, group.to_string_lossy()).await?;
                }
            }
        }
        Result::Ok(())
    })()
    .await
    {
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
