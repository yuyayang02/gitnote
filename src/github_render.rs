use axum::http::{HeaderMap, HeaderValue};
use reqwest::header;
use serde::Serialize;

use crate::articles;
use crate::error::Result;

#[derive(Clone)]
pub struct GithubAPiRenderer {
    client: reqwest::Client,
}

impl Default for GithubAPiRenderer {
    fn default() -> Self {
        Self::new(std::env::var("GITHUB_MARKDOWN_RENDER_KEY").unwrap())
    }
}

impl GithubAPiRenderer {
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
                    HeaderValue::from_str(&format!("Bearer {}", token.as_ref())).unwrap(),
                );
                header
            })
            .build()
            .unwrap();

        Self { client }
    }
}

#[derive(Serialize)]
struct RequestBody<'a> {
    text: &'a str,
    mode: &'a str,
}

impl articles::Renderer for GithubAPiRenderer {
    async fn render<T: AsRef<str>>(&self, content: T) -> Result<String> {
        const GITHUB_MARKDOWN_RENDER_API: &'static str = "https://api.github.com/markdown";

        let resp = self
            .client
            .post(GITHUB_MARKDOWN_RENDER_API)
            .json(&RequestBody {
                text: content.as_ref(),
                mode: "gfm", // 启用 GitHub Flavored Markdown
            })
            .send()
            .await?;
        Ok(resp.text().await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::articles::Renderer;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_render() {
        let render = GithubAPiRenderer::default();

        println!("{:?}", render.render("content").await);
    }
}
