use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::git::AsSummary;

use super::{App, PersistMode, RefKind, RepoEntryPersist, Result};

pub fn setup_route() -> Router<App> {
    Router::new().route("/repo/update", post(update))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitUpdateHookArgs {
    pub refname: String,
    pub oldrev: String,
    pub newrev: String,
}

async fn update(State(app): State<App>, Json(data): Json<GitUpdateHookArgs>) -> Result<Response> {
    let ref_kind = RefKind::parse_ref_kind(&data.refname);

    match ref_kind {
        RefKind::MainBranch => {
            let entries = app
                .repo()
                .open()?
                .diff_commits(&data.oldrev, &data.newrev)?;
            entries.persist(app, PersistMode::Incremental).await?;
            return Ok((StatusCode::OK, entries.as_summary()).into_response());
        }
        RefKind::Archive => {
            let info = app.repo().open()?.archive(&data.newrev)?;
            return Ok((StatusCode::OK, info.as_summary()).into_response());
        }
        RefKind::ArchiveMerge => {
            return Ok((StatusCode::NOT_IMPLEMENTED, "Not implemented yet").into_response());
        }
        RefKind::Rebuild => {
            app.repo()
                .open()?
                .diff_all_with_archive()?
                .persist(app, PersistMode::ResetAll)
                .await?;
        }
        RefKind::RebuildAll => {
            app.repo()
                .open()?
                .diff_all()?
                .persist(app, PersistMode::ResetAll)
                .await?;
        }
        _ => return Ok(StatusCode::NO_CONTENT.into_response()),
    };

    Ok(StatusCode::OK.into_response())
}
