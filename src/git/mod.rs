mod entry;
mod error;
mod git_operate;
mod git_repository;
mod repo_path;

use self::{
    entry::{IntoRepoEntry, RepoEntryPrune},
    git_operate::{AsyncRepository, GitOps},
    repo_path::RepoDir,
};

pub use self::{
    entry::{AsSummary, ChangeKind, FileKind, RepoEntry},
    error::GitError,
};

pub type GitRepository = git_repository::GitRepository<AsyncRepository>;
