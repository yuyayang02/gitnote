use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};

use super::{App, ArticleQuery, Error, Result};

/// 配置文章相关路由。
///
/// 路由包括：
/// - `GET /articles`：文章列表
/// - `GET /articles/{slug}`：获取单篇文章
/// - `GET /articles/tags`：获取所有标签
/// - `GET /articles/categories`：获取所有分类
pub fn setup_route() -> Router<App> {
    Router::new()
        .route("/articles", get(articles_list))
        .route("/articles/{slug}", get(articles_get_one))
        .route("/articles/tags", get(articles_tags))
        .route("/articles/categories", get(articels_categories))
}

/// 文章元信息，用于列表展示。
#[derive(Debug, Serialize)]
pub struct ArticleMeta {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub category: Option<Category>,
    pub author: Option<Author>,
    pub updated_at: i64,
    pub created_at: i64,
}

/// 完整文章，包括元信息和正文。
#[derive(Debug, Serialize)]
pub struct ArticleFull {
    #[serde(flatten)]
    meta: ArticleMeta,

    content: String,
}

/// 文章分类。
#[derive(Debug, Serialize)]
pub struct Category {
    id: String,
    name: String,
}

/// 文章作者。
#[derive(Debug, Serialize)]
pub struct Author {
    name: String,
}

/// 根据 slug 获取单篇文章。
///
/// 返回 [`ArticleFull`]，如果文章不存在返回 [`Error::NotFound`]。
async fn articles_get_one(
    Path(slug): Path<String>,
    State(app): State<App>,
) -> Result<Json<ArticleFull>> {
    let article = app.db().get_one(&slug).await?.ok_or(Error::NotFound)?;

    Ok(Json(ArticleFull {
        meta: ArticleMeta {
            slug: article.slug,
            title: article.title,
            summary: article.summary,
            tags: article.tags,
            category: article.category.map(|c| Category {
                id: c.0.id,
                name: c.0.name,
            }),
            author: article.author_name.map(|name| Author { name }),
            updated_at: article.updated_at.timestamp_millis(),
            created_at: article.created_at.timestamp_millis(),
        },
        content: article.content,
    }))
}

/// 获取所有文章标签。
///
/// 返回标签列表。
async fn articles_tags(State(app): State<App>) -> Result<Json<Vec<String>>> {
    app.db().tags().await.map(Json).map_err(Into::into)
}

/// 获取所有文章分类。
///
/// 返回 [`Category`] 列表。
async fn articels_categories(State(app): State<App>) -> Result<Json<Vec<Category>>> {
    app.db()
        .categories()
        .await
        .map(|c_vec| {
            Json(
                c_vec
                    .into_iter()
                    .map(|c| Category {
                        id: c.id,
                        name: c.name,
                    })
                    .collect(),
            )
        })
        .map_err(Into::into)
}

/// 查询参数，用于文章列表分页和筛选。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct QueryParams {
    limit: i32,
    page: i32,
    author: Option<String>,
    category: Option<String>,
    tags: Vec<String>, // 支持 tags=rust&tags=test
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            limit: 13,
            page: 1,
            author: None,
            category: None,
            tags: Vec::default(),
        }
    }
}

/// 获取文章列表。
///
/// 支持分页、作者、分类和标签筛选。
/// 返回 [`ArticleMeta`] 列表。
async fn articles_list(
    Query(params): Query<QueryParams>,
    State(app): State<App>,
) -> Result<Json<Vec<ArticleMeta>>> {
    Ok(Json(
        app.db()
            .list(
                params.limit,
                params.page,
                params.category,
                params.author,
                params.tags,
            )
            .await
            .map(|va| {
                va.into_iter()
                    .map(|a| ArticleMeta {
                        slug: a.slug,
                        title: a.title,
                        summary: a.summary,
                        tags: a.tags,
                        category: a.category.map(|c| Category {
                            id: c.0.id,
                            name: c.0.name,
                        }),
                        author: a.author_name.map(|name| Author { name }),
                        updated_at: a.updated_at.timestamp_millis(),
                        created_at: a.created_at.timestamp_millis(),
                    })
                    .collect()
            })?,
    ))
}
