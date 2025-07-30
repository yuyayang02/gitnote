mod archive_tagger;
mod archiver;
mod diff_util;
mod entry;

use std::path::Path;

pub use archive_tagger::ArchiveTagger;
pub use entry::RepoEntry;
use git2::{Oid, Repository};

use crate::{error::Result, repo::diff_util::DiffUtil};

pub(self) const GITNOTE_FILENAME: &'static str = ".gitnote.toml";

/// 表示裸仓库（bare Git repository）的路径封装。
#[derive(Debug, Clone)]
pub struct GitBareRepository(String);

impl GitBareRepository {
    /// 创建一个新的 GitBareRepository 实例，传入路径。
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    fn repo(&self) -> Result<Repository> {
        Ok(git2::Repository::open_bare(&self.0)?)
    }

    pub fn diff_commit(
        &self,
        old_commit_str: impl AsRef<str>,
        new_commit_str: impl AsRef<str>,
    ) -> Result<Vec<RepoEntry>> {
        let repo = self.repo()?;
        DiffUtil::new(&repo).diff_commits_from_str(old_commit_str, new_commit_str)
    }

    pub fn diff_all(&self) -> Result<Vec<RepoEntry>> {
        let repo = self.repo()?;
        DiffUtil::new(&repo).diff_all()
    }

    pub fn archive(
        &self,
        tag_str: impl AsRef<str>,
        tag_commit_str: impl AsRef<str>,
    ) -> Result<archiver::ArchivedInfo> {
        let repo = self.repo()?;
        archiver::Archiver::new(&repo).archive(tag_str.as_ref(), tag_commit_str.as_ref())
    }
}

/// 从 blob 对象读取 UTF-8 内容（如文件内容）。
pub trait ReadBlob {
    fn read_blob(repo: &Repository, oid: Oid) -> Option<String> {
        repo.find_blob(oid).ok().and_then(|blob| {
            std::str::from_utf8(blob.content())
                .ok()
                .map(|s| s.to_string())
        })
    }
}

pub(self) fn is_md(p: &Path) -> bool {
    matches!(
        p.extension().and_then(|e| e.to_str()),
        Some("md") | Some("markdown")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_REPO_PATH: &str = "gitnote.git";
    const ARCHIVE_TAG: &str = "2025-Q1";
    const ARCHIVE_COMMIT: &str = "26888d4240776ddfdaf96168047d76791967bc1e";

    fn open_repo() -> GitBareRepository {
        GitBareRepository::new(TEST_REPO_PATH)
    }

    #[test]
    fn test_archive_creates_archive_branch() {
        let repo = open_repo();

        // 执行归档操作
        let info = repo
            .archive(ARCHIVE_TAG, ARCHIVE_COMMIT)
            .expect("archive failed");

        println!("{}", info.summary());
    }

    #[test]
    fn test_commit_diff_extracts_entries() {
        let _repo = open_repo();
        let repo = _repo.repo().unwrap();
        // 获取当前 HEAD 和其父提交
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let parent = head.parent(0).expect("HEAD has no parent");

        println!("Diff range: {} → {}", parent.id(), head.id());

        let entries = _repo
            .diff_commit(parent.id().to_string(), head.id().to_string())
            .expect("Failed to diff commits");

        for entry in &entries {
            println!("{}", entry);
        }

        assert!(!entries.is_empty(), "Expected at least one change in diff");
    }

    #[test]
    fn test_rebuild_all_generates_entry_list() {
        let repo = open_repo();

        let entries = repo.diff_all().expect("Failed to rebuild");

        for entry in &entries {
            println!("{}", entry);
        }

        assert!(
            !entries.is_empty(),
            "Expected rebuild_all to return entries"
        );
    }
}
