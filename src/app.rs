use std::{path::Path, sync::Arc};

use crate::{render::GithubAPiRenderer, storage::Db};

/// 应用程序上下文
///
/// [`App`] 封装了数据库连接池、Git 渲染器和裸仓库引用，提供统一访问入口。
#[derive(Clone)]
pub struct App {
    db: Arc<Db>,
    repo_path: Arc<Path>,
    renderer: GithubAPiRenderer,
}

impl App {
    /// 创建一个新的 [`App`] 实例
    pub fn new(db: Db, renderer: GithubAPiRenderer, repo_path: impl AsRef<Path>) -> App {
        let repo_path = Arc::<Path>::from(repo_path.as_ref());

        Self {
            db: Arc::new(db),
            repo_path,
            renderer,
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

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
}
