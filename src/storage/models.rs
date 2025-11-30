use chrono::{DateTime, Local};
use serde::Deserialize;
use sqlx::types::Json;

/// 文章详情
///
/// 包含文章完整内容、元信息以及可选分类和作者信息。
#[derive(Debug, sqlx::FromRow)]
pub struct ArticleDetail {
    /// 文章唯一标识
    pub slug: String,
    /// 标题
    pub title: String,
    /// 摘要
    pub summary: String,
    /// 标签列表
    pub tags: Vec<String>,
    /// 文章内容
    pub content: String,
    /// 文章的分组
    pub group: Json<Group>,
    /// 更新时间
    pub updated_at: DateTime<Local>,
    /// 创建时间
    pub created_at: DateTime<Local>,
}

/// 文章列表项
///
/// 包含文章基础信息，用于列表展示，不包含完整内容。
#[derive(Debug, sqlx::FromRow)]
pub struct ArticleSummary {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub group: Json<Group>,
    pub updated_at: DateTime<Local>,
    pub created_at: DateTime<Local>,
}

/// 组信息
#[derive(Debug, sqlx::FromRow, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub public: bool,
    pub kind: Json<serde_json::Value>,
}
