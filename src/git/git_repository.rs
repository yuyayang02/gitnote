use std::{collections::BTreeMap, path::Path, time::Duration};

use chrono::{DateTime, Local};
use git2::{Oid, Repository};

use super::{AsSummary, GitRepository, IntoEntry, RepoEntry};
use crate::error::{Error, Result};

/// 封装裸仓库（bare Git repository）的路径。
#[derive(Debug, Clone)]
pub struct GitBareRepository(String);

/// 已打开的裸仓库。
///
/// 包含具体的 [`GitRepository`] 实例。
pub struct OpenedGitBareRepository<R: GitRepository> {
    repo: R,
}

impl GitBareRepository {
    /// 构建 [`GitBareRepository`]，保存裸仓库路径。
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// 打开裸仓库，返回 [`OpenedGitBareRepository`]。
    pub fn open(&self) -> Result<OpenedGitBareRepository<Repository>> {
        Ok(OpenedGitBareRepository { repo: self.repo()? })
    }

    /// 打开底层 [`Repository`]。
    pub fn repo(&self) -> Result<Repository> {
        Ok(git2::Repository::open_bare(&self.0)?)
    }
}

/// 归档操作的结果信息。
pub struct ArchivedInfo {
    /// 创建的归档分支名称，例如 `"refs/heads/archive"`。
    pub branch: String,

    /// 执行归档操作的本地时间戳。
    pub datetime: DateTime<Local>,

    /// 归档操作耗时。
    pub duration: Duration,

    /// 被归档的所有文件（包括普通文件与 GitNote）。
    pub repo_entries: Vec<RepoEntry>,
}

impl ArchivedInfo {
    /// 生成归档结果的概要文本。
    pub fn as_summary(&self) -> String {
        let header = format!(
            "Archive branch: {}\nDate: {}\nDuration: {:?}\nEntries:",
            self.branch,
            self.datetime.format("%Y-%m-%d %H:%M:%S"),
            self.duration
        );

        let entries_summary = self.repo_entries.as_summary();

        format!("{}\n{}", header, entries_summary)
    }
}

impl<R: GitRepository> OpenedGitBareRepository<R> {
    /// 计算两个 commit 之间的差异，返回变更的文件条目。
    pub fn diff_commits(
        &self,
        old_commit_str: &str,
        new_commit_str: &str,
    ) -> Result<Vec<RepoEntry>> {
        let old_commit = self.repo.resolve_commit(old_commit_str)?;
        let new_commit = self.repo.resolve_commit(new_commit_str)?;

        let entries = self
            .repo
            .diff_commits_stream(Some(&old_commit), &new_commit)?
            .flat_map(IntoEntry::into_entry)
            .collect::<Vec<_>>();

        Ok(entries)
    }

    /// 从初始 commit 到当前 HEAD 的所有差异。
    pub fn diff_all(&self) -> Result<Vec<RepoEntry>> {
        let commit = self.repo.head_commit()?;

        let entries = self
            .repo
            .diff_commits_stream(None, &commit)?
            .flat_map(IntoEntry::into_entry)
            .collect::<Vec<_>>();

        Ok(entries)
    }

    /// 获取完整差异，包含归档分支与主分支的变更。
    ///
    /// - `archive` 分支的差异  
    /// - 从 `cmd/archive` 标签到当前 HEAD 的差异
    pub fn diff_all_with_archive(&self) -> Result<Vec<RepoEntry>> {
        const ARCHIVE_BRANCH: &str = "archive";
        const ARCHIVE_TAG: &str = "cmd/archive";

        let archive_head_commit = self.repo.branch_head_commit(ARCHIVE_BRANCH)?;
        let mut archive_entries = self
            .repo
            .diff_commits_stream(None, &archive_head_commit)?
            .flat_map(IntoEntry::into_entry)
            .collect::<Vec<_>>();

        let start_commit = self.repo.tag_commit(ARCHIVE_TAG)?;
        let end_commit = self.repo.head_commit()?;

        let main_entries = self
            .repo
            .diff_commits_stream(Some(&start_commit), &end_commit)?
            .flat_map(IntoEntry::into_entry)
            .collect::<Vec<_>>();

        archive_entries.extend(main_entries);

        Ok(archive_entries)
    }

    /// 加载指定 blob 的内容为 [`String`]。
    ///
    /// 如果 blob 不存在或不是 UTF-8，返回 [`Error::Custom`]。
    pub fn load_file(&self, file_id: impl AsRef<str>) -> Result<String> {
        self.repo
            .read_blob(file_id.as_ref())
            .ok_or(Error::Custom("file not found"))
    }

