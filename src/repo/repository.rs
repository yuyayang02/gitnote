use std::path::Path;

use chrono::{Local, TimeZone};
use git2::{Commit, Delta, Oid, Repository};

use crate::error::Result;

use super::entry::RepoEntry;

pub struct GitBareRepository(String);

pub struct OpenGitBareRepository(Repository);

impl OpenGitBareRepository {
    /// `.gitnote.toml` 配置文件的标准文件名
    const GITNOTE_FILENAME: &'static str = ".gitnote.toml";

    /// 内部封装的 Git 仓库对象引用（只读）
    #[inline]
    fn repo(&self) -> &Repository {
        &self.0
    }

    /// 通过 Oid 读取 Blob 内容并以 UTF-8 字符串返回（若失败则返回 None）
    fn read_blob(&self, oid: Oid) -> Option<String> {
        let blob = self.repo().find_blob(oid).ok()?;

        std::str::from_utf8(blob.content())
            .ok()
            .map(|s| s.to_string())
    }

    /// 计算两个提交之间的差异，生成语义化的数据流（RepoEntry）
    /// - `parent_commit`: 旧提交；若为 None 表示首次提交（空树）
    /// - `commit`: 新提交
    /// 返回值中包括 `.gitnote.toml` 和 `.md` 文件的新增/删除/变更
    fn compute_commit_diff(
        &self,
        old_commit: Option<&Commit>,
        new_commit: &Commit,
    ) -> Result<Vec<RepoEntry>> {
        let tree = new_commit.tree()?; // 新提交的文件快照（Tree）
        let parent_tree: Option<git2::Tree<'_>> = old_commit.map(|c| c.tree()).transpose()?; // 旧提交的 Tree（如有）

        let mut entries = Vec::new();

        // Git 树之间差异（如 git diff），支持 old_tree=None => 等价于空树
        let diff = self
            .repo()
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

        // 遍历差异文件列表（Delta 表示单个文件的变化记录）
        diff.foreach(
            &mut |delta, _| {
                let old_file = delta.old_file();
                let new_file = delta.new_file();

                match delta.status() {
                    // 新增文件：检测是 GitNote 还是文章
                    Delta::Added => {
                        if let Some(path) = new_file.path() {
                            if path.ends_with(Self::GITNOTE_FILENAME) {
                                if let Some(content) = self.read_blob(new_file.id()) {
                                    entries.push(RepoEntry::GitNote {
                                        group: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                                        content,
                                    });
                                }
                            } else if is_markdown_file(path) {
                                if let Some(content) = self.read_blob(new_file.id()) {
                                    entries.push(RepoEntry::File {
                                        group: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                                        name: path
                                            .file_name()
                                            .unwrap()
                                            .to_string_lossy()
                                            .to_string(),
                                        datetime: Local
                                            .timestamp_opt(new_commit.time().seconds(), 0)
                                            .unwrap(), // 用提交时间作为文章时间戳
                                        content,
                                    });
                                }
                            }
                        }
                    }

                    // 删除文件：生成 RemoveGitNote 或 RemoveFile
                    Delta::Deleted => {
                        if let Some(path) = old_file.path() {
                            if path.ends_with(Self::GITNOTE_FILENAME) {
                                entries.push(RepoEntry::RemoveGitNote {
                                    group: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                                });
                            } else if is_markdown_file(path) {
                                entries.push(RepoEntry::RemoveFile {
                                    group: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                                    name: path.file_name().unwrap().to_string_lossy().to_string(),
                                });
                            }
                        }
                    }

                    // 修改、重命名、复制：先删后增（保守策略）
                    Delta::Modified | Delta::Renamed | Delta::Copied => {
                        // 删除旧文件
                        if let Some(path) = old_file.path() {
                            if path.ends_with(Self::GITNOTE_FILENAME) {
                                entries.push(RepoEntry::RemoveGitNote {
                                    group: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                                });
                            } else if is_markdown_file(path) {
                                entries.push(RepoEntry::RemoveFile {
                                    group: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                                    name: path.file_name().unwrap().to_string_lossy().to_string(),
                                });
                            }
                        }

                        // 添加新文件
                        if let Some(path) = new_file.path() {
                            if path.ends_with(Self::GITNOTE_FILENAME) {
                                if let Some(content) = self.read_blob(new_file.id()) {
                                    entries.push(RepoEntry::GitNote {
                                        group: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                                        content,
                                    });
                                }
                            } else if is_markdown_file(path) {
                                if let Some(content) = self.read_blob(new_file.id()) {
                                    entries.push(RepoEntry::File {
                                        group: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                                        name: path
                                            .file_name()
                                            .unwrap()
                                            .to_string_lossy()
                                            .to_string(),
                                        datetime: Local
                                            .timestamp_opt(new_commit.time().seconds(), 0)
                                            .unwrap(),
                                        content,
                                    });
                                }
                            }
                        }
                    }

                    // 其他状态忽略（如未变）
                    _ => {}
                }

                true // 继续遍历下一个文件 delta
            },
            None,
            None,
            None, // 仅使用文件级遍历（无差异内容比较）
        )?;

        Ok(entries)
    }

