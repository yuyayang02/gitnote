/// Git 引用类型
///
/// 用于区分不同类型的 refs，如主分支、归档命令、重建命令等。
#[derive(Debug)]
pub enum RefKind {
    /// 主分支（main 分支）
    MainBranch,
    /// 归档命令对应的 refs（如 `cmd/archive`）
    Archive,
    /// 归档合并命令 refs（如 `cmd/archive.merge`）
    ArchiveMerge,
    /// 重建数据库命令 refs（如 `cmd/rebuild`）
    Rebuild,
    /// 全量重建数据库命令 refs（如 `cmd/rebuild.all`）
    RebuildAll,
    /// 其他不关心的 refs（分支或 tag）
    Other,
}

impl RefKind {
    /// 根据 ref 名称解析 [`RefKind`] 类型
    ///
    /// 支持的 ref 名称：
    /// - `"refs/heads/main"` → [`RefKind::MainBranch`]
    /// - `"refs/tags/cmd/archive"` → [`RefKind::Archive`]
    /// - `"refs/tags/cmd/archive.merge"` → [`RefKind::ArchiveMerge`]
    /// - `"refs/tags/cmd/rebuild"` → [`RefKind::Rebuild`]
    /// - `"refs/tags/cmd/rebuild.all"` → [`RefKind::RebuildAll`]
    /// - 其他任意 ref → [`RefKind::Other`]
    ///
    /// ```
    /// # use gitnote::git_repo::RefKind;
    /// let kind = RefKind::parse_ref_kind("refs/heads/main");
    /// assert!(matches!(kind, RefKind::MainBranch));
    /// ```
    pub fn parse_ref_kind(ref_name: &str) -> Self {
        match ref_name {
            "refs/heads/main" => RefKind::MainBranch,
            "refs/tags/cmd/archive" => RefKind::Archive,
            "refs/tags/cmd/archive.merge" => RefKind::ArchiveMerge,
            "refs/tags/cmd/rebuild" => RefKind::Rebuild,
            "refs/tags/cmd/rebuild.all" => RefKind::RebuildAll,
            _ => RefKind::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ref_kind_all_cases() {
        let cases = vec![
            ("refs/heads/main", RefKind::MainBranch),
            ("refs/tags/cmd/archive", RefKind::Archive),
            ("refs/tags/cmd/archive.merge", RefKind::ArchiveMerge),
            ("refs/tags/cmd/rebuild", RefKind::Rebuild),
            ("refs/tags/cmd/rebuild.all", RefKind::RebuildAll),
            ("refs/heads/feature-x", RefKind::Other),
            ("refs/tags/random-tag", RefKind::Other),
        ];

        for (ref_name, expected) in cases {
            let kind = RefKind::parse_ref_kind(ref_name);
            match (kind, expected) {
                (RefKind::MainBranch, RefKind::MainBranch)
                | (RefKind::Archive, RefKind::Archive)
                | (RefKind::ArchiveMerge, RefKind::ArchiveMerge)
                | (RefKind::Rebuild, RefKind::Rebuild)
                | (RefKind::RebuildAll, RefKind::RebuildAll)
                | (RefKind::Other, RefKind::Other) => {}
                (a, b) => panic!("For ref '{}', expected {:?}, got {:?}", ref_name, b, a),
            }
        }
    }
}
