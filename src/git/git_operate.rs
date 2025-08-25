use std::path::Path;

use crate::error::Error;
use chrono::{DateTime, Local};
use git2::{
    Commit, Diff, Oid, Reference, Repository, Signature, Sort, Time, Tree, Worktree,
    WorktreeAddOptions, WorktreePruneOptions,
};

/// 提供对 Git 仓库的常用操作。
///
/// 实现 [`GitRepository`] 可以方便地执行 commit、diff、worktree 管理等操作。
pub trait GitRepository {
    /// 根据 commit ID 获取对应的 [`Commit`]。
    ///
    /// ```rust,no_run
    /// let commit = repo.resolve_commit("a1b2c3").unwrap();
    /// ```
    fn resolve_commit(&self, commit_id: &str) -> Result<Commit, Error>;

    /// 根据 tree ID 获取对应的 [`Tree`]。
    fn resolve_tree(&self, tree_id: &str) -> Result<Tree, Error>;

    /// 返回一个迭代器，用于按提交顺序遍历两个提交之间的差异。
    ///
    /// 每个元素是 `(Diff, Commit)`。
    fn diff_commits_stream<'repo>(
        &'repo self,
        old: Option<&Commit<'repo>>,
        new: &Commit<'repo>,
    ) -> Result<impl Iterator<Item = (Diff<'repo>, Commit<'repo>)>, Error>;

    /// 获取当前 HEAD 所指向的 [`Commit`]。
    fn head_commit(&self) -> Result<Commit, Error>;

    /// 获取指定分支的最新 [`Commit`]。
    fn branch_head_commit(&self, branch: &str) -> Result<Commit, Error>;

    /// 获取指定标签对应的 [`Commit`]。
    fn tag_commit(&self, tag: &str) -> Result<Commit, Error>;

    /// 创建新的 commit 并返回 commit ID。
    fn commit_with_paths(
        &self,
        parent: Option<Oid>,
        paths: &[&Path],
        dt: DateTime<Local>,
        message: impl AsRef<str>,
    ) -> Result<Oid, Error>;

    /// 获取指定时间点的 [`Signature`]。
    fn signature_at(&self, dt: DateTime<Local>) -> Result<Signature, Error>;

    /// 创建新的工作树，并返回 [`Worktree`] 和 [`Repository`]。
    fn create_worktree<P: AsRef<Path>>(
        &self,
        base_path: P,
        name: &str,
    ) -> Result<(Worktree, Repository), Error>;

    /// 清理指定的 [`Worktree`]。
    fn prune_worktree(&self, worktree: Worktree) -> Result<(), Error>;

    /// 创建新的引用（分支或标签）。
    fn create_reference(&self, name: &str, oid: Oid) -> Result<Reference, Error>;

    /// 读取指定 blob 的内容，如果无法解析为 UTF-8 返回 [`None`]。
    fn read_blob(&self, blob_id: &str) -> Option<String>;
}

impl GitRepository for Repository {
    /// 清理指定工作树的无效文件。
    fn prune_worktree(&self, worktree: Worktree) -> Result<(), Error> {
        let mut opts = WorktreePruneOptions::new();
        worktree.prune(Some(opts.valid(true))).map_err(Into::into)
    }

    /// 创建新的 commit 并返回 commit 的 Oid。
    ///
    /// 流程：
    /// 1. 获取并清空索引
    /// 2. 如果存在父 commit，加载父 commit 的 tree 作为基础
    /// 3. 添加指定文件的变更
    /// 4. 写入 tree 并生成 commit
    fn commit_with_paths(
        &self,
        parent: Option<Oid>,
        paths: &[&Path],
        dt: DateTime<Local>,
        message: impl AsRef<str>,
    ) -> Result<Oid, Error> {
        let mut index = self.index()?;
        index.clear()?;

        let parent = parent.map(|oid| self.find_commit(oid)).transpose()?;
        if let Some(commit) = parent.as_ref() {
            index.read_tree(&commit.tree()?)?;
        }

        for path in paths {
            index.add_path(path)?;
        }

        let tree_oid = index.write_tree()?;
        let tree_obj = self.find_tree(tree_oid)?;
        let sig = self.signature_at(dt)?;
        let parents = parent.iter().collect::<Vec<_>>();
        let commit_oid = self.commit(None, &sig, &sig, message.as_ref(), &tree_obj, &parents)?;
        Ok(commit_oid)
    }

