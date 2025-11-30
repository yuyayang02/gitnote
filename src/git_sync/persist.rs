use crate::{
    content::{ArticleBuilder, Group, Renderer},
    git_client::{ChangeKind, FileKind, GitClient, GitFileEntry},
    storage::Store,
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
/// 提供一个 [`GitFileEntryPersist::persist`] 方法，将条目持久化到数据库或存储中
pub trait Persistable {
    type Error;

    /// 持久化条目
    ///
    fn persist<R, S>(
        &self,
        storage: S,
        renderer: &R,
        repo: &GitClient,
        mode: PersistMode,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>>
    where
        R: Renderer,
        S: Store,
        S::Owned: Store;
}

impl Persistable for Vec<GitFileEntry> {
    type Error = crate::error::Error;

    /// 将多个 [`GitFileEntry`] 持久化到数据库
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
    async fn persist<R, S>(
        &self,
        mut storage: S,
        renderer: &R,
        repo: &GitClient,
        mode: PersistMode,
    ) -> Result<(), Self::Error>
    where
        R: Renderer,
        S: Store,
        S::Owned: Store,
    {
        if let PersistMode::ResetAll = mode {
            storage.clean();
        };

        for entry in self {
            match (entry.file_kind(), entry.change_kind()) {
                (FileKind::Group, ChangeKind::Added | ChangeKind::Modified) => {
                    let content = repo.load_file(entry.id())?;
                    let group = Group::new(entry.path(), content)?;
                    storage.upsert_group(&group);
                }

                (FileKind::Group, ChangeKind::Deleted) => {
                    let group = Group::empty(entry.path());
                    storage.remove_group(&group);
                }

                (FileKind::Markdown, ChangeKind::Added | ChangeKind::Modified) => {
                    let content = repo.load_file(entry.id())?;
                    let article = ArticleBuilder::new(entry.path())
                        .content(content)
                        .build_with_renderer(renderer)
                        .await?;
                    storage.upsert_article(&article);
                }

                (FileKind::Markdown, ChangeKind::Deleted) => {
                    let article_builder = ArticleBuilder::new(entry.path());
                    storage.remove_article(article_builder.to_ref());
                }

                (FileKind::Other, _) => (),
            }
        }

        storage.commit().await?;
        Ok(())
    }
}
