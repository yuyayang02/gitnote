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
        slug: impl Into<String>,
        md_content: impl Into<String>,
    ) -> Self {
        // 去除文件扩展名
        // let name: String = slug.into();


        Self {
            group: group.into(),
            slug: slug.into(),
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

fn parse_to_local<'de, D>(deserializer: D) -> std::result::Result<DateTime<Local>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    for fmt in &["%Y-%m-%d %H:%M:%S", "%Y/%m/%d %H:%M:%S"] {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&s, fmt) {
            return Ok(Local
                .from_local_datetime(&naive_dt)
                .single()
                .ok_or_else(|| serde::de::Error::custom("本地时间不明确"))?);
        }
    }

    for fmt in &["%Y-%m-%d", "%Y/%m/%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(&s, fmt) {
            if let Some(naive_dt) = date.and_hms_opt(0, 0, 0) {
                return Ok(Local
                    .from_local_datetime(&naive_dt)
                    .single()
                    .ok_or_else(|| serde::de::Error::custom("本地时间不明确"))?);
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
datetime = "2024-06-01"
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
            "summary should be included"
        );
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
