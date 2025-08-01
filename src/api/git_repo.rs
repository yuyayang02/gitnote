use std::path::Path;

use axum::{Json, Router, extract::State, routing::post};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    api::App, articles::ArticleBuilder, error::Result, group::Group, model::ArticleRepo,
    repo::RepoEntry,
};

pub fn setup_route() -> Router<App> {
    Router::new()
        .route("/repo/update", post(update))
}

#[derive(Debug)]
pub enum RefKind<'a> {
    MainBranch,
    ArchiveTag(&'a str),
    RebuildTag,
    Other,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitUpdateHookArgs {
    pub refname: String,
    pub oldrev: String,
    pub newrev: String,
}

impl GitUpdateHookArgs {
    fn ref_kind(&self) -> RefKind {
        match self.refname.as_str() {
            "refs/heads/main" => RefKind::MainBranch,
            "refs/tags/rebuild" => RefKind::RebuildTag,
            r if r.starts_with("refs/tags/archive/") => {
                let tag = r.trim_start_matches("refs/tags/archive/");
                if tag.is_empty() {
                    RefKind::Other
                } else {
                    RefKind::ArchiveTag(tag)
                }
            }
            _ => RefKind::Other,
        }
    }

    /// 判断此次操作是否是删除（newrev 全为 0）
    fn is_deletion(&self) -> bool {
        self.newrev == "0000000000000000000000000000000000000000"
    }
}

#[derive(Debug)]
pub struct RepoEntries(pub Vec<RepoEntry>);

impl From<Vec<RepoEntry>> for RepoEntries {
    fn from(v: Vec<RepoEntry>) -> Self {
        Self(v)
    }
}

impl RepoEntries {
    pub fn as_summary_string(&self) -> String {
        self.0
            .iter() // 遍历 RepoEntry 的迭代器（原始顺序）
            .rev() // 反转迭代顺序，使得较新的条目显示在最下方（由旧到新）
            .map(|s| s.to_string()) // 将每个 RepoEntry 格式化为 String（通过实现的 Display trait）
            .collect::<Vec<_>>() // 收集为一个 String 向量（每行一个）
            .join("\n") // 使用换行符将每个字符串拼接为最终输出
    }

    pub async fn persist_to_db(self, app: &App, reset: bool) -> Result<()> {
        let mut tx = app.db.begin().await?;

        if reset {
            ArticleRepo::reset_all(&mut tx).await?;
        }

        match self.persist_to_db_with_tx(app, &mut tx).await {
            Ok(value) => {
                tx.commit().await?;
                Ok(value)
            }
            Err(e) => {
                if let Err(rollback_err) = tx.rollback().await {
                    tracing::warn!("transaction rollback failed: {:?}", rollback_err);
                }
                Err(e)
            }
        }
    }

    async fn persist_to_db_with_tx<'c>(
        self,
        app: &App,
        tx: &mut sqlx::PgTransaction<'c>,
    ) -> Result<()> {
        for entry in self.0 {
            match entry {
                RepoEntry::GitNote { group, content } => {
                    let group = Group::new(group.to_string_lossy(), content)?;
                    ArticleRepo::update_group(tx, &group).await?;
                }

                RepoEntry::RemoveGitNote { group } => {
                    let group = Group::new_with_meta(group.to_string_lossy(), Default::default());
                    ArticleRepo::update_group(tx, &group).await?;
                }

                RepoEntry::File {
                    group,
                    name,
                    datetime,
                    content,
                } => {
                    let article = ArticleBuilder::new(
                        group.to_string_lossy(),
                        Self::slug_from_filename(name),
                        content,
                    )
                    .build_with_renderer(&app.renderer)
                    .await?;

                    ArticleRepo::upsert(tx, article, datetime).await?;
                }

                RepoEntry::RemoveFile { group, name } => {
                    ArticleRepo::remove(
                        tx,
                        group.to_string_lossy(),
                        Self::slug_from_filename(name),
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }

    fn slug_from_filename(filename: String) -> String {
        Path::new(&filename)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or(filename)
    }
}

async fn update(
    State(app): State<App>,
    Json(data): Json<GitUpdateHookArgs>,
) -> Result<(StatusCode, String)> {
    let ref_kind = data.ref_kind();
    match (ref_kind, data.is_deletion()) {
        (RefKind::ArchiveTag(tag), false) => {
            let info = app.repo.archive(tag, &data.newrev)?;
            Ok((StatusCode::OK, info.summary()))
        }

        (RefKind::MainBranch, false) => {
            let entries: RepoEntries = app.repo.diff_commit(data.oldrev, data.newrev)?.into();
            let entries_str = entries.as_summary_string();
            entries.persist_to_db(&app, false).await?;
            Ok((StatusCode::OK, entries_str))
        }

        (RefKind::RebuildTag, false) => {
            let entries: RepoEntries = app.repo.diff_all()?.into();
            let entries_str = entries.as_summary_string();
            entries.persist_to_db(&app, true).await?;
            Ok((StatusCode::OK, entries_str))
        }

        (RefKind::MainBranch, true) => Ok((
            StatusCode::FORBIDDEN,
            "❌ Deleting 'main' branch is not allowed.".to_string(),
        )),

        // (_, true) => Ok((StatusCode::NO_CONTENT, String::new())), // 忽略删除
        _ => Ok((StatusCode::NO_CONTENT, String::new())),
    }
}
