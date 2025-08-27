mod command;
mod entry;
mod error;
mod git_operate;
mod git_repository;
mod repo_path;

use self::{
    command::GitCommand,
    entry::IntoRepoEntry,
    git_operate::{AsyncRepository, GitOps},
    repo_path::RepoDir,
};

pub use self::{
    repo_path::init_git_repositories_from_env,
    entry::{AsSummary, ChangeKind, FileKind, RepoEntry},
    error::GitError,
};

pub type GitRepository = git_repository::GitRepository<AsyncRepository>;
