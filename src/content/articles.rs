use std::path::Path;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use serde::{Deserialize, Deserializer};

use crate::error::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub summary: String,
    #[serde(deserialize_with = "parse_to_local")]
    pub datetime: DateTime<Local>,
    pub tags: Vec<String>,
}

#[derive(Debug)]
pub struct Article {
    pub group: String,
    pub slug: String,
    pub frontmatter: FrontMatter,
    pub rendered_content: String,
}

#[derive(Debug)]
pub struct ArticleRef<'a> {
    pub slug: &'a str,
    pub group: &'a str,
}

pub struct NoContent;
pub struct Content(String);

pub struct ArticleBuilder<T> {
    group: String,
    slug: String,
    content: T,
}

pub trait Renderer: Send + Sync {
    fn render<T: AsRef<str>>(
        &self,
        content: T,
    ) -> impl std::future::Future<Output = Result<String>>;
}

impl ArticleBuilder<NoContent> {
    pub fn new(path: impl AsRef<Path>) -> Self {
        // 去除文件扩展名
        let path = path.as_ref();
        let group = path
            .parent()
            .map(|p| p.to_string_lossy().trim_matches('/').to_string())
            .unwrap_or_default();

        let slug = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        Self {
            group,
            slug,
            content: NoContent,
        }
    }

    pub fn to_ref<'a>(&'a self) -> ArticleRef<'a> {
        ArticleRef {
            slug: &self.slug,
            group: &self.group,
        }
    }

    pub fn content(self, md_content: impl Into<String>) -> ArticleBuilder<Content> {
        ArticleBuilder {
            group: self.group,
            slug: self.slug,
            content: Content(md_content.into()),
        }
    }
}

impl<T> ArticleBuilder<T> {
    pub fn group(&self) -> &str {
        &self.group
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }
}

impl ArticleBuilder<Content> {
    fn parse_content(&self) -> Result<(FrontMatter, String)> {
        let (toml_str, body_str) = Self::extract_front_matter_and_body(&self.content.0)?;
        let front_matter = Self::parse_front_matter(toml_str)?;
        Ok((front_matter, body_str.to_string()))
    }

    /// 从原始 Markdown 内容中提取 Front Matter 字符串和正文。
    fn extract_front_matter_and_body(content: &str) -> Result<(&str, &str)> {
        const DELIM: &str = "---";

        let content = content.trim_start();

        if !content.starts_with(DELIM) {
            return Err(Error::Custom("Missing required YAML front matter"));
        }

        let rest = &content[DELIM.len()..];
        let end_pos = rest.find(DELIM).ok_or_else(|| {
            Error::Custom("Front matter does not terminate with expected delimiter ---")
        })?;

        let yaml_str = &rest[..end_pos];
        let body_str = &rest[end_pos + DELIM.len()..].trim_start();

        Ok((yaml_str.trim(), body_str))
    }

    /// 解析 YAML 格式的 Front Matter 字符串。
    fn parse_front_matter(yaml_str: &str) -> Result<FrontMatter> {
        serde_yaml::from_str(yaml_str).map_err(Into::into)
    }

    pub async fn build_with_renderer<R: Renderer>(self, renderer: &R) -> Result<Article> {
        let (mut frontmatter, body) = self.parse_content()?;

        let (rendered_content, rendered_summary) =
            tokio::try_join!(renderer.render(body), renderer.render(&frontmatter.summary))?;

        frontmatter.summary = rendered_summary;

        Ok(Article {
            group: self.group,
            slug: self.slug,
            frontmatter,
            rendered_content,
        })
    }
}

fn parse_to_local<'de, D>(deserializer: D) -> std::result::Result<DateTime<Local>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    for fmt in &["%Y-%m-%d %H:%M:%S", "%Y/%m/%d %H:%M:%S"] {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&s, fmt) {
            return Local
                .from_local_datetime(&naive_dt)
                .single()
                .ok_or_else(|| serde::de::Error::custom("本地时间不明确"));
        }
    }

    for fmt in &["%Y-%m-%d", "%Y/%m/%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(&s, fmt) {
            if let Some(naive_dt) = date.and_hms_opt(0, 0, 0) {
                return Local
                    .from_local_datetime(&naive_dt)
                    .single()
                    .ok_or_else(|| serde::de::Error::custom("本地时间不明确"));
            } else {
                return Err(serde::de::Error::custom("无法构建时间"));
            }
        }
    }

    Err(serde::de::Error::custom(format!("无法解析日期: {}", s)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 模拟 Renderer：只包裹一层 <rendered> 标签
    struct FakeRenderer;

    impl Renderer for FakeRenderer {
        fn render<T: AsRef<str>>(
            &self,
            content: T,
        ) -> impl std::future::Future<Output = Result<String>> {
            let content = content.as_ref().to_string();
            async move { Ok(format!("<rendered>{}</rendered>", content)) }
        }
    }

    fn sample_markdown() -> String {
        r#"
---
title: Test Article
summary: |
    This is a test summary.

    This is a test summary too.
datetime: 2024-06-01
tags: [ rust, testing ]
---

# Markdown Content

This is the body of the article.
"#
        .to_string()
    }

    #[tokio::test]
    async fn test_article_builder_with_valid_front_matter() {
        let markdown = sample_markdown();

        // 注意 ArticleBuilder::new 需要 path 和 updated_at
        let builder = ArticleBuilder::new("group-a/test-article.md").content(markdown);
        let article = builder
            .build_with_renderer(&FakeRenderer)
            .await
            .expect("Failed to build article");

        // 校验 group 和 slug
        assert_eq!(article.group, "group-a");
        assert_eq!(article.slug, "test-article");

        // 校验 frontmatter
        assert_eq!(article.frontmatter.title, "Test Article");
        assert_eq!(article.frontmatter.tags, vec!["rust", "testing"]);

        // 校验渲染内容
        assert!(
            article.frontmatter.summary.contains("<rendered>"),
            "Rendered summary should wrap with <rendered> tag"
        );
        assert!(
            article
                .frontmatter
                .summary
                .contains("This is a test summary."),
            "Summary should include original text"
        );
        assert!(
            article
                .frontmatter
                .summary
                .contains("This is a test summary too."),
            "Summary should include original text"
        );
        assert!(
            article.rendered_content.contains("<rendered>"),
            "Rendered content should wrap with <rendered> tag"
        );
        assert!(
            article.rendered_content.contains("This is the body"),
            "Body content should include original markdown"
        );
    }

    #[tokio::test]
    async fn test_article_builder_missing_front_matter_should_fail() {
        let markdown = r#"
# No Front Matter

This article has no TOML front matter.
"#;

        let builder = ArticleBuilder::new("group-a/invalid-article.md").content(markdown);
        let result = builder.build_with_renderer(&FakeRenderer).await;

        assert!(result.is_err(), "Should fail due to missing front matter");
    }
}
