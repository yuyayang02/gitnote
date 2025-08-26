use std::sync::{Arc, Mutex};

use git2::{Oid, Repository, Sort};

use crate::git::IntoRepoEntry;

use super::{GitCommand, GitError, RepoEntry};
/// 提供对 Git 仓库的常用操作。
///
/// 实现 [`GitOps`] 可以方便地执行 commit 差异分析、远程更新和文件读取等操作。
pub trait GitOps: Send {
    /// 更新远程仓库。
    fn remote_update(&self) -> Result<(), GitError>;

    /// 按提交顺序遍历两个提交之间的差异，返回对应的 [`RepoEntry`] 列表。
    ///
    /// 如果指定了 `old`，则计算从该 commit 到 `new` 的差异；否则返回从仓库初始提交到 `new` 的差异。
    fn diff_commits_range(&self, old: Option<&str>, new: &str) -> Result<Vec<RepoEntry>, GitError>;

    /// 读取指定 blob 内容为 UTF-8 字符串，解析失败返回 [`None`]。
    fn read_blob(&self, blob_id: &str) -> Option<String>;
}

impl GitOps for Repository {
    /// 按提交顺序遍历两个 commit 之间的差异，并生成 [`RepoEntry`] 列表。
    ///
    /// 流程：
    /// 1. 解析 commit ID 为 Oid
    /// 2. 创建 revwalk，按拓扑顺序从新 commit 向旧 commit 遍历
    /// 3. 对每个 commit 生成相对于前一个 tree 的差异
    /// 4. 将差异转换为 [`RepoEntry`] 列表返回
    fn diff_commits_range(&self, old: Option<&str>, new: &str) -> Result<Vec<RepoEntry>, GitError> {
        let old_oid = old.map(Oid::from_str).transpose()?;
        let new_oid = Oid::from_str(new)?;

        let mut revwalk = self.revwalk()?;
        revwalk.push(new_oid)?;
        if let Some(old_oid) = old_oid {
            revwalk.hide(old_oid)?;
        }
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;

        let mut prev_tree_opt = if let Some(oid) = old_oid {
            let commit = self.find_commit(oid)?;
            Some(commit.tree()?)
        } else {
            None
        };

        let iter = revwalk
            .filter_map(|oid_res| {
                let oid = oid_res.ok()?;
                let commit = self.find_commit(oid).ok()?;
                let tree = commit.tree().ok()?;
                let diff = self
                    .diff_tree_to_tree(prev_tree_opt.as_ref(), Some(&tree), None)
                    .ok()?;
                prev_tree_opt = Some(tree);
                Some((diff, commit))
            })
            .flat_map(IntoRepoEntry::into_entry)
            .collect::<Vec<_>>();

        Ok(iter)
    }

    /// 更新远程仓库。
    fn remote_update(&self) -> Result<(), GitError> {
        GitCommand::remote_update(self.path())?;
        Ok(())
    }

    /// 读取 blob 内容为 UTF-8 字符串，解析失败返回 [`None`]。
    fn read_blob(&self, oid: &str) -> Option<String> {
        let blob = self.find_blob(Oid::from_str(oid).ok()?).ok()?;
        std::str::from_utf8(blob.content())
            .ok()
            .map(|s| s.to_string())
    }
}

/// 异步访问的仓库封装。
///
/// 内部使用 `Arc<Mutex<Repository>>` 保证线程安全。
pub struct AsyncRepository {
    inner: Arc<Mutex<Repository>>,
}

impl AsyncRepository {
    /// 构造新的异步仓库封装。
    pub fn new(repo: Repository) -> Self {
        Self {
            inner: Arc::new(Mutex::new(repo)),
        }
    }
}

impl GitOps for AsyncRepository {
    fn read_blob(&self, oid: &str) -> Option<String> {
        self.inner.lock().unwrap().read_blob(oid)
    }

    fn remote_update(&self) -> Result<(), GitError> {
        self.inner.lock().unwrap().remote_update()
    }

    fn diff_commits_range(&self, old: Option<&str>, new: &str) -> Result<Vec<RepoEntry>, GitError> {
        self.inner.lock().unwrap().diff_commits_range(old, new)
    }
}
