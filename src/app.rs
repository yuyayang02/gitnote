use std::sync::Arc;

use crate::{git::GitBareRepository, render::GithubAPiRenderer, storage::Db};

/// 应用程序上下文
///
/// [`App`] 封装了数据库连接池、Git 渲染器和裸仓库引用，提供统一访问入口。
#[derive(Clone)]
pub struct App {
    db: Arc<Db>,
    renderer: GithubAPiRenderer,
    git: Arc<GitBareRepository>,
}

impl App {
    /// 创建一个新的 [`App`] 实例
    ///
    /// ```ignore
    /// # let db: Db = todo!();
    /// # let renderer: GithubAPiRenderer = todo!();
    /// # let repo: GitBareRepository = todo!();
    /// let app = App::new(db, renderer, repo);
    /// ```
    pub fn new(db: Db, renderer: GithubAPiRenderer, repo: GitBareRepository) -> App {
        Self {
            db: Arc::new(db),
            renderer,
            git: Arc::new(repo),
        }
    }

    /// 获取数据库连接池
    ///
    /// 返回 [`Db`] 的引用。
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// 获取 Markdown 渲染器
    ///
    /// 返回 [`GithubAPiRenderer`] 的引用。
    pub fn renderer(&self) -> &GithubAPiRenderer {
        &self.renderer
    }

    /// 获取裸 Git 仓库引用
    ///
    /// 返回 [`GitBareRepository`] 的引用。
    pub fn repo(&self) -> &GitBareRepository {
        &self.git
    }
}
