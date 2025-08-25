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
    /// 更新时间
    pub updated_at: DateTime<Local>,
    /// 创建时间
    pub created_at: DateTime<Local>,
    /// 文章内容
    pub content: String,

    /// 可选分类信息，参见 [`CategoryInfo`]
    pub category: Option<Json<CategoryInfo>>,
    /// 可选作者名称
    pub author_name: Option<String>,
}

/// 文章列表项
///
/// 包含文章基础信息，用于列表展示，不包含完整内容。
#[derive(Debug, sqlx::FromRow)]
pub struct ArticleListItem {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Local>,
    pub created_at: DateTime<Local>,

    /// 可选分类信息，参见 [`CategoryInfo`]
    pub category: Option<Json<CategoryInfo>>,
    /// 可选作者名称
    pub author_name: Option<String>,
}

/// 分类信息
///
/// 包含分类 ID 和名称。
#[derive(Debug, sqlx::FromRow, Deserialize)]
pub struct CategoryInfo {
    /// 分类 ID
    pub id: String,
    /// 分类名称
    pub name: String,
}
