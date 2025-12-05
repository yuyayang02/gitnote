use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};

use super::{Error, Querier, Result};

use crate::{state::AppState, storage::DBPool};

/// 配置文章相关路由。
///
/// 路由包括：
/// - `GET /articles`：文章列表
/// - `GET /articles/{slug}`：获取单篇文章
/// - `GET /articles/tags`：获取所有标签
/// - `GET /articles/categories`：获取所有分类
pub fn setup_route() -> Router<AppState> {
    Router::new()
        .route("/articles", get(articles_list))
        .route("/articles/{slug}", get(article))
        .route("/tags", get(tag_list))
        .route("/groups", get(group_list))
}

/// 文章元信息，用于列表展示。
#[derive(Debug, Serialize)]
pub struct ArticleMeta {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub group: Group,
    pub updated_at: i64,
    pub created_at: i64,
}

/// 完整文章，包括元信息和正文。
#[derive(Debug, Serialize)]
pub struct ArticleDetail {
    #[serde(flatten)]
    meta: ArticleMeta,

    content: String,
}

/// 文章分类。
#[derive(Debug, Serialize)]
pub struct Group {
    id: String,
    name: String,
}

/// 根据 slug 获取单篇文章。
///
/// 返回 [`ArticleFull`]，如果文章不存在返回 [`Error::NotFound`]。
async fn article(
    Path(slug): Path<String>,
    State(pool): State<DBPool>,
) -> Result<Json<ArticleDetail>> {
    let article = pool.get_one(&slug).await?.ok_or(Error::NotFound)?;

    Ok(Json(ArticleDetail {
        meta: ArticleMeta {
            slug: article.slug,
            title: article.title,
            summary: article.summary,
            tags: article.tags,
            updated_at: article.updated_at.timestamp_millis(),
            created_at: article.created_at.timestamp_millis(),
            group: Group {
                id: article.group.0.id,
                name: article.group.0.name,
            },
        },
        content: article.content,
    }))
}

/// 获取所有文章标签。
///
/// 返回标签列表。
async fn tag_list(State(pool): State<DBPool>) -> Result<Json<Vec<String>>> {
    pool.tags().await.map(Json).map_err(Into::into)
}

/// 获取所有文章分类。
///
/// 返回 [`Category`] 列表。
async fn group_list(State(pool): State<DBPool>) -> Result<Json<Vec<Group>>> {
    match pool.groups().await {
        Ok(data) => Ok(Json(
            data.into_iter()
                .map(|d| Group {
                    id: d.id,
                    name: d.name,
                })
                .collect::<Vec<_>>(),
        )),
        Err(e) => Err(e.into()),
    }
}

/// 查询参数，用于文章列表分页和筛选。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct QueryParams {
    limit: i32,
    page: i32,
    group: Option<String>,
    tags: String,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            limit: 13,
            page: 1,
            group: None,
            tags: Default::default(),
        }
    }
}

/// 获取文章列表。
///
/// 支持分页、作者、分类和标签筛选。
/// 返回 [`ArticleMeta`] 列表。
async fn articles_list(
    Query(params): Query<QueryParams>,
    State(pool): State<DBPool>,
) -> Result<Json<Vec<ArticleMeta>>> {
    match pool
        .article_list(
            params.page,
            params.limit,
            params.group.as_deref(),
            params
                .tags
                .split(",")
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>(),
        )
        .await
    {
        Ok(data) => Ok(Json(
            data.into_iter()
                .map(|a| ArticleMeta {
                    slug: a.slug,
                    title: a.title,

                    summary: a.summary,
                    tags: a.tags,
                    updated_at: a.updated_at.timestamp_millis(),
                    created_at: a.created_at.timestamp_millis(),
                    group: Group {
                        id: a.group.0.id,
                        name: a.group.0.name,
                    },
                })
                .collect(),
        )),
        Err(e) => Err(e.into()),
    }
}
