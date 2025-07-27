use std::path::Path;

use chrono::{Local, TimeZone};
use git2::{Commit, Delta, Oid, Repository};

use crate::error::Result;

use super::entry::RepoEntry;

pub struct GitBareRepository(String);
/// 表示一个打开的 Git bare 仓库（只读），用于处理笔记文件变更等语义化操作。
pub struct OpenGitBareRepository(Repository);

impl OpenGitBareRepository {
    /// `.gitnote.toml` 配置文件的标准文件名（用于识别配置文件）
    const GITNOTE_FILENAME: &'static str = ".gitnote.toml";

    /// 获取内部封装的 Git 仓库对象引用
    #[inline]
    fn repo(&self) -> &Repository {
        &self.0
    }

    /// 通过对象 ID（Oid）读取对应 Blob 内容，并尝试将其解析为 UTF-8 字符串。
    /// 失败（如找不到、非 UTF-8 编码）时返回 None。
    fn read_blob(&self, oid: Oid) -> Option<String> {
        let blob = self.repo().find_blob(oid).ok()?;
        std::str::from_utf8(blob.content())
            .ok()
            .map(|s| s.to_string())
    }

    /// 比较两个 Git 提交之间的差异，返回语义化的变更列表（RepoEntry）。
    ///
    /// 参数：
    /// - `old_commit`: 可选的旧提交；若为 `None`，则视为空树（首次提交）。
    /// - `new_commit`: 新提交。
    ///
    /// 返回：
    /// - 所有 `.gitnote.toml` 和 `.md` 文件的新增、删除、修改的变更项。
    fn compute_commit_diff(
        &self,
        old_commit: Option<&Commit>,
        new_commit: &Commit,
    ) -> Result<Vec<RepoEntry>> {
        // 获取新旧 Tree（树）对象
        let tree = new_commit.tree()?; // 新提交对应的 Tree
        let parent_tree: Option<git2::Tree<'_>> = old_commit.map(|c| c.tree()).transpose()?; // 旧 Tree（如有）

        let mut entries = Vec::new();

        // 比较两个 Tree 的差异（相当于 git diff）
        let diff = self
            .repo()
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

        // 遍历每个差异文件（delta）
        diff.foreach(
            &mut |delta, _| {
                let old_file = delta.old_file();
                let new_file = delta.new_file();

                match delta.status() {
                    // 新增文件：检测是否为 GitNote 配置或 Markdown 文章
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
                                            .unwrap(),
                                        content,
                                    });
                                }
                            }
                        }
                    }

                    // 删除文件：记录删除动作
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

                    // 修改、重命名、复制等变化：保守策略是先删后增
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

                    // 其他状态忽略（如 Unmodified、Ignored、Conflicted）
                    _ => {}
                }

                true // 继续处理下一个文件
            },
            None,
            None,
            None, // 只关心文件级差异，不处理 diff 内容
        )?;

        Ok(entries)
    }

    /// 获取从 `old_commit` 到 `new_commit` 之间的所有变更（语义化 RepoEntry 列表）。
    ///
    /// - 如果 `old_commit` 为 `None`，则表示从空树开始构建（即重建整个历史）。
    /// - 该方法自动使用 Git 的 revwalk（拓扑排序 + 正序）处理多提交差异。
    pub fn diff_commits(
        &self,
        old_commit: Option<&Commit>,
        new_commit: &Commit,
    ) -> Result<Vec<RepoEntry>> {
        let repo = self.repo();

        let old_oid = old_commit.map(|c| c.id());
        let new_oid = new_commit.id();

        // 设置 revwalk：从 new_oid 开始，排除 old_oid 及其祖先
        let mut revwalk = repo.revwalk()?;
        revwalk.push(new_oid)?;
        if let Some(old_oid) = old_oid {
            revwalk.hide(old_oid)?;
        }
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)?; // 按拓扑 + 正序

        let mut prev_commit = old_commit.cloned();

        // 依次比较每个提交之间的变化
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

        Ok(super::entry::strip_add_then_remove(entries))
    }

    /// 提供字符串形式的提交 ID 比较差异（封装 `diff_commits` 的便捷入口）。
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

    /// 重建整个 Git 历史，从初始提交开始回放变更（语义化）。
    ///
    /// 本质上是从空树到 `HEAD` 的 `diff_commits`。
    pub fn rebuild_all(&self) -> Result<Vec<RepoEntry>> {
        let head = self.repo().head()?.peel_to_commit()?;
        self.diff_commits(None, &head)
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
