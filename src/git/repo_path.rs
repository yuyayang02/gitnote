use std::path::{Path, PathBuf};

use super::GitCommand;

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

/// 根据配置字符串初始化多个 Git 仓库。
///
/// 配置格式为 `name=url`，多个仓库用逗号分隔。例如：
///
/// ```text
/// repo1=https://example.com/repo1.git,repo2=https://example.com/repo2.git
/// ```
///
/// 对每个仓库，尝试 clone 或 fetch，保证本地仓库是最新状态。
///
/// # Panics
///
/// 当配置格式不符合 `name=url` 时会触发 panic。
pub fn init_git_repositories(config: String) {
    for entry in config.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        if let Some((name, url)) = entry.split_once('=') {
            let name = name.trim();
            let url = url.trim();
            tracing::debug!("Initializing repository '{}' from '{}'", name, url);

            GitCommand::clone_or_fetch(name, url);
        } else {
            panic!(
                "Invalid repository entry: '{}'. Expected format: name=url",
                entry
            );
        }
    }
}

/// 从环境变量 `REPO_CONFIG` 初始化 Git 仓库。
///
/// # Panics
///
/// 如果环境变量未设置会触发 panic。
pub fn init_git_repositories_from_env() {
    let config = std::env::var("REPO_CONFIG").expect("REPO_CONFIG not set");
    init_git_repositories(config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "需要操作主机目录"]
    fn test_init_git_repositories() {
        tracing_subscriber::fmt()
            .with_target(false)
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                "%Y-%m-%d %H:%M:%S%.3f".to_string(),
            ))
            .with_env_filter(tracing_subscriber::EnvFilter::from_env("GITNOTE_LOG"))
            .init();

        let repo_config = "repo_test1=gitnote.git";
        init_git_repositories(repo_config.to_string());
    }
}
