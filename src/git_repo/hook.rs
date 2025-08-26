/// Git 推送类型，用于区分不同的 push 行为。
///
/// - [`PushKind::Sync`]：同步主分支，如 `refs/heads/main`
/// - [`PushKind::Rebuild`]：触发数据库重建命令 refs，如 `refs/tags/cmd/rebuild`
/// - [`PushKind::Ignore`]：其他不关心的 refs（分支或 tag）
#[derive(Debug)]
pub enum PushKind {
    Sync,
    Rebuild,
    Ignore,
}

/// Git 更新 hook payload，通常由 `update` hook 触发。
///
/// 包含触发更新的 ref、变更前后的 commit、操作用户以及仓库信息。
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GitPushPayload {
    /// 触发更新的 ref 名称，如 `refs/heads/main`
    pub refname: String,
    /// 更新前的 commit ID
    pub before: String,
    /// 更新后的 commit ID
    pub after: String,
    /// 执行 push 的用户名或邮箱
    pub pusher: String,
    /// 仓库名称或路径
    pub repository: String,
}

impl GitPushPayload {
    const ZERO_OID: &str = "0000000000000000000000000000000000000000";

    /// 根据 [`refname`] 和 [`before`] 推断对应的 [`PushKind`]。
    ///
    /// 规则：
    /// - `"refs/heads/main"` → [`PushKind::Sync`]
    /// - `"refs/tags/cmd/rebuild"` 且 `before` 为零值 → [`PushKind::Rebuild`]
    /// - 其他任意 ref → [`PushKind::Ignore`]
    ///
    pub fn push_kind(&self) -> PushKind {
        match self.refname.as_ref() {
            "refs/heads/main" => PushKind::Sync,
            "refs/tags/cmd/rebuild" if &self.before == Self::ZERO_OID => PushKind::Rebuild,
            _ => PushKind::Ignore,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_kind_main_branch() {
        let args = GitPushPayload {
            refname: "refs/heads/main".to_string(),
            before: "0000000000000000000000000000000000000000".to_string(),
            after: "abc123".to_string(),
            pusher: "alice".to_string(),
            repository: "repo1".to_string(),
        };
        assert!(matches!(args.push_kind(), PushKind::Sync));
    }

    #[test]
    fn test_ref_kind_rebuild() {
        let args = GitPushPayload {
            refname: "refs/tags/cmd/rebuild".to_string(),
            before: "0000000000000000000000000000000000000000".to_string(),
            after: "abc123".to_string(),
            pusher: "bob".to_string(),
            repository: "repo2".to_string(),
        };
        assert!(matches!(args.push_kind(), PushKind::Rebuild));
    }

    #[test]
    fn test_ref_kind_other() {
        let args = GitPushPayload {
            refname: "refs/heads/feature".to_string(),
            before: "abc123".to_string(),
            after: "def456".to_string(),
            pusher: "carol".to_string(),
            repository: "repo3".to_string(),
        };
        assert!(matches!(args.push_kind(), PushKind::Ignore));
    }

    #[test]
    fn test_ref_kind_rebuild_with_nonzero_before() {
        let args = GitPushPayload {
            refname: "refs/tags/cmd/rebuild".to_string(),
            before: "abc123".to_string(),
            after: "def456".to_string(),
            pusher: "dave".to_string(),
            repository: "repo4".to_string(),
        };
        assert!(matches!(args.push_kind(), PushKind::Ignore));
    }
}
