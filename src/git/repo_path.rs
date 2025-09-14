use std::path::{Path, PathBuf};

/// 仓库路径工具。
///
/// 所有仓库都存放在固定的根目录 [`RepoPath::ROOT_PATH`] 下。
/// 用 [`RepoPath::path`] 可以得到某个仓库的完整路径。
///
pub(super) struct RepoDir;

impl RepoDir {
    /// 仓库根目录，所有仓库均存放于此。
    const ROOT_DIR: &str = "repositories";

    /// 返回指定仓库的完整路径。
    pub fn path(path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();

        // 如果已经是 .git 结尾，直接使用，否则添加 .git
        let final_name = if path.extension().and_then(|ext| ext.to_str()) == Some("git") {
            path.to_path_buf()
        } else {
            path.with_extension("git")
        };

        Path::new(Self::ROOT_DIR).join(final_name)
    }
}