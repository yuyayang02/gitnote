use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, TimeZone};
use git2::{Commit, Diff};

/// 枚举表示文件的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// `.gitnote.toml` 文件
    GitNote,
    /// Markdown 文件 (`.md` 或 `.markdown`)
    Markdown,
    /// 其他文件类型
    Other,
}

impl FileKind {
    /// 根据文件路径推断文件类型
    ///
    /// 文件类型判断规则：
    /// - 文件名为 `.gitnote.toml` 返回 [`FileKind::GitNote`]
    /// - 扩展名为 `.md` 或 `.markdown` 返回 [`FileKind::Markdown`]
    /// - 其他情况返回 [`FileKind::Other`]
    ///
    /// # Example
    /// ```
    /// # use gitnote::git::FileKind;
    /// assert_eq!(FileKind::from_path(".gitnote.toml"), FileKind::GitNote);
    /// assert_eq!(FileKind::from_path("doc.md"), FileKind::Markdown);
    /// assert_eq!(FileKind::from_path("image.png"), FileKind::Other);
    /// ```
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name == ".gitnote.toml" {
                return FileKind::GitNote;
            }
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            return match ext {
                "md" | "markdown" => FileKind::Markdown,
                _ => FileKind::Other,
            };
        }

        FileKind::Other
    }
}

/// 枚举表示文件变更类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// 新增
    Added,
    /// 删除
    Deleted,
}

/// 表示 Git 仓库中一次文件或 GitNote 的变更。
///
/// `RepoEntry` 包含文件路径、变更类型、文件类型和提交时间等信息，
#[derive(Debug)]
pub struct RepoEntry {
    pub(crate) id: String,
    pub(crate) path: PathBuf,
    pub(crate) change_kind: ChangeKind,
    pub(crate) file_kind: FileKind,
    pub(crate) timestamp: DateTime<Local>,
}

impl RepoEntry {
    /// RepoEntry 的唯一 ID。
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 文件路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 文件变更类型。
    pub fn change_kind(&self) -> ChangeKind {
        self.change_kind
    }

    /// 文件类型。
    pub fn file_kind(&self) -> FileKind {
        self.file_kind
    }

    /// 提交时间。
    pub fn timestamp(&self) -> DateTime<Local> {
        self.timestamp
    }
}

/// Trait，用于将 Git `Diff` 和 `Commit` 转换为 [`RepoEntry`]。
pub trait IntoEntry {
    /// 将类型转换为 [`RepoEntry`] 列表。
    fn into_entry(self) -> Vec<RepoEntry>;
}

impl<'a> IntoEntry for (Diff<'a>, Commit<'a>) {
    fn into_entry(self) -> Vec<RepoEntry> {
        let (diff, commit) = self;
        let timestamp = Local.timestamp_opt(commit.time().seconds(), 0).unwrap();

        let mut final_map: HashMap<&Path, ChangeKind> = HashMap::new();

        diff.deltas()
            .flat_map(|d| match d.status() {
                git2::Delta::Added | git2::Delta::Copied => {
                    vec![(d.new_file(), ChangeKind::Added)]
                }
                git2::Delta::Deleted => vec![(d.old_file(), ChangeKind::Deleted)],
                git2::Delta::Modified => vec![
                    (d.old_file(), ChangeKind::Deleted),
                    (d.new_file(), ChangeKind::Added),
                ],
                git2::Delta::Renamed => vec![
                    (d.old_file(), ChangeKind::Deleted),
                    (d.new_file(), ChangeKind::Added),
                ],
                _ => vec![],
            })
            .filter_map(|(file, change)| {
                let path = file.path()?;
                let real_change = merge_change(final_map.get(&path), change)?;
                final_map.insert(path, real_change);

                Some(RepoEntry {
                    id: file.id().to_string(),
                    path: path.to_path_buf(),
                    change_kind: real_change,
                    file_kind: FileKind::from_path(path),
                    timestamp,
                })
            })
            .collect()
    }
}
/// 合并旧的变更状态和新的变更状态。
///
/// 规则说明：
/// - [`ChangeKind::Added`] -> [`ChangeKind::Deleted`] = 消失 (返回 None)
/// - [`ChangeKind::Deleted`] -> [`ChangeKind::Added`] = [`ChangeKind::Added`]
/// - 其他情况保持最新状态
fn merge_change(old: Option<&ChangeKind>, new: ChangeKind) -> Option<ChangeKind> {
    match (old, new) {
        (None, now) => Some(now),
        (Some(ChangeKind::Added), ChangeKind::Added) => Some(ChangeKind::Added),
        (Some(ChangeKind::Added), ChangeKind::Deleted) => None,
        (Some(ChangeKind::Deleted), ChangeKind::Added) => Some(ChangeKind::Added),
        (Some(ChangeKind::Deleted), ChangeKind::Deleted) => Some(ChangeKind::Deleted),
    }
}

