/// 模块：归档器（Archiver）
///
/// 该模块提供一个完整的归档处理流程，核心功能包括：
///
/// - 基于标签提交生成一组精简后的提交历史
/// - 使用 [`git2::Worktree`] 临时工作树写入新的提交
/// - 每组归档内容生成独立提交，并最终推送至 `archive/<tag>` 分支
///
/// 本模块依赖 [`RepoEntry`] 表示归档的变更项；依赖 [`git2::Repository`] 管理底层仓库。
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Local};
use git2::{
    Oid, Repository, Signature, Time, Tree, Worktree, WorktreeAddOptions, WorktreePruneOptions,
};

use super::RepoEntry;
use crate::error::Result;

/// 表示一次归档操作完成后的汇总信息。
///
/// 包括归档生成的分支名、归档时刻、耗时，以及归档过程中收集到的文件条目。
/// 用于在归档执行完毕后进行结果展示、日志输出等。
pub struct ArchivedInfo {
    /// 创建的归档分支名称，例如："refs/heads/archive/2025-Q2"
    pub branch: String,

    /// 执行归档操作的本地时间戳。
    pub datetime: DateTime<Local>,

    /// 归档操作耗时。
    pub duration: Duration,

    /// 被归档的所有文件（包括普通文件与 GitNote）。
    pub repo_entries: Vec<RepoEntry>,
}

impl ArchivedInfo {
    /// 构建一条格式化的摘要消息，用于描述归档的结果。
    ///
    /// 会统计归档中包含的文件和 GitNote 数量，以及分支信息与耗时，通常用于终端或日志输出。
    pub fn summary(&self) -> String {
        let count_md = self
            .repo_entries
            .iter()
            .filter(|entry| matches!(entry, RepoEntry::File { .. }))
            .count();
        let count_gitnote = self
            .repo_entries
            .iter()
            .filter(|entry| matches!(entry, RepoEntry::GitNote { .. }))
            .count();

        let duration_secs = self.duration.as_secs_f64();

        format!(
            "[ARCHIVE SUMMARY] Archive completed successfully in {:.3}s\n\
             [branch]    => {}\n\
             [entries]   => {} files, {} configs\n\
             [datetime]  => {}\n",
            duration_secs,
            self.branch,
            count_md,
            count_gitnote,
            self.datetime.format("%Y-%m-%d %H:%M:%S"),
        )
    }
}

/// 用于执行 Git 归档操作的结构体。
///
/// 提供从指定标签或提交构建归档分支的功能，生成新的归档历史。
/// 每个归档会以时间分组方式，依次构造提交历史，从而形成简洁可读的归档轨迹。
/// 其生命周期绑定于一个 [`git2::Repository`] 实例。
pub(super) struct Archiver<'a> {
    repo: &'a Repository,
}

impl super::ReadBlob for Archiver<'_> {}

impl<'a> Archiver<'a> {
    /// 创建归档器实例，绑定目标仓库。
    pub fn new(repo: &'a Repository) -> Self {
        Self { repo }
    }

    /// 创建临时 [`Worktree`] 作为归档写入用的独立工作目录。
    ///
    /// 每次归档操作都在独立目录下运行，避免污染主仓库结构。
    fn prepare_worktree(
        base_path: &Path,
        repo: &Repository,
        tag_str: &str,
    ) -> Result<(Worktree, Repository)> {
        let worktree_path = base_path.join(tag_str);
        let reference = repo.head()?.resolve()?;

        let mut opts = WorktreeAddOptions::new();

        // 清理空的残留工作树路径（可能是上次中断的结果）
        let meta_path = repo.path().join("worktrees").join(tag_str);
        if meta_path.exists() && meta_path.read_dir()?.next().is_none() {
            std::fs::remove_dir_all(&meta_path)?;
        }

        // 创建工作树并打开其 repo 实例
        let worktree = repo.worktree(
            tag_str,
            &worktree_path,
            Some(&mut opts.reference(Some(&reference))),
        )?;
        let wt_repo = Repository::open(worktree.path())?;

        Ok((worktree, wt_repo))
    }