    pub fn diff_commits(
        &self,
        old_commit: Option<&Commit>,
        new_commit: &Commit,
    ) -> Result<Vec<RepoEntry>> {
        let repo = self.repo();

        // 获取旧提交的 Oid，如果有的话
        let old_oid = old_commit.map(|c| c.id());

        let new_oid = new_commit.id();

        let mut revwalk = repo.revwalk()?;
        revwalk.push(new_oid)?; // 包含 new_commit

        if let Some(old_oid) = old_oid {
            revwalk.hide(old_oid)?; // 排除旧提交及其祖先
        }

        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)?; // 正序

        let mut prev_commit = old_commit.cloned();

        let entries = revwalk
            .filter_map(|oid_result| {
                let oid = oid_result.ok()?;
                let commit = self.repo().find_commit(oid).ok()?;

                let diff = self
                    .compute_commit_diff(prev_commit.as_ref(), &commit)
                    .ok()?;
                prev_commit = Some(commit);

                Some(diff)
            })
            .flatten()
            .collect();

        Ok(entries)
    }

    pub fn diff_commits_from_str(
        &self,
        old_commit_str: impl AsRef<str>,
        new_commit_str: impl AsRef<str>,
    ) -> Result<Vec<RepoEntry>> {
        let repo = self.repo();

        let old_commit = repo
            .find_commit(Oid::from_str(old_commit_str.as_ref())?)
            .ok();
        let new_commit = repo.find_commit(Oid::from_str(new_commit_str.as_ref())?)?;

        self.diff_commits(old_commit.as_ref(), &new_commit)
    }

    /// Rebuild 整个历史（从最早 commit 开始重放所有变更）
    /// 返回按时间顺序的所有语义化 RepoEntry 流
    pub fn rebuild_all(&self) -> Result<Vec<RepoEntry>> {
        let mut revwalk = self.repo().revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)?;

        let mut prev_commit: Option<Commit> = None;

        let entries = revwalk
            .filter_map(|oid_result| {
                let oid = oid_result.ok()?;
                let commit = self.repo().find_commit(oid).ok()?;

                let diff = self
                    .compute_commit_diff(prev_commit.as_ref(), &commit)
                    .ok()?;
                prev_commit = Some(commit);

                Some(diff)
            })
            .flatten()
            .collect();

        Ok(entries)
    }
}

impl GitBareRepository {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn open(&self) -> Result<OpenGitBareRepository> {
        let repo = git2::Repository::open_bare(self.0.as_str())?;
        Ok(OpenGitBareRepository(repo))
    }
}

fn is_markdown_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md") | Some("markdown")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_repo() -> GitBareRepository {
        GitBareRepository::new("gitnote.git")
    }

    #[test]
    fn test_diff_commit_range() {
        let repo = open_repo().open().expect("Failed to open repo");

        // 获取最新提交（HEAD）
        let head_commit = repo
            .repo()
            .head()
            .and_then(|h| h.peel_to_commit())
            .expect("Failed to get HEAD commit");

        // 获取其父提交（作为旧提交）
        let parent_commit = head_commit.parent(0).expect("HEAD has no parent commit");

        // 你也可以手动指定 commit hash 来对比特定范围
        let old_commit_hash = parent_commit.id().to_string();
        // let old_commit_hash = "0000000000000000000000000000000000000000".to_string();
        let new_commit_hash = head_commit.id().to_string();

        println!("Diff range: {} → {}", old_commit_hash, new_commit_hash);

        let entries = repo
            .diff_commits_from_str(&old_commit_hash, &new_commit_hash)
            .expect("Failed to diff commit range");

        // 打印变更记录
        for entry in &entries {
            if let RepoEntry::File {
                group: _,
                name: _,
                datetime,
                content: _,
            } = entry
            {
                println!("{} | datetime={}", entry, datetime);
            } else {
                println!("{}", entry);
            }
        }

        // 至少断言有结果（或者根据需要断言具体行为）
        assert!(
            !entries.is_empty(),
            "Expected some diff entries between commits"
        );
    }

    #[test]
    fn test_rebuild_all() {
        let repo = open_repo().open().expect("Failed to open repo");

        // 调用 rebuild_all 获取历史语义变更流
        let entries = repo.rebuild_all().expect("Failed to rebuild entries");

        // 输出结果（仅调试打印）
        for entry in entries {
            println!("{}", entry)
        }
    }
    #[test]
    fn test_is_markdown_file() {
        use std::path::Path;

        assert!(is_markdown_file(Path::new("doc.md")));
        assert!(is_markdown_file(Path::new("README.markdown")));
        assert!(!is_markdown_file(Path::new("config.toml")));
        assert!(!is_markdown_file(Path::new("no_extension")));
    }
}