    /// 按提交顺序遍历两个 commit 之间的差异。
    ///
    /// 返回一个迭代器，元素为 `(Diff, Commit)`，表示每个 commit 与上一个 tree 的差异。
    fn diff_commits_stream<'repo>(
        &'repo self,
        old: Option<&Commit<'repo>>,
        new: &Commit<'repo>,
    ) -> Result<impl Iterator<Item = (Diff<'repo>, Commit<'repo>)>, Error> {
        let old_oid = old.map(|c| c.id());
        let new_oid = new.id();

        let mut revwalk = self.revwalk()?;
        revwalk.push(new_oid)?;
        if let Some(old_oid) = old_oid {
            revwalk.hide(old_oid)?;
        }
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;

        let mut prev_tree_opt = old.map(|c| c.tree().ok()).flatten();

        let iter = revwalk.filter_map(move |oid_res| {
            let oid = oid_res.ok()?;
            let commit: Commit<'repo> = self.find_commit(oid).ok()?;
            let tree = commit.tree().ok()?;
            let diff = self
                .diff_tree_to_tree(prev_tree_opt.as_ref(), Some(&tree), None)
                .ok()?;
            prev_tree_opt = Some(tree);
            Some((diff, commit))
        });

        Ok(iter)
    }

    /// 根据指定时间构建 commit 签名。
    fn signature_at(&self, dt: DateTime<Local>) -> Result<Signature, Error> {
        const SIGNATURE_AUTHOR_NAME: &str = "gitnote-archive";
        const SIGNATURE_AUTHOR_EMAIL: &str = "gitnote-archive@example.com";

        let timestamp = dt.timestamp();
        let offset = dt.offset().local_minus_utc() / 60;
        Signature::new(
            SIGNATURE_AUTHOR_NAME,
            SIGNATURE_AUTHOR_EMAIL,
            &Time::new(timestamp, offset),
        )
        .map_err(Into::into)
    }

    /// 根据 commit ID 获取对应的 [`Commit`]。
    fn resolve_commit(&self, commit_id: &str) -> Result<Commit, Error> {
        let oid = Oid::from_str(commit_id)?;
        let commit = self.find_commit(oid)?;
        Ok(commit)
    }

    /// 根据 tree ID 获取对应的 [`Tree`]。
    fn resolve_tree(&self, tree_id: &str) -> Result<Tree, Error> {
        let oid = Oid::from_str(tree_id)?;
        let tree = self.find_tree(oid)?;
        Ok(tree)
    }

    /// 创建新的引用（分支或标签）。
    fn create_reference(&self, name: &str, commid_id: Oid) -> Result<Reference, Error> {
        self.reference(name, commid_id, true, "created branch")
            .map_err(Into::into)
    }

    /// 获取分支的最新 commit。
    ///
    /// 支持短名或完整引用路径。
    fn branch_head_commit(&self, branch: &str) -> Result<Commit, Error> {
        let full_ref = if branch.starts_with("refs/heads/") {
            branch
        } else {
            &format!("refs/heads/{}", branch)
        };
        let reference = self.find_reference(full_ref)?;
        let commit = reference.peel_to_commit()?;
        Ok(commit)
    }

    /// 获取 HEAD 指向的 commit。
    fn head_commit(&self) -> Result<Commit, Error> {
        let head = self.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit)
    }

    /// 获取标签对应的 commit。
    ///
    /// 会处理标签可能指向 tag 对象的情况。
    fn tag_commit(&self, tag: &str) -> Result<Commit, Error> {
        let full_ref = if tag.starts_with("refs/tags/") {
            tag
        } else {
            &format!("refs/tags/{}", tag)
        };
        let reference = self.find_reference(&full_ref)?;
        let commit = reference.peel_to_commit()?;
        Ok(commit)
    }

    /// 创建工作树并返回 [`Worktree`] 与对应 [`Repository`]。
    ///
    /// 流程：
    /// 1. 构建工作树路径
    /// 2. 获取 HEAD 作为参考
    /// 3. 清理残留 metadata
    /// 4. 创建工作树并打开 repository
    fn create_worktree<P: AsRef<Path>>(
        &self,
        base_path: P,
        name: &str,
    ) -> Result<(Worktree, Repository), Error> {
        let worktree_path = base_path.as_ref().join(name);
        let head_ref = self.head()?.resolve()?;
        let mut opts = WorktreeAddOptions::new();
        opts.reference(Some(&head_ref));

        let meta_path = self.path().join("worktrees").join(name);
        if meta_path.exists() {
            std::fs::remove_dir_all(&meta_path)?;
        }

        let worktree = self.worktree(name, &worktree_path, Some(&mut opts))?;
        let wt_repo = Repository::open(worktree.path())?;
        Ok((worktree, wt_repo))
    }

    /// 读取 blob 内容为 UTF-8 字符串，解析失败返回 [`None`]。
    fn read_blob(&self, oid: &str) -> Option<String> {
        let blob = self.find_blob(Oid::from_str(oid).ok()?).ok()?;
        std::str::from_utf8(blob.content())
            .ok()
            .map(|s| s.to_string())
    }
}
