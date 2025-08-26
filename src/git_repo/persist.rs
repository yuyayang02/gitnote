use crate::{
    app::App,
    content::{ArticleBuilder, Group},
    git::{ChangeKind, FileKind, GitRepository, RepoEntry},
    storage::ArticleStorage,
};
/// 持久化模式
///
/// - [`PersistMode::ResetAll`]：重置所有数据，然后再写入
/// - [`PersistMode::Incremental`]：增量更新，只处理变化部分
pub enum PersistMode {
    ResetAll,
    Incremental,
}

/// 定义可持久化的条目接口
///
/// 提供一个 [`RepoEntryPersist::persist`] 方法，将条目持久化到数据库或存储中
pub trait RepoEntryPersist {
    type Error;

    /// 持久化条目
    ///
    fn persist(
        &self,
        app: App,
        repo: &GitRepository,
        mode: PersistMode,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>>;
}

impl RepoEntryPersist for Vec<RepoEntry> {
    type Error = crate::error::Error;

    /// 将多个 [`RepoEntry`] 持久化到数据库
    ///
    /// 根据 [`PersistMode`] 决定是重置全部还是增量更新。
    ///
    /// 处理逻辑：
    /// - GitNote 文件：
    ///     - Added：加载内容，构建 [`Group`]，更新数据库
    ///     - Deleted：构建空 [`Group`]，从数据库删除
    /// - Markdown 文件：
    ///     - Added：加载内容，构建 [`ArticleBuilder`]，使用 [`App::renderer`] 渲染后写入数据库
    ///     - Deleted：从数据库删除
    /// - Other 文件类型：忽略
    ///
    async fn persist(
        &self,
        app: App,
        repo: &GitRepository,
        mode: PersistMode,
    ) -> Result<(), Self::Error> {
        let mut tx = app.db().begin().await?;

        if let PersistMode::ResetAll = mode {
            tx.reset_all().await?;
        };

        for entry in self {
            match (entry.file_kind(), entry.change_kind()) {
                (FileKind::Group, ChangeKind::Added) => {
                    let content = repo.load_file(entry.id())?;
                    let group = Group::new(entry.path(), content)?;
                    tx.update_group(&group).await?;
                }

                (FileKind::Group, ChangeKind::Deleted) => {
                    let group = Group::empty(entry.path());
                    tx.remove_group(&group).await?;
                }

                (FileKind::Markdown, ChangeKind::Added) => {
                    let content = repo.load_file(entry.id())?;
                    let article = ArticleBuilder::new(entry.path(), entry.timestamp())
                        .content(content)
                        .build_with_renderer(app.renderer())
                        .await?;
                    tx.upsert(&article).await?;
                }

                (FileKind::Markdown, ChangeKind::Deleted) => {
                    let article_builder = ArticleBuilder::new(entry.path(), entry.timestamp());
                    tx.remove(&article_builder).await?;
                }

                (FileKind::Other, _) => (),
            }
        }

        tx.commit().await?;
        Ok(())
    }
}