    /// 按时间戳分组文件条目。
    pub fn group_by_timestamp<'a>(
        entries: &'a [RepoEntry],
    ) -> BTreeMap<DateTime<Local>, Vec<&'a Path>> {
        let mut grouped = BTreeMap::new();

        for entry in entries {
            grouped
                .entry(entry.timestamp())
                .or_insert_with(Vec::new)
                .push(entry.path());
        }

        grouped
    }

    const ARCHIVE_BRANCH: &str = "archive";

    /// 将指定 commit 的文件归档到独立分支。
    ///
    /// 流程：
    /// 1. 解析目标 commit  
    /// 2. 获取从初始到目标 commit 的变更  
    /// 3. 按时间戳分组  
    /// 4. 在临时工作树中重放变更并生成 commit 链  
    /// 5. 创建归档分支  
    /// 6. 清理临时工作树  
    /// 7. 返回 [`ArchivedInfo`]
    pub fn archive(&self, tag_commit_str: impl AsRef<str>) -> Result<ArchivedInfo> {
        let start = std::time::Instant::now();

        // 解析目标 commit
        let tag_commit = self.repo.resolve_commit(tag_commit_str.as_ref())?;

        // 获取从初始到目标 commit 的精简变更列表
        let entries = self
            .repo
            .diff_commits_stream(None, &tag_commit)?
            .flat_map(IntoEntry::into_entry)
            .collect::<Vec<_>>();

        // 归档分组
        let grouped = Self::group_by_timestamp(&entries);

        // 创建临时工作树
        let tempdir = tempfile::tempdir()?;
        let (worktree, worktree_repo) = self
            .repo
            .create_worktree(tempdir.path(), Self::ARCHIVE_BRANCH)?;

        // 构建 commit 链
        let mut parent_oid: Option<Oid> = None;
        for (dt, paths) in grouped {
            let msg = format!("archive: archived at {}", dt.format("%Y-%m-%d %H:%M:%S"));
            parent_oid = Some(worktree_repo.commit_with_paths(parent_oid, &paths, dt, &msg)?);
        }
        let final_oid = parent_oid.expect("至少应有一次提交");

        // 创建归档分支
        worktree_repo.create_reference(Self::ARCHIVE_BRANCH, final_oid)?;

        // 清理临时工作树
        self.repo.prune_worktree(worktree)?;

        // 返回归档信息
        let duration = start.elapsed();
        Ok(ArchivedInfo {
            branch: Self::ARCHIVE_BRANCH.to_string(),
            datetime: Local::now(),
            duration,
            repo_entries: entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone as _};
    use std::path::PathBuf;

    #[test]
    fn test_group_by_timestamp() {
        // 构造测试数据
        let ts1 = Local.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();
        let ts2 = Local.with_ymd_and_hms(2023, 1, 1, 11, 0, 0).unwrap();
        let ts3 = Local.with_ymd_and_hms(2023, 1, 2, 11, 0, 0).unwrap();

        let entries = vec![
            RepoEntry {
                id: String::new(),
                path: PathBuf::from(""),
                change_kind: crate::git::ChangeKind::Added,
                file_kind: crate::git::FileKind::Other,
                timestamp: ts1,
            },
            RepoEntry {
                id: String::new(),
                path: PathBuf::from(""),
                change_kind: crate::git::ChangeKind::Added,
                file_kind: crate::git::FileKind::GitNote,
                timestamp: ts1,
            },
            RepoEntry {
                id: String::new(),
                path: PathBuf::from(""),
                change_kind: crate::git::ChangeKind::Added,
                file_kind: crate::git::FileKind::Other,
                timestamp: ts2,
            },
            RepoEntry {
                id: String::new(),
                path: PathBuf::from(""),
                change_kind: crate::git::ChangeKind::Added,
                file_kind: crate::git::FileKind::Markdown,
                timestamp: ts3,
            },
        ];

        // 手动调用 group_by_timestamp
        let grouped = OpenedGitBareRepository::<Repository>::group_by_timestamp(&entries);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[&ts1].len(), 2);
        assert_eq!(grouped[&ts2].len(), 1);
    }

    #[test]
    fn test_archived_info_as_summary() {
        use chrono::TimeZone;

        let info = ArchivedInfo {
            branch: "refs/heads/archive".to_string(),
            datetime: Local.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap(),
            duration: Duration::from_secs(42),
            repo_entries: vec![], // 这里用空，as_summary 内部会调用 as_summary()
        };

        let summary = info.as_summary();
        assert!(summary.contains("Archive branch: refs/heads/archive"));
        assert!(summary.contains("2023-01-01 12:00:00"));
        assert!(summary.contains("Duration: 42s"));
    }
}
