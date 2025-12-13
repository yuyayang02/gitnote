use std::io;

use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;

use crate::git_client;

pub type Result<T> = core::result::Result<T, Error>;

/// 应用统一错误类型
///
/// 包含常见错误来源：
/// - [`git2::Error`]（Git 仓库操作错误）
/// - [`toml::de::Error`]（TOML 解析错误）
/// - [`reqwest::Error`]（HTTP 请求错误）
/// - [`sqlx::Error`]（数据库操作错误）
/// - [`io::Error`]（文件 IO 错误）
/// - 自定义错误消息 [`Error::Custom`] 或 [`Error::NotFound`]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Git(#[from] git_client::GitError),

    /// TOML 解析错误
    #[error(transparent)]
    Serde(#[from] toml::de::Error),

    /// 自定义错误消息
    #[error("{0}")]
    Custom(&'static str),

    /// HTTP 请求错误
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// 数据库操作错误
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    /// 资源未找到
    #[error("Not Found")]
    NotFound,

    /// 文件 IO 错误
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl IntoResponse for Error {
    /// 将 [`Error`] 转换为 HTTP 响应
    ///
    /// 错误对应的 HTTP 状态码：
    /// - [`Error::Sqlx`] -> 500 Internal Server Error
    /// - [`Error::Reqwest`] -> 502 Bad Gateway
    /// - [`Error::NotFound`] -> 404 Not Found
    /// - [`Error::Custom`] -> 400 Bad Request
    /// - [`Error::Serde`] -> 400 Bad Request
    /// - [`Error::Io`] -> 500 Internal Server Error
    fn into_response(self) -> Response {
        match self {
            Error::Git(e) => {
                tracing::error!(%e, "git repo error");
                match e {
                    git_client::GitError::NotFound | git_client::GitError::NotExist => {
                        (StatusCode::NOT_FOUND, e.to_string())
                    }
                    git_client::GitError::Git2(e) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, e.message().to_string())
                    }
                    git_client::GitError::IO(e) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                    }
                    git_client::GitError::CommandFailed(s) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, s)
                    }
                }
                .into_response()
            }

            Error::Sqlx(e) => {
                tracing::error!(%e, "sqlx error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
            .into_response(),

            Error::Reqwest(_) => (StatusCode::BAD_GATEWAY, "Bad Gateway").into_response(),

            Error::NotFound => (StatusCode::NOT_FOUND, "NOT FOUND").into_response(),

            Error::Custom(s) => (StatusCode::BAD_REQUEST, s.to_string()).into_response(),

            Error::Serde(e) => (StatusCode::BAD_REQUEST, e.message().to_string()).into_response(),

            Error::Io(e) => {
                tracing::error!(%e, "file io error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
            .into_response(),
        }
    }
}
