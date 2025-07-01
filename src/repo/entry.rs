use std::path::PathBuf;

use chrono::{DateTime, Local};

#[derive(Debug)]
pub enum RepoEntry {
    GitNote {
        group: PathBuf,
        content: String,
    },
    RemoveGitNote {
        group: PathBuf,
    },
    File {
        group: PathBuf,
        name: String,
        datetime: DateTime<Local>,
        content: String,
    },
    RemoveFile {
        group: PathBuf,
        name: String,
    },
}
