use std::path::Path;

use crate::git::RepoDir;

use super::GitError;

/// 提供基于命令行 `git` 的仓库操作。
///
/// 与 [`git2`] API 相比，这里直接调用系统上的 `git` 命令，
/// 适合需要复用现有 `git` 行为的场景。
pub(super) struct GitCommand;

impl GitCommand {
    /// 在指定仓库目录执行 `git remote update`。
    ///
    /// 当需要强制与远程保持一致时使用。
    pub fn remote_update(name: impl AsRef<Path>) -> Result<(), GitError> {
        let repo_path = RepoDir::path(name);

        let output = std::process::Command::new("git")
            .arg("remote")
            .arg("update")
            .current_dir(repo_path)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    /// 克隆或更新远程仓库。
    ///
    /// - 如果目录已存在，则执行 `git remote update`
    /// - 如果目录不存在，则执行 `git clone --mirror`
    ///
    /// 常用于确保本地有一份最新的镜像仓库。
    pub fn clone_or_fetch(name: impl AsRef<Path>, remote: impl AsRef<str>) {
        let repo_path = RepoDir::path(name);

        let remote = remote.as_ref();

        if repo_path.exists() {
            tracing::debug!(
                "Repository '{}' exists, running 'git remote update'",
                repo_path.display()
            );
            let output = std::process::Command::new("git")
                .args(["remote", "update"])
                .current_dir(&repo_path)
                .output()
                .expect("Failed to start git process");

            if !output.status.success() {
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                panic!(
                    "git remote update failed for '{}': {}",
                    repo_path.display(),
                    stderr_str
                );
            } else {
                tracing::debug!("git remote update succeeded for '{}'", repo_path.display());
            }
        } else {
            tracing::debug!("Cloning into '{}' from '{}'", repo_path.display(), remote);

            let output = std::process::Command::new("git")
                .args(["clone", "--mirror", remote])
                .arg(&repo_path)
                .output()
                .expect("Failed to start git process");

            if !output.status.success() {
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                panic!(
                    "git clone --mirror '{}' '{}' failed: {}",
                    remote,
                    repo_path.display(),
                    stderr_str
                );
            } else {
                tracing::debug!(
                    "git clone succeeded: '{}' -> '{}'",
                    remote,
                    repo_path.display()
                );
            }
        }
    }
}