impl fmt::Display for RepoEntry {
    /// 格式化 [`RepoEntry`] 为字符串，用于 CLI 或日志输出。
    ///
    /// 格式示例：
    /// ```text
    /// [md]    + group-a/test.md @ 2024-08-22 12:30
    /// [group] + .gitnote.toml @ 2024-08-22 12:35
    /// ```
    ///
    /// 枚举跳转：
    /// - [`FileKind::GitNote`]
    /// - [`FileKind::Markdown`]
    /// - [`FileKind::Other`]
    /// - [`ChangeKind::Added`]
    /// - [`ChangeKind::Deleted`]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_str = match self.file_kind {
            FileKind::GitNote => "[group]",
            FileKind::Markdown => "[md]",
            FileKind::Other => "[-]",
        };

        let change_str = match self.change_kind {
            ChangeKind::Added => "+",
            ChangeKind::Deleted => "-",
        };

        write!(
            f,
            "{:<7} {} {} @ {}",
            kind_str,
            change_str,
            self.path.display(),
            self.timestamp.format("%Y-%m-%d %H:%M")
        )
    }
}

pub trait AsSummary {
    fn as_summary(&self) -> String;
}

impl AsSummary for Vec<RepoEntry> {
    fn as_summary(&self) -> String {
        // 按时间从老到新排序
        // self.sort_by_key(|e| e.timestamp);
        // 转换为多行字符串
        self
            .into_iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_file_kind_from_path() {
        assert_eq!(FileKind::from_path(".gitnote.toml"), FileKind::GitNote);
        assert_eq!(FileKind::from_path("doc.md"), FileKind::Markdown);
        assert_eq!(FileKind::from_path("notes.markdown"), FileKind::Markdown);
        assert_eq!(FileKind::from_path("image.png"), FileKind::Other);
        assert_eq!(FileKind::from_path("folder/unknown"), FileKind::Other);
    }

    #[test]
    fn test_merge_change_logic() {
        use ChangeKind::*;

        // 没有旧状态
        assert_eq!(merge_change(None, Added), Some(Added));
        assert_eq!(merge_change(None, Deleted), Some(Deleted));

        // Added -> Added
        assert_eq!(merge_change(Some(&Added), Added), Some(Added));

        // Added -> Deleted → None
        assert_eq!(merge_change(Some(&Added), Deleted), None);

        // Deleted -> Added → Added
        assert_eq!(merge_change(Some(&Deleted), Added), Some(Added));

        // Deleted -> Deleted → Deleted
        assert_eq!(merge_change(Some(&Deleted), Deleted), Some(Deleted));
    }

    #[test]
    fn test_repo_entry_display() {
        let entry = RepoEntry {
            id: "123".to_string(),
            path: PathBuf::from("group-a/test.md"),
            change_kind: ChangeKind::Added,
            file_kind: FileKind::Markdown,
            timestamp: Local.with_ymd_and_hms(2024, 8, 22, 12, 30, 0).unwrap(),
        };

        let output = format!("{}", entry);
        assert!(output.contains("[md]"));
        assert!(output.contains("+"));
        assert!(output.contains("group-a/test.md"));
        assert!(output.contains("2024-08-22 12:30"));
    }
}
