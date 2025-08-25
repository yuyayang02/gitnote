use super::{ArticleDetail, ArticleListItem, CategoryInfo, Db};

/// Trait 用于查询文章相关数据
///
/// 提供获取文章详情、列表、分类和标签的接口。
pub trait ArticleQuery {
    /// 获取 [`Db`] 对象
    fn db(&self) -> &Db;

    /// 查询单个文章详情
    ///
    /// 返回 [`ArticleDetail`]，如果文章不存在则返回 `None`。
    ///
    /// ```ignore
    /// let db: Db = /* 初始化 Db */;
    /// let article = db.get_one("slug-example").await.unwrap();
    /// ```
    fn get_one(
        &self,
        slug: impl AsRef<str>,
    ) -> impl Future<Output = Result<Option<ArticleDetail>, sqlx::Error>> {
        async move {
            let result = sqlx::query_as::<_, ArticleDetail>(
                r#"
                SELECT a.slug, a.title, a.summary, a.tags, a.content, a.updated_at, a.created_at,
                       g.category ->> 'id' AS category_id,
                       g.category ->> 'name' AS category_name,
                       g.author ->> 'name' AS author_name
                FROM articles a
                INNER JOIN groups g ON a.group_path = g.path
                WHERE a.slug = $1
                AND g.public = TRUE
                "#,
            )
            .bind(slug.as_ref())
            .fetch_optional(self.db())
            .await?;
            Ok(result)
        }
    }

    /// 分页查询文章列表
    ///
    /// 返回 [`ArticleListItem`] 的向量，可按分类、作者或标签过滤。
    ///
    /// ```ignore
    /// let db: Db = /* 初始化 Db */;
    /// let articles = db.list(10, 1, None, None, vec!["rust".to_string()]).await.unwrap();
    /// ```
    fn list(
        &self,
        limit: i32,
        page: i32,
        category: Option<String>,
        author: Option<String>,
        tags: Vec<String>,
    ) -> impl Future<Output = Result<Vec<ArticleListItem>, sqlx::Error>> {
        async move {
            let offset = (page.max(1) - 1) * limit;
            let mut builder = sqlx::QueryBuilder::new(
                r#"
                SELECT a.slug, a.title, a.summary, a.tags, a.updated_at, a.created_at,
                       g.category ->> 'id' AS category_id,
                       g.category ->> 'name' AS category_name,
                       g.author ->> 'name' AS author_name
                FROM articles a
                INNER JOIN groups g ON a.group_path = g.path
                "#,
            );

            builder.push("WHERE g.public = TRUE");
            if let Some(cat) = category {
                builder.push(" AND g.category->>'id' = ").push_bind(cat);
            }
            if let Some(auth) = author {
                builder.push(" AND g.author->>'name' = ").push_bind(auth);
            }
            if !tags.is_empty() {
                builder.push(" AND a.tags && ").push_bind(tags);
            }

            builder.push(" ORDER BY a.updated_at DESC ");
            builder.push(" LIMIT ").push_bind(limit);
            builder.push(" OFFSET ").push_bind(offset);

            let query = builder.build_query_as::<ArticleListItem>();
            let result = query.fetch_all(self.db()).await?;
            Ok(result)
        }
    }

    /// 查询所有公开的 [`CategoryInfo`]
    ///
    /// 返回系统中所有公开分组的分类信息。
    ///
    /// ```ignore
    /// let db: Db = /* 初始化 Db */;
    /// let categories = db.categories().await.unwrap();
    /// ```
    fn categories(&self) -> impl Future<Output = Result<Vec<CategoryInfo>, sqlx::Error>> + '_ {
        async move {
            let rows = sqlx::query_as::<_, CategoryInfo>(
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
            .fetch_all(self.db())
            .await?;

            Ok(rows)
        }
    }

    /// 查询所有文章标签
    ///
    /// 返回系统中所有公开文章的标签集合。
    ///
    /// ```ignore
    /// let db: Db = /* 初始化 Db */;
    /// let tags = db.tags().await.unwrap();
    /// ```
    fn tags(&self) -> impl Future<Output = Result<Vec<String>, sqlx::Error>> + '_ {
        async move {
            Ok(sqlx::query_scalar(
                r#"
                SELECT DISTINCT UNNEST(a.tags) AS "tag"
                FROM articles a
                JOIN groups g ON a.group_path = g.path
                WHERE g.public = true
                ORDER BY tag
                "#,
            )
            .fetch_all(self.db())
            .await?)
        }
    }
}

impl ArticleQuery for &Db {
    fn db(&self) -> &Db {
        &self
    }
}
