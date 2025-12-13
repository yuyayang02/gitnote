mod entry;
mod error;
mod operations;
mod repository;

use self::{
    entry::{GitFileEntryPrune, IntoGitFileEntry},
    operations::{AsyncGitClient, GitOperation},
};

pub use self::{
    entry::{AsSummary, ChangeKind, FileKind, GitFileEntry},
    error::GitError,
};

pub type GitClient = repository::GitClient<AsyncGitClient>;
