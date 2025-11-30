use std::{path::Path, sync::Arc};

use axum::extract::FromRef;

use crate::{
    render::GithubAPiRenderer,
    storage::{DBPool, SqlxStore},
};

/// 应用程序上下文
///
/// [`AppState`] 封装了数据库连接池、Git 渲染器和裸仓库引用，提供统一访问入口。
#[derive(Clone, FromRef)]
pub struct AppState {
    pool: DBPool,
    repo_path: Arc<Path>,
    renderer: GithubAPiRenderer,
}

impl AppState {
    /// 创建一个新的 [`App`] 实例
    pub fn new(pool: DBPool, renderer: GithubAPiRenderer, repo_path: impl AsRef<Path>) -> Self {
        let repo_path = Arc::<Path>::from(repo_path.as_ref());

        Self {
            repo_path,
            renderer,
            pool,
        }
    }

    /// 获取仓储对象
    pub fn storage(&self) -> SqlxStore {
        SqlxStore::new(self.pool.clone())
    }

    /// 获取查询对象
    pub fn querier(&self) -> &DBPool {
        &self.pool
    }

    /// 获取 Markdown 渲染器
    pub fn renderer(&self) -> &GithubAPiRenderer {
        &self.renderer
    }

    /// 获取仓库路径
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
}
