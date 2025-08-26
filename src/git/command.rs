use std::path::Path;

use super::GitError;

pub struct GitCommand;

impl GitCommand {
    pub fn remote_update(path: impl AsRef<Path>) -> Result<(), GitError> {
        let output = std::process::Command::new("git")
            .arg("remote")
            .arg("update")
            .current_dir(path.as_ref())
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    pub fn clone_or_fetch(name: impl AsRef<str>, remote: impl AsRef<str>) -> String {
        let target = format!("{}.git", name.as_ref());
        let repo_path = Path::new(&target);
        let remote = remote.as_ref();

        if repo_path.exists() {
            tracing::debug!(
                "Repository '{}' exists, running 'git remote update'",
                target
            );
            let output = std::process::Command::new("git")
                .args(["remote", "update"])
                .current_dir(repo_path)
                .output()
                .expect("Failed to start git process");

            if !output.status.success() {
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                panic!("git remote update failed for '{}': {}", target, stderr_str);
            } else {
                tracing::debug!("git remote update succeeded for '{}'", target);
            }
        } else {
            tracing::debug!("Cloning repository '{}' from '{}'", target, remote);
            let output = std::process::Command::new("git")
                .args(["clone", "--mirror", remote, &target])
                .output()
                .expect("Failed to start git process");

            if !output.status.success() {
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                panic!(
                    "git clone --mirror '{}' '{}' failed: {}",
                    remote, target, stderr_str
                );
            } else {
                tracing::debug!("git clone succeeded: '{}' -> '{}'", remote, target);
            }
        }
        target
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
pub fn init_git_repositories(config: String) -> Vec<String> {
    let mut result = vec![];
    for entry in config.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        if let Some((name, url)) = entry.split_once('=') {
            let name = name.trim();
            let url = url.trim();
            tracing::debug!("Initializing repository '{}' from '{}'", name, url);

            let path = GitCommand::clone_or_fetch(name, url);
            result.push(path);
        } else {
            panic!(
                "Invalid repository entry: '{}'. Expected format: name=url",
                entry
            );
        }
    }

    result
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

        let repo_config =
            "repo_test1=gitnote.git,repo_test2=git@github.com:octocat/Hello-World.git";
        init_git_repositories(repo_config.to_string());
    }
}
