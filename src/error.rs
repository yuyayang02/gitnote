use std::io;

use axum::response::IntoResponse;
use reqwest::StatusCode;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not Found")]
    NotFound,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Repository(#[from] git2::Error),

    // #[error("Invalid branch: {0}")]
    // InvaildBranch(String),
    #[error(transparent)]
    Serde(#[from] toml::de::Error),

    #[error("{0}")]
    FormatError(&'static str),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    ApiError(#[from] ApiError),

    #[error(transparent)]
    Io(#[from] io::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::Repository(e) => {
                tracing::error!(%e, "git repo error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
            .into_response(),
            Error::Sqlx(e) => {
                tracing::error!(%e, "sqlx error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
            .into_response(),
            Error::Reqwest(_) => (StatusCode::BAD_GATEWAY, "Bad Gateway").into_response(),
            Error::ApiError(api_error) => match api_error {
                ApiError::NotFound => (StatusCode::NOT_FOUND, "NOT FOUND").into_response(),
            },
            Error::FormatError(s) => (StatusCode::BAD_REQUEST, s.to_string()).into_response(),
            Error::Serde(e) => (StatusCode::BAD_REQUEST, e.message().to_string()).into_response(),
            Error::Io(e) => {
                tracing::error!(%e, "file io error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
            .into_response(),
        }
    }
}
