use sqlx::types::Json;

use crate::{
    content::{Article, ArticleRef, Group},
    error,
    storage::DBPool,
};

/// 提供文章和分组的数据库操作接口
///
/// 支持增删改查，包括文章的 [`ArticleRef`]、[`Article`] 和组的 [`Group`]。
pub trait Store: ToOwned + Send + Sync {
    /// 清空所有文章和组
    fn clean(&mut self) -> &mut Self;
    /// 插入或更新组
    fn upsert_group(&mut self, group: &Group) -> &mut Self;
    /// 删除组
    fn remove_group(&mut self, group: &Group) -> &mut Self;
    /// 插入或更新文章
    fn upsert_article(&mut self, article: &Article) -> &mut Self;
    /// 删除指定的文章
    fn remove_article(&mut self, article_ref: ArticleRef<'_>) -> &mut Self;
    /// 提交更改
    fn commit(self) -> impl std::future::Future<Output = Result<(), error::Error>>;
}

/// sqlx 的 [`Store`] 实现
pub struct SqlxStore {
    pool: DBPool,
    queries: Vec<sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments>>,
}

impl SqlxStore {
    pub fn new(pool: DBPool) -> Self {
        Self {
            pool,
            queries: Default::default(),
        }
    }
}

impl ToOwned for SqlxStore {
    type Owned = SqlxStore;

    fn to_owned(&self) -> Self::Owned {
        Self {
            pool: self.pool.clone(),
            queries: Default::default(),
        }
    }
}

impl Store for SqlxStore {
    fn clean(&mut self) -> &mut Self {
        let query = sqlx::query("TRUNCATE TABLE groups, articles");
        self.queries.push(query);
        self
    }

    fn remove_article(&mut self, article_ref: ArticleRef<'_>) -> &mut Self {
        let query = sqlx::query("DELETE FROM articles WHERE slug = $1 AND group_id = $2")
            .bind(article_ref.slug.to_owned())
            .bind(article_ref.group.to_owned());
        self.queries.push(query);
        self
    }

    fn upsert_group(&mut self, group: &Group) -> &mut Self {
        let q = sqlx::query(
            r#"
            INSERT INTO groups (id, name, public, kind)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE
            SET
                public = EXCLUDED.public,
                name = EXCLUDED.name,
                kind = EXCLUDED.kind
            "#,
        )
        .bind(group.id.to_owned())
        .bind(group.name.to_owned())
        .bind(group.public)
        .bind(Json(group.kind.clone()));

        self.queries.push(q);
        self
    }

    fn remove_group(&mut self, group: &Group) -> &mut Self {
        let q = sqlx::query(
            r#"
            DELETE FROM groups
            WHERE id = $1
            "#,
        )
        .bind(group.id.to_owned());

        self.queries.push(q);
        self
    }

    fn upsert_article(&mut self, article: &Article) -> &mut Self {
        let q = sqlx::query(
            "
            INSERT INTO articles
                (slug, group_id, title, summary, tags, content, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (slug)
            DO UPDATE SET
                group_id = EXCLUDED.group_id,
                title = EXCLUDED.title,
                summary = EXCLUDED.summary,
                tags = EXCLUDED.tags,
                content = EXCLUDED.content,
                updated_at = EXCLUDED.updated_at
            ",
        )
        .bind(article.slug.to_owned())
        .bind(article.group.to_owned())
        .bind(article.frontmatter.title.to_owned())
        .bind(article.frontmatter.summary.to_owned())
        .bind(article.frontmatter.tags.to_owned())
        .bind(article.rendered_content.to_owned())
        .bind(article.frontmatter.datetime)
        .bind(article.frontmatter.datetime);

        self.queries.push(q);
        self
    }

    async fn commit(mut self) -> Result<(), error::Error> {
        let mut tx = self.pool.begin().await?;

        for q in self.queries.drain(..) {
            q.execute(tx.as_mut()).await?;
        }

        Ok(tx.commit().await?)
    }
}
