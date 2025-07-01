use crate::api::App;
use crate::error::{ApiError, Result};
use crate::model::ArticleModel;
use axum::Json;
use axum::extract::{Path, State};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ArticleMeta {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub category: Option<Category>,
    pub author: Option<Author>,
    pub updated_at: i64,
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

pub async fn articles_get_one(
    Path(slug): Path<String>,
    State(app): State<App>,
) -> Result<Json<ArticleFull>> {
    let article = ArticleModel::get_one(app.db.as_ref(), slug)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(ArticleFull {
        meta: ArticleMeta {
            slug: article.slug,
            title: article.title,
            summary: article.summary,
            tags: article.tags,
            category: article
                .category_id
                .zip(article.category_name)
                .map(|(id, name)| Category { id, name }),
            author: article.author_name.map(|name| Author { name }),
            updated_at: article.updated_at.timestamp_millis(),
        },
        content: article.content,
    }))
}

pub async fn articles_tags(State(app): State<App>) -> Result<Json<Vec<String>>> {
    ArticleModel::tags(app.db.as_ref()).await.map(|r| Json(r))
}

pub async fn articels_categories(State(app): State<App>) -> Result<Json<Vec<Category>>> {
    ArticleModel::categories(app.db.as_ref()).await.map(|a| {
        Json(
            a.into_iter()
                .map(|(id, name)| Category { id, name })
                .collect(),
        )
    })
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

pub async fn articles_list(
    Query(params): Query<QueryParams>,
    State(app): State<App>,
) -> Result<Json<Vec<ArticleMeta>>> {
    Ok(Json(
        ArticleModel::list(
            app.db.as_ref(),
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
                    category: a
                        .category_id
                        .zip(a.category_name)
                        .map(|(id, name)| Category { id, name }),
                    author: a.author_name.map(|name| Author { name }),
                    updated_at: a.updated_at.timestamp_millis(),
                })
                .collect()
        })?,
    ))
}
