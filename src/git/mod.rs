mod command;
mod entry;
mod error;
mod git_operate;
mod git_repository;

use self::{
    command::GitCommand,
    entry::IntoRepoEntry,
    git_operate::{AsyncRepository, GitOps},
};

pub use self::{
    command::init_git_repositories_from_env,
    entry::{AsSummary, ChangeKind, FileKind, RepoEntry},
    error::GitError,
};

pub type GitRepository = git_repository::GitRepository<AsyncRepository>;
