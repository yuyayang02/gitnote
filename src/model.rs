use chrono::{DateTime, Local};
use sqlx::types::Json;
use sqlx::{QueryBuilder, Row};

use crate::{articles::Article, error::Result, group::Group};

#[derive(Debug, sqlx::FromRow)]
pub struct ArticleDetail {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Local>,
    pub created_at: DateTime<Local>,
    pub content: String,

    // 平铺字段
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub author_name: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ArticleListItem {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Local>,
    pub created_at: DateTime<Local>,

    // 平铺字段
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub author_name: Option<String>,
}

pub struct ArticleModel;

impl ArticleModel {
    pub async fn reset_all<'c>(tx: &mut sqlx::PgTransaction<'c>) -> Result<()> {
        sqlx::query(
            "
                WITH deleted_articles AS (
                    DELETE FROM articles RETURNING 1
                )
                DELETE FROM groups
                ",
        )
        .execute(tx.as_mut())
        .await?;
        Ok(())
    }

    pub async fn upsert<'c>(
        tx: &mut sqlx::PgTransaction<'c>,
        article: Article,
        updated_at: DateTime<Local>,
    ) -> Result<()> {
        sqlx::query(
            "
            INSERT INTO articles
                (slug, group_path, title, summary, tags, content, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (group_path, slug)
            DO UPDATE SET
                group_path = $2,
                title = $3,
                summary = $4,
                tags = $5,
                content = $6,
                updated_at = $8
            ",
        )
        .bind(&article.slug)
        .bind(&article.group)
        .bind(&article.frontmatter.title)
        .bind(&article.frontmatter.summary)
        .bind(&article.frontmatter.tags)
        .bind(&article.rendered_content)
        .bind(&article.frontmatter.datetime)
        .bind(updated_at)
        .execute(tx.as_mut())
        .await?;

        Ok(())
    }

    pub async fn remove<'c>(
        tx: &mut sqlx::PgTransaction<'c>,
        group: impl AsRef<str>,
        slug: impl AsRef<str>,
    ) -> Result<()> {
        sqlx::query("DELETE FROM articles WHERE slug = $1 AND group_path = $2")
            .bind(slug.as_ref())
            .bind(group.as_ref())
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }

    pub async fn update_group<'c>(tx: &mut sqlx::PgTransaction<'c>, group: &Group) -> Result<()> {
        sqlx::query(
            r#"
                    INSERT INTO groups (path, public, category, author)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT (path) DO UPDATE 
                    SET 
                        public = EXCLUDED.public,
                        category = EXCLUDED.category,
                        author = EXCLUDED.author
                "#,
        )
        .bind(&group.name)
        .bind(&group.meta.public)
        .bind(Json(&group.meta.category))
        .bind(Json(&group.meta.author))
        .execute(tx.as_mut())
        .await?;
        Ok(())
    }
}

impl ArticleModel {
    pub async fn get_one<'a, E: sqlx::PgExecutor<'a>>(
        executor: E,
        slug: impl AsRef<str>,
    ) -> Result<Option<ArticleDetail>> {
        Ok(sqlx::query_as::<_, ArticleDetail>(
            r#"
                SELECT
                    a.slug,
                    a.title,
                    a.summary,
                    a.tags,
                    a.content,
                    a.updated_at,
                    a.created_at,
                    g.category ->> 'id' AS category_id,
                    g.category ->> 'name' AS category_name,
                    g.author ->> 'name' AS author_name
                FROM
                    articles a
                INNER JOIN
                    groups g ON a.group_path = g.path
                WHERE
                    a.slug = $1
                    AND g.public = true;
                "#,
        )
        .bind(slug.as_ref())
        .fetch_optional(executor)
        .await?)
    }

    pub async fn list<'a, E: sqlx::PgExecutor<'a>>(
        executor: E,
        limit: i32,
        page: i32,
        category: Option<String>,
        author: Option<String>,
        tags: Vec<String>,
    ) -> Result<Vec<ArticleListItem>> {
        let offset = (page.max(1) - 1) * limit;

        let mut builder = QueryBuilder::new(
            r#"
            SELECT
                a.slug,
                a.title,
                a.summary,
                a.tags,
                a.updated_at,
                a.created_at,
                g.category ->> 'id' AS category_id,
                g.category ->> 'name' AS category_name,
                g.author ->> 'name' AS author_name
            FROM articles a
            INNER JOIN groups g ON a.group_path = g.path
            "#,
        );

        builder.push("WHERE g.public = TRUE");

        if let Some(cat) = category {
            builder.push(" AND g.category->>'id' = ");
            builder.push_bind(cat);
        }

        if let Some(auth) = author {
            builder.push(" AND g.author->>'name' = ");
            builder.push_bind(auth);
        }

        if !tags.is_empty() {
            builder.push(" AND a.tags && ");
            builder.push_bind(tags);
        }

        builder.push(" ORDER BY a.updated_at DESC ");
        builder.push(" LIMIT ");
        builder.push_bind(limit);
        builder.push(" OFFSET ");
        builder.push_bind(offset);

        let query = builder.build_query_as::<ArticleListItem>();
        let articles = query.fetch_all(executor).await?;

        Ok(articles)
    }

    pub async fn categories<'a, E: sqlx::PgExecutor<'a>>(
        executor: E,
    ) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query(
            r#"
                SELECT DISTINCT
                    category->>'id' AS id,
                    category->>'name' AS name
                FROM groups
                WHERE public = true
                AND category IS NOT NULL
                ORDER BY id
                "#,
        )
        .fetch_all(executor)
        .await?;

        let result = rows
            .into_iter()
            .filter_map(|row| {
                let id: Option<String> = row.try_get("id").ok();
                let name: Option<String> = row.try_get("name").ok();
                match (id, name) {
                    (Some(id), Some(name)) => Some((id, name)),
                    _ => None,
                }
            })
            .collect();

        Ok(result)
    }

    pub async fn tags<'a, E: sqlx::PgExecutor<'a>>(executor: E) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar(
            r#"
            SELECT DISTINCT UNNEST(a.tags) AS "tag"
            FROM articles a
            JOIN groups g ON a.group_path = g.path
            WHERE g.public = true
            ORDER BY tag
            "#,
        )
        .fetch_all(executor)
        .await?)
    }
}
