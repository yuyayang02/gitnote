use std::path::Path;

use git2::Repository;

use super::{AsyncGitClient, GitError, GitFileEntry, GitOperation};

/// 内部持有实现了 [`GitOps`] 的实例，用于执行 Git 操作。
#[derive(Debug)]
pub struct GitClient<R: GitOperation>(R);

impl GitClient<AsyncGitClient> {
    /// 打开一个裸仓库并返回 [`GitRepository`] 实例。
    ///
    /// 仓库路径基于传入的名称
    pub fn open(repo_name: impl AsRef<Path>) -> Result<Self, GitError> {
        let repo = Repository::open_bare(repo_name)?;
        Ok(Self(AsyncGitClient::new(repo)))
    }
}

impl<R: GitOperation> GitClient<R> {
    /// 获取内部仓库实例。
    fn repo(&self) -> &R {
        &self.0
    }

    /// 获取指定 commit 的快照。
    ///
    /// 返回对应的 [`GitFileEntry`] 列表，用于查看当前 commit 的文件状态。
    pub fn snapshot(&self, commit_str: &str) -> Result<Vec<GitFileEntry>, GitError> {
        let repo = self.repo();
        let entries = repo.diff_commits_range(R::EMPTY_TREE_OID, commit_str)?;
        Ok(entries)
    }

    /// 比较两个 commit 之间的差异。
    ///
    /// 返回 [`GitFileEntry`] 列表，表示变更的文件和内容。
    pub fn diff_commits(
        &self,
        old_commit_str: &str,
        new_commit_str: &str,
    ) -> Result<Vec<GitFileEntry>, GitError> {
        let repo = self.repo();
        let entries = repo.diff_commits_range(old_commit_str, new_commit_str)?;
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
    use crate::git_client::AsSummary;

    use super::*;

    const LAST_COMMIT_OID: &str = "4db775450dee399c328935eb03fd4fcc6c60e333";

    #[test]
    fn test_diff_commits() {
        let repo = GitClient::open(crate::REPO_PATH).unwrap();
        let entries = repo.snapshot(LAST_COMMIT_OID).unwrap();

        let summary = entries.as_summary();

        dbg!(&summary);
        let mut lines = summary.lines();

        assert!(lines.next().unwrap().starts_with("[md]"));
        assert!(lines.next().unwrap().starts_with("[group]"));
        assert!(lines.next().is_none());
    }
}
