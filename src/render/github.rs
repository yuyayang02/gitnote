use axum::http::{HeaderMap, HeaderValue};
use reqwest::header;
use serde::Serialize;

use crate::content;
use crate::error::Result;

/// GithubAPiRenderer 用于将 Markdown 文本渲染为 HTML。
///
/// 它使用 GitHub Markdown API，可以渲染 GitHub Flavored Markdown。
#[derive(Clone)]
pub struct GithubAPiRenderer {
    client: reqwest::Client,
}

impl Default for GithubAPiRenderer {
    /// 从环境变量 GITHUB_MARKDOWN_RENDER_KEY 创建默认渲染器
    ///
    /// - Panics
    ///
    /// 如果环境变量未设置，会 panic
    fn default() -> Self {
        Self::new(
            std::env::var("GITHUB_MARKDOWN_RENDER_KEY")
                .expect("GITHUB_MARKDOWN_RENDER_KEY not set"),
        )
    }
}

impl GithubAPiRenderer {
    /// 使用指定的 GitHub Token 创建渲染器
    ///
    /// ```ignore
    /// let renderer = GithubAPiRenderer::new("your_token");
    /// // 使用环境变量
    /// let renderer = GithubAPiRenderer::default();
    /// ```
    pub fn new<T: AsRef<str>>(token: T) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .default_headers({
                let mut header = HeaderMap::new();
                header.insert(
                    header::ACCEPT,
                    HeaderValue::from_static("application/vnd.github+json"),
                );
                header.insert(
                    "X-GitHub-Api-Version",
                    HeaderValue::from_static("2022-11-28"),
                );
                header.insert(
                    header::AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", token.as_ref()))
                        .expect("Failed to create Authorization header"),
                );
                header
            })
            .build()
            .expect("Failed to build reqwest client");

        Self { client }
    }
}

#[derive(Serialize)]
struct RequestBody<'a> {
    text: &'a str,
    mode: &'a str,
}

impl content::Renderer for GithubAPiRenderer {
    /// 将 Markdown 文本渲染为 HTML
    ///
    async fn render<T: AsRef<str>>(&self, content: T) -> Result<String> {
        const GITHUB_MARKDOWN_RENDER_API: &'static str = "https://api.github.com/markdown";

        let resp = self
            .client
            .post(GITHUB_MARKDOWN_RENDER_API)
            .json(&RequestBody {
                text: content.as_ref(),
                mode: "gfm",
            })
            .send()
            .await?;
        Ok(resp.text().await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::content::Renderer;

    use super::*;

    /// 访问 GitHub API 的测试，需要网络和有效 token
    #[tokio::test]
    #[ignore = "需要访问 github api"]
    async fn test_render() {
        let render = GithubAPiRenderer::default();
        println!("{:?}", render.render("content").await);
    }
}
