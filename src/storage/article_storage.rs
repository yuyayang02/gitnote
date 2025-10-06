use sqlx::{PgExecutor, types::Json};

use crate::content::{Article, ArticleBuilder, Group, NoContent};

/// 提供文章和分组的数据库操作接口
///
/// 支持增删改查，包括文章的 [`ArticleBuilder`]、[`Article`] 和组的 [`Group`]。
pub trait ArticleStorage {
    /// 获取 SQL 执行器，用于 [`sqlx::query()`] 执行
    fn executor<'t>(&'t mut self) -> impl PgExecutor<'t>;

    /// 清空所有文章和组
    ///
    /// 会删除 `articles` 表和 `groups` 表中的全部内容
    ///
    fn reset_all(&mut self) -> impl std::future::Future<Output = Result<(), sqlx::Error>> {
        async {
            sqlx::query(
                "
                WITH deleted_articles AS (
                    DELETE FROM articles RETURNING 1
                )
                DELETE FROM groups
                ",
            )
            .execute(self.executor())
            .await?;
            Ok(())
        }
    }

    /// 删除指定的文章
    fn remove(
        &mut self,
        article_builder: &ArticleBuilder<NoContent>,
    ) -> impl std::future::Future<Output = Result<(), sqlx::Error>> {
        async {
            sqlx::query("DELETE FROM articles WHERE slug = $1 AND group_path = $2")
                .bind(article_builder.slug())
                .bind(article_builder.group())
                .execute(self.executor())
                .await?;
            Ok(())
        }
    }

    /// 插入或更新文章
    ///
    /// 使用 [`ON CONFLICT`] 实现“存在则更新，否则插入”
    fn upsert(
        &mut self,
        article: &Article,
    ) -> impl std::future::Future<Output = Result<(), sqlx::Error>> {
        async {
            sqlx::query(
                "
                INSERT INTO articles
                    (slug, group_path, title, summary, tags, content, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (slug)
                DO UPDATE SET
                    group_path = EXCLUDED.group_path,
                    title = EXCLUDED.title,
                    summary = EXCLUDED.summary,
                    tags = EXCLUDED.tags,
                    content = EXCLUDED.content,
                    updated_at = EXCLUDED.updated_at
                ",
            )
            .bind(&article.slug)
            .bind(&article.group)
            .bind(&article.frontmatter.title)
            .bind(&article.frontmatter.summary)
            .bind(&article.frontmatter.tags)
            .bind(&article.rendered_content)
            .bind(&article.frontmatter.datetime)
            .bind(&article.frontmatter.datetime)
            .execute(self.executor())
            .await?;
            Ok(())
        }
    }

    /// 插入或更新组
    fn update_group(
        &mut self,
        group: &Group,
    ) -> impl std::future::Future<Output = Result<(), sqlx::Error>> {
        async {
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
            .execute(self.executor())
            .await?;
            Ok(())
        }
    }

    /// 删除组
    fn remove_group(
        &mut self,
        group: &Group,
    ) -> impl std::future::Future<Output = Result<(), sqlx::Error>> {
        async {
            sqlx::query(
                r#"
                    DELETE FROM groups
                    WHERE path = $1
                "#,
            )
            .bind(&group.name)
            .execute(self.executor())
            .await?;
            Ok(())
        }
    }
}

/// 为 [`sqlx::PgTransaction`] 实现 [`ArticleStorage`]
impl ArticleStorage for sqlx::PgTransaction<'_> {
    fn executor<'t>(&'t mut self) -> impl PgExecutor<'t> {
        self.as_mut()
    }
}

use super::Db;

/// 为 [`Db`] 实现 [`ArticleStorage`]
impl ArticleStorage for &'_ Db {
    fn executor<'t>(&'t mut self) -> impl PgExecutor<'t> {
        *self
    }
}
