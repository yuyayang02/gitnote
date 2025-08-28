use std::path::Path;

use git2::Repository;

use crate::git::RepoDir;

use super::{AsyncRepository, GitError, GitOps, RepoEntry};

/// 内部持有实现了 [`GitOps`] 的实例，用于执行 Git 操作。
#[derive(Debug)]
pub struct GitRepository<R: GitOps>(R);

impl GitRepository<AsyncRepository> {
    /// 打开一个裸仓库并返回 [`GitRepository`] 实例。
    ///
    /// 仓库路径基于传入的名称
    pub fn open(repo_name: impl AsRef<Path>) -> Result<Self, GitError> {
        let repo_path = RepoDir::path(repo_name);

        let repo = Repository::open_bare(repo_path)?;
        Ok(Self(AsyncRepository::new(repo)))
    }
}

impl<R: GitOps> GitRepository<R> {
    /// 更新远程仓库，并返回自身以支持链式调用。
    pub fn fetch(self) -> Result<Self, GitError> {
        self.0.remote_update()?;
        Ok(self)
    }

    /// 获取内部仓库实例。
    fn repo(&self) -> &R {
        &self.0
    }

    /// 获取指定 commit 的快照。
    ///
    /// 返回对应的 [`RepoEntry`] 列表，用于查看当前 commit 的文件状态。
    pub fn snapshot(&self, commit_str: &str) -> Result<Vec<RepoEntry>, GitError> {
        let repo = self.repo();
        let entries = repo.diff_commits_range(R::EMPTY_TREE_OID, commit_str)?;
        Ok(entries)
    }

    /// 比较两个 commit 之间的差异。
    ///
    /// 返回 [`RepoEntry`] 列表，表示变更的文件和内容。
    pub fn diff_commits(
        &self,
        old_commit_str: &str,
        new_commit_str: &str,
    ) -> Result<Vec<RepoEntry>, GitError> {
        let repo = self.repo();
        let entries = repo.diff_commits_range(&old_commit_str, &new_commit_str)?;
        Ok(entries)
    }

    /// 读取指定文件的内容。
    ///
    /// 返回 UTF-8 字符串，文件不存在时返回 [`GitError::NotFound`]。
    pub fn load_file(&self, file_id: impl AsRef<str>) -> Result<String, GitError> {
        self.repo()
            .read_blob(file_id.as_ref())
            .ok_or(GitError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use crate::git::AsSummary;

    use super::*;

    #[test]
    #[ignore = "需要有效的git仓库"]
    fn test_diff_commits() {
        let repo = GitRepository::open("gitnote").unwrap();
        let entries = repo
            .snapshot("84779e0a9461e9130b5ad09241349b2c9a9619a3")
            .unwrap();
        println!("{}", entries.as_summary());
    }
}
