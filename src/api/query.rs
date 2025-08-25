use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};

use super::{App, ArticleQuery, Error, Result};

pub fn setup_route() -> Router<App> {
    Router::new()
        .route("/articles", get(articles_list))
        .route("/articles/{slug}", get(articles_get_one))
        .route("/articles/tags", get(articles_tags))
        .route("/articles/categories", get(articels_categories))
}

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

#[derive(Debug, Serialize)]
pub struct ArticleFull {
    #[serde(flatten)]
    meta: ArticleMeta,

    content: String,
}

#[derive(Debug, Serialize)]
pub struct Category {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
pub struct Author {
    name: String,
}

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

async fn articles_tags(State(app): State<App>) -> Result<Json<Vec<String>>> {
    app.db().tags().await.map(|r| Json(r)).map_err(Into::into)
}

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
