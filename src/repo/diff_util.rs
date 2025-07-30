use std::path::Path;

use chrono::{Local, TimeZone};
use git2::{Commit, DiffDelta, Oid, Repository, Sort};

use super::entry::RepoEntry;
use crate::{error::Result, repo::ReadBlob};

/// 用于对比两个 Git 提交之间的差异，并将变更内容转换为 [`RepoEntry`] 类型，便于后续处理。
///
/// 此结构封装了对 `git2::Repository` 的引用，并提供了多个差异分析方法，包括单次提交比较、多个提交合并对比、以及 HEAD 全量差异分析等。
///
/// 典型用途：用于在 Git 钩子触发时（如推送或打标签）分析变更内容，并将相关 Markdown 文件或 `.gitnote.toml` 配置文件的变化提取出来，归类为创建、修改或删除等逻辑操作。
///
/// # 相关类型
/// - [`RepoEntry`]：代表一次对仓库内容的结构化修改记录
/// - [`crate::repo::ReadBlob`]：用于读取 blob 内容并转为字符串
pub(super) struct DiffUtil<'a> {
    repo: &'a Repository,
}

impl super::ReadBlob for DiffUtil<'_> {}

impl<'a> DiffUtil<'a> {
    /// 构造新的 DiffUtil，用于对比差异。
    pub fn new(repo: &'a Repository) -> Self {
        Self { repo }
    }
    /// 将新添加的文件按类型解析为 [`RepoEntry::GitNote`] 或 [`RepoEntry::File`] 类型。
    fn push_new_file_entry(
        &self,
        entries: &mut Vec<RepoEntry>,
        path: &Path,
        oid: Oid,
        commit: &Commit,
    ) {
        if path.ends_with(super::GITNOTE_FILENAME) {
            if let Some(content) = Self::read_blob(self.repo, oid) {
                entries.push(RepoEntry::GitNote {
                    group: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
                    content,
                });
            }
        } else if super::is_md(path) {
            if let Some(content) = Self::read_blob(self.repo, oid) {
                entries.push(RepoEntry::File {
                    group: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
                    name: path.file_name().unwrap().to_string_lossy().to_string(),
                    datetime: Local.timestamp_opt(commit.time().seconds(), 0).unwrap(),
                    content,
                });
            }
        }
    }

    /// 处理被删除的文件，转换为对应的删除类型 [`RepoEntry`]。
    fn push_remove_file_entry(&self, entries: &mut Vec<RepoEntry>, path: &Path) {
        if path.ends_with(super::GITNOTE_FILENAME) {
            entries.push(RepoEntry::RemoveGitNote {
                group: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
            });
        } else if super::is_md(path) {
            entries.push(RepoEntry::RemoveFile {
                group: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
                name: path.file_name().unwrap().to_string_lossy().to_string(),
            });
        }
    }

    /// 处理单个变更项 [`DiffDelta`]，并转换为对应的结构化操作（增删改）。
    fn handle_delta(&self, entries: &mut Vec<RepoEntry>, delta: &DiffDelta, new_commit: &Commit) {
        use git2::Delta::*;

        let old_file = delta.old_file();
        let new_file = delta.new_file();

        match delta.status() {
            Added => {
                if let Some(path) = new_file.path() {
                    self.push_new_file_entry(entries, path, new_file.id(), new_commit);
                }
            }
            Deleted => {
                if let Some(path) = old_file.path() {
                    self.push_remove_file_entry(entries, path);
                }
            }
            Modified | Renamed | Copied => {
                if let Some(path) = old_file.path() {
                    self.push_remove_file_entry(entries, path);
                }
                if let Some(path) = new_file.path() {
                    self.push_new_file_entry(entries, path, new_file.id(), new_commit);
                }
            }
            _ => {}
        }
    }

    /// 对比两个提交之间的变更，分析并提取所有 [`RepoEntry`]。
    pub fn compute_commit_diff(
        &self,
        old_commit: Option<&Commit>,
        new_commit: &Commit,
    ) -> Result<Vec<RepoEntry>> {
        let new_tree = new_commit.tree()?;
        let old_tree = old_commit.map(|c| c.tree()).transpose()?;

        let diff = self
            .repo
            .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), None)?;
        let mut entries = Vec::new();

        diff.foreach(
            &mut |delta, _| {
                self.handle_delta(&mut entries, &delta, new_commit);
                true
            },
            None,
            None,
            None,
        )?;

        Ok(entries)
    }

    /// 从多个提交之间提取所有变更，合并并过滤“增后删”冗余，适用于归档时历史分析。
    ///
    /// 内部通过 [`revwalk`] 遍历提交路径，从旧提交到新提交依次比对并提取 [`RepoEntry`]。
    pub fn diff_commits(
        &self,
        old_commit: Option<&Commit>,
        new_commit: &Commit,
    ) -> Result<Vec<RepoEntry>> {
        let old_oid = old_commit.map(|c| c.id());
        let new_oid = new_commit.id();

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(new_oid)?;
        if let Some(old_oid) = old_oid {
            revwalk.hide(old_oid)?;
        }
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;

        let mut prev_commit = old_commit.cloned();

        let entries = revwalk
            .filter_map(|oid_res| {
                let oid = oid_res.ok()?;
                let commit = self.repo.find_commit(oid).ok()?;

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

    /// 提供字符串形式的提交 ID 接口，便于 HTTP 解析调用。
    pub fn diff_commits_from_str(
        &self,
        old_commit_str: impl AsRef<str>,
        new_commit_str: impl AsRef<str>,
    ) -> Result<Vec<RepoEntry>> {
        let old_commit = self
            .repo
            .find_commit(Oid::from_str(old_commit_str.as_ref())?)
            .ok();
        let new_commit = self
            .repo
            .find_commit(Oid::from_str(new_commit_str.as_ref())?)?;

        self.diff_commits(old_commit.as_ref(), &new_commit)
    }

    /// 对比当前 HEAD 与空提交之间的所有差异，相当于获取仓库所有现有文件。
    pub fn diff_all(&self) -> Result<Vec<RepoEntry>> {
        let head = self.repo.head()?.peel_to_commit()?;
        self.diff_commits(None, &head)
    }
}