    /// 将变更项按时间归组，生成时间 -> 文件路径列表的映射。
    ///
    /// 用于批量归档时构造一个个提交，每组同时间的文件归为一个提交。
    fn group_entries_by_time(
        entries: &[RepoEntry],
    ) -> (BTreeMap<DateTime<Local>, Vec<PathBuf>>, DateTime<Local>) {
        let now = Local::now();
        let mut grouped: BTreeMap<DateTime<Local>, Vec<PathBuf>> = BTreeMap::new();

        for entry in entries {
            let (path, dt) = match entry {
                RepoEntry::File {
                    datetime,
                    name,
                    group,
                    ..
                } => (group.join(name), datetime),
                RepoEntry::GitNote { group, .. } => (group.join(super::GITNOTE_FILENAME), &now),
                _ => continue,
            };
            grouped.entry(*dt).or_default().push(path);
        }

        (grouped, now)
    }

    /// 构造指定时间的 [`Signature`]，用于归档提交保持历史一致性。
    fn signature_at_time<'s>(dt: DateTime<Local>) -> Result<Signature<'s>> {
        let timestamp = dt.timestamp();
        let offset = dt.offset().local_minus_utc() / 60;
        Signature::new(
            "gitnote-archive",
            "gitnote-archive@example.com",
            &Time::new(timestamp, offset),
        )
        .map_err(Into::into)
    }

    /// 生成归档分支的引用名
    ///
    /// 输入归档标签，如 "2025-Q2"，返回完整的 Git 分支路径：refs/heads/archived/2025-Q2。
    fn reference_name(name: &str) -> String {
        format!("refs/heads/archived/{}", name)
    }

    /// 主归档函数：执行整个归档操作流程。
    ///
    /// - 创建临时工作树
    /// - 获取归档变更项（[`RepoEntry`]）
    /// - 将其按时间归组，构造多个提交
    /// - 最后挂载到新分支 `archived/<tag>` 下
    ///
    /// 返回 [`ArchivedInfo`] 包含归档摘要信息
    pub fn archive(&self, tag_str: &str, tag_commit_str: &str) -> Result<ArchivedInfo> {
        let start = std::time::Instant::now();

        // 创建临时目录用于挂载工作树
        let tempdir = tempfile::tempdir()?;

        // 创建工作树
        let (worktree, worktree_repo) = Self::prepare_worktree(tempdir.path(), self.repo, tag_str)?;

        let tag_commit = worktree_repo.find_commit(Oid::from_str(tag_commit_str)?)?;

        // 获取从初始到目标 commit 的精简变更列表
        let entries = super::DiffUtil::new(self.repo).diff_commits(None, &tag_commit)?;

        // 根据时间分组归档条目
        let (grouped, now) = Self::group_entries_by_time(&entries);

        let mut parent: Option<Oid> = None;
        let mut tree: Option<Tree> = None;

        // 按时间构建 commit 链
        for (dt, paths) in grouped {
            let mut index = worktree_repo.index()?;
            index.clear()?;

            // 若有前一个 tree，加载作为当前 index 基础
            if let Some(ref t) = tree {
                index.read_tree(t)?;
            }

            // 添加本时间点下的文件变更
            for path in paths {
                index.add_path(&path)?;
            }

            let tree_oid = index.write_tree()?;
            let tree_obj = worktree_repo.find_tree(tree_oid)?;
            let sig = Self::signature_at_time(dt)?;

            // 构建 commit，连接上一个父节点（若有）
            let parents = if let Some(p) = parent {
                vec![worktree_repo.find_commit(p)?]
            } else {
                vec![]
            };

            let commit_oid = worktree_repo.commit(
                None,
                &sig,
                &sig,
                &format!(
                    "archive: archived({}) at {}",
                    tag_str,
                    dt.format("%Y-%m-%d %H:%M:%S")
                ),
                &tree_obj,
                &parents.iter().collect::<Vec<_>>(),
            )?;

            tree = Some(tree_obj);
            parent = Some(commit_oid);
        }

        let final_oid = parent.expect("至少应有一次提交");

        let reference_name = Self::reference_name(tag_str);

        // 创建归档分支引用
        worktree_repo.reference(&reference_name, final_oid, true, "created archive branch")?;

        // 清理临时工作树
        let mut opts = WorktreePruneOptions::new();
        worktree.prune(Some(opts.valid(true)))?;

        let dt = start.elapsed();

        Ok(ArchivedInfo {
            branch: reference_name,
            datetime: now,
            duration: dt,
            repo_entries: entries,
        })
    }
}
