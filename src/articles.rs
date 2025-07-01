use std::path::Path;

use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
}
#[derive(Debug)]
pub struct Article {
    pub group: String,
    pub slug: String,
    pub frontmatter: FrontMatter,
    pub rendered_content: String,
}

pub struct ArticleBuilder {
    group: String,
    slug: String,
    md_content: String,
}

pub trait Renderer {
    fn render<T: AsRef<str>>(
        &self,
        content: T,
    ) -> impl std::future::Future<Output = Result<String>>;
}

impl ArticleBuilder {
    pub fn new(
        group: impl Into<String>,
        name: impl Into<String>,
        md_content: impl Into<String>,
    ) -> Self {
        // 去除文件扩展名
        let name: String = name.into();
        let slug: String = if let Some(s) = Path::new(&name).file_stem() {
            s.to_string_lossy().into()
        } else {
            name
        };

        Self {
            group: group.into(),
            slug,
            md_content: md_content.into(),
        }
    }

    fn parse_content(&self) -> Result<(FrontMatter, String)> {
        const DELIM: &'static str = "+++";

        let content = self.md_content.trim_start(); // 忽略开头空白

        // 必须以 front matter 起始
        if !content.starts_with(DELIM) {
            return Err(Error::FormatError("Missing required TOML front matter"));
        }

        // 去掉起始标志
        let rest = &content[DELIM.len()..];

        // 找到结束标志的位置
        let end_pos = rest.find(DELIM).ok_or_else(|| {
            Error::FormatError("Front matter does not terminate with expected delimiter +++")
        })?;

        // 提取 front matter 和正文
        let toml_str = &rest[..end_pos];
        let body = &rest[end_pos + DELIM.len()..];

        let front_matter: FrontMatter = toml::from_str(toml_str.trim())?;

        Ok((front_matter, body.trim_start().to_string()))
    }

    pub async fn build_with_renderer<R: Renderer>(self, renderer: &R) -> Result<Article> {
        let (mut frontmatter, body) = self.parse_content()?;

        let rendered_content = renderer.render(body).await?;
        frontmatter.summary = renderer.render(&frontmatter.summary).await?;

        Ok(Article {
            group: self.group,
            slug: self.slug,
            frontmatter,
            rendered_content,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 模拟 Renderer：只加标签
    struct FakeRenderer;

    impl Renderer for FakeRenderer {
        async fn render<T: AsRef<str>>(&self, content: T) -> Result<String> {
            Ok(format!("<rendered>{}</rendered>", content.as_ref()))
        }
    }

    #[tokio::test]
    async fn test_article_builder_with_valid_front_matter() {
        let markdown = r#"
+++
title = "Test Article"
summary = "This is a test summary."
tags = ["rust", "testing"]
+++

# Markdown Content

This is the body of the article.
"#;

        let builder = ArticleBuilder::new("group-a", "test-article", markdown);
        let article = builder.build_with_renderer(&FakeRenderer).await.unwrap();

        // 校验元数据
        assert_eq!(article.group, "group-a");
        assert_eq!(article.slug, "test-article");
        assert_eq!(article.frontmatter.title, "Test Article");
        assert_eq!(article.frontmatter.summary, "This is a test summary.");
        assert_eq!(article.frontmatter.tags, vec!["rust", "testing"]);

        // 校验渲染内容
        assert!(
            article.rendered_content.contains("<rendered>"),
            "Rendered content should wrap with <rendered> tag"
        );
        assert!(
            article.rendered_content.contains("This is the body"),
            "Body content should be included"
        );
    }

    #[tokio::test]
    async fn test_article_builder_missing_front_matter_should_fail() {
        let markdown = r#"
# No Front Matter

This article has no TOML front matter.
"#;

        let builder = ArticleBuilder::new("group-a", "invalid-article", markdown);
        let result = builder.build_with_renderer(&FakeRenderer).await;

        assert!(result.is_err(), "Should fail due to missing front matter");
    }
}
