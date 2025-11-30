use super::{ArticleDetail, ArticleSummary, DBPool, Group};

/// 用于查询文章相关数据
///
/// 提供获取文章详情、列表、分类和标签的接口。
pub trait Querier: Send + Sync {
    type Error;
    /// 查询单个文章详情
    ///
    /// 返回 [`ArticleDetail`]，如果文章不存在则返回 `None`。
    fn get_one(
        &self,
        slug: impl AsRef<str>,
    ) -> impl std::future::Future<Output = Result<Option<ArticleDetail>, Self::Error>>;

    /// 分页查询文章列表
    ///
    /// 返回 [`ArticleListItem`] 的向量，可按分类、作者或标签过滤。
    ///
    fn article_list(
        &self,
        page: i32,
        size: i32,
        group: Option<&str>,
        tags: Vec<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<ArticleSummary>, Self::Error>>;

    /// 查询所有公开的 [`CategoryInfo`]
    ///
    /// 返回系统中所有公开分组的分类信息。
    ///
    fn groups(&self) -> impl std::future::Future<Output = Result<Vec<Group>, Self::Error>>;

    /// 查询所有文章标签
    ///
    /// 返回系统中所有公开文章的标签集合。
    ///
    fn tags(&self) -> impl std::future::Future<Output = Result<Vec<String>, sqlx::Error>>;
}

impl Querier for DBPool {
    type Error = sqlx::Error;

    async fn get_one(&self, slug: impl AsRef<str>) -> Result<Option<ArticleDetail>, Self::Error> {
        let result = sqlx::query_as::<_, ArticleDetail>(
                r#"
                SELECT a.slug, a.title, a.summary, a.tags, a.content, to_jsonb(g) as group, a.updated_at, a.created_at
                FROM articles a
                INNER JOIN groups g ON a.group_id = g.id
                WHERE a.slug = $1
                AND g.public = TRUE
                LIMIT 1
                "#,
            )
            .bind(slug.as_ref())
            .fetch_optional(self)
            .await?;
        Ok(result)
    }

    async fn article_list(
        &self,
        page: i32,
        size: i32,
        group: Option<&str>,
        tags: Vec<&str>,
    ) -> Result<Vec<ArticleSummary>, sqlx::Error> {
        let offset = (page.max(1) - 1) * size;
        let mut builder = sqlx::QueryBuilder::new(
            r#"
                SELECT a.slug, a.title, a.summary, a.tags, to_jsonb(g) as group, a.updated_at, a.created_at
                FROM articles a
                INNER JOIN groups g ON a.group_id = g.id
                "#,
        );

        builder.push("WHERE g.public = true");
        if let Some(g) = group {
            builder.push(" AND g.id = ").push_bind(g);
        }
        if !tags.is_empty() {
            builder.push(" AND a.tags && ").push_bind(tags);
        }

        builder.push(" ORDER BY a.updated_at DESC ");
        builder.push(" LIMIT ").push_bind(size);
        builder.push(" OFFSET ").push_bind(offset);

        let query = builder.build_query_as::<ArticleSummary>();
        let result = query.fetch_all(self).await?;
        Ok(result)
    }

    async fn groups(&self) -> Result<Vec<Group>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Group>(
            r#"
                SELECT *
                FROM groups
                WHERE public = true
                ORDER BY id DESC
                "#,
        )
        .fetch_all(self)
        .await?;

        Ok(rows)
    }

    async fn tags(&self) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
                SELECT DISTINCT UNNEST(a.tags) AS "tag"
                FROM articles a
                JOIN groups g ON a.group_id = g.id
                WHERE g.public = true
                ORDER BY tag
                "#,
        )
        .fetch_all(self)
        .await
    }
}
