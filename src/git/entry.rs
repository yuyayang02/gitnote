use std::{
    fmt,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, TimeZone};
use git2::{Commit, Diff};

/// 枚举表示文件的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// `.group.toml` 文件
    Group,
    /// Markdown 文件 (`.md` 或 `.markdown`)
    Markdown,
    /// 其他文件类型
    Other,
}

impl FileKind {
    /// 根据文件路径推断文件类型
    ///
    /// 文件类型判断规则：
    /// - 文件名为 `.group.toml` 返回 [`FileKind::Group`]
    /// - 扩展名为 `.md` 或 `.markdown` 返回 [`FileKind::Markdown`]
    /// - 其他情况返回 [`FileKind::Other`]
    ///
    pub(super) fn from_path(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name == ".group.toml" {
                return FileKind::Group;
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
    /// 更改
    Modified,
    /// 删除
    Deleted,
}

/// 表示 Git 仓库中一次文件或目录的变更。
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
pub(super) trait IntoRepoEntry {
    /// 将类型转换为 [`RepoEntry`] 列表。
    fn into_entry(self) -> Vec<RepoEntry>;
}

impl<'a> IntoRepoEntry for (Diff<'a>, Commit<'a>) {
    fn into_entry(self) -> Vec<RepoEntry> {
        let (diff, commit) = self;
        let timestamp = Local.timestamp_opt(commit.time().seconds(), 0).unwrap();

        diff.deltas()
            .filter_map(|d| {
                let (file, change_kind) = match d.status() {
                    git2::Delta::Added | git2::Delta::Copied => (d.new_file(), ChangeKind::Added),
                    git2::Delta::Deleted => (d.old_file(), ChangeKind::Deleted),
                    git2::Delta::Modified => (d.new_file(), ChangeKind::Modified),
                    git2::Delta::Renamed => (d.new_file(), ChangeKind::Modified), // 重命名也可以算作修改
                    _ => return None,
                };

                let path = file.path()?;
                Some(RepoEntry {
                    id: file.id().to_string(),
                    path: path.to_path_buf(),
                    change_kind,
                    file_kind: FileKind::from_path(path),
                    timestamp,
                })
            })
            .collect()
    }
}

/// 合并旧的变更状态和新的变更状态，返回合并后的结果。
///
/// 合并规则：
/// — [`ChangeKind::Added`] -> [`ChangeKind::Deleted`] = 消失 (返回 `None`)
/// — [`ChangeKind::Deleted`] -> [`ChangeKind::Added`] = [`ChangeKind::Added`]
/// — 新增后修改 (`Added` -> `Modified`) = [`ChangeKind::Modified`]
/// — 其他情况保持最新状态或按逻辑覆盖
fn merge_change(old: Option<&ChangeKind>, new: ChangeKind) -> Option<ChangeKind> {
    match (old, new) {
        (None, now) => Some(now),
        (Some(ChangeKind::Deleted), new) => Some(new),
        (Some(ChangeKind::Added), ChangeKind::Deleted) => None,
        (Some(ChangeKind::Added), _) => Some(ChangeKind::Modified),
        (Some(ChangeKind::Modified), ChangeKind::Deleted) => Some(ChangeKind::Deleted),
        (Some(ChangeKind::Modified), _) => Some(ChangeKind::Modified),
    }
}

/// 定义对一组 [`RepoEntry`] 进行裁剪的行为。
///
/// 用于在序列中合并或抵消重复的文件变更，得到精简后的最终结果。
pub trait RepoEntryPrune {
    fn prune(self) -> Vec<RepoEntry>;
}

impl RepoEntryPrune for Vec<RepoEntry> {
    /// 对变更序列进行裁剪，合并同一路径的连续修改，去掉无效的抵消操作。
    ///
    /// 返回的结果只包含必要的文件变更，便于后续处理。
    fn prune(self) -> Vec<RepoEntry> {
        use std::collections::HashMap;

        let mut state: HashMap<std::path::PathBuf, usize> = HashMap::new();
        let mut result: Vec<Option<RepoEntry>> = (0..self.len()).map(|_| None).collect();

        for (idx, mut entry) in self.into_iter().enumerate() {
            let path = entry.path.clone();
            match merge_change(
                state
                    .get(&path)
                    .and_then(|&i| result[i].as_ref().map(|e: &RepoEntry| &e.change_kind)),
                entry.change_kind,
            ) {
                Some(real_change) => {
                    entry.change_kind = real_change;
                    if let Some(prev_idx) = state.insert(path, idx) {
                        result[prev_idx] = None;
                    }
                    result[idx] = Some(entry);
                }
                None => {
                    if let Some(prev_idx) = state.remove(&path) {
                        result[prev_idx] = None;
                    }
                }
            }
        }

        result.into_iter().flatten().collect()
    }
}

impl fmt::Display for RepoEntry {
    /// 格式化 [`RepoEntry`] 为字符串，用于 CLI 或日志输出。
    ///
    /// 格式示例：
    /// ```text
    /// [md]    + group-a/test.md @ 2024-08-22 12:30
    /// [group] + .group.toml @ 2024-08-22 12:35
    /// ```
    ///
    /// 枚举跳转：
    /// - [`FileKind::Group`]
    /// - [`FileKind::Markdown`]
    /// - [`FileKind::Other`]
    /// - [`ChangeKind::Added`]
    /// - [`ChangeKind::Deleted`]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_str = match self.file_kind {
            FileKind::Group => "[group]",
            FileKind::Markdown => "[md]",
            FileKind::Other => "[-]",
        };

        let change_str = match self.change_kind {
            ChangeKind::Added => "+",
            ChangeKind::Deleted => "-",
            ChangeKind::Modified => "~",
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
        if self.is_empty() {
            return "No entries".to_string();
        }

        // 按时间从老到新排序
        // self.sort_by_key(|e| e.timestamp);
        // 转换为多行字符串
        self.into_iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use std::path::PathBuf;

    #[test]
    fn test_file_kind_from_path() {
        assert_eq!(FileKind::from_path(".group.toml"), FileKind::Group);
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
        assert_eq!(merge_change(None, Modified), Some(Modified));

        // Added -> Added / Modified / Deleted
        assert_eq!(merge_change(Some(&Added), Added), Some(Modified));
        assert_eq!(merge_change(Some(&Added), Modified), Some(Modified));
        assert_eq!(merge_change(Some(&Added), Deleted), None);

        // Deleted -> Added / Deleted / Modified
        assert_eq!(merge_change(Some(&Deleted), Added), Some(Added));
        assert_eq!(merge_change(Some(&Deleted), Deleted), Some(Deleted));
        assert_eq!(merge_change(Some(&Deleted), Modified), Some(Modified));

        // Modified -> Added / Deleted / Modified
        assert_eq!(merge_change(Some(&Modified), Added), Some(Modified));
        assert_eq!(merge_change(Some(&Modified), Deleted), Some(Deleted));
        assert_eq!(merge_change(Some(&Modified), Modified), Some(Modified));
    }

    #[test]
    fn test_repo_entry_display() {
        let entry_added = RepoEntry {
            id: "123".to_string(),
            path: PathBuf::from("group-a/test.md"),
            change_kind: ChangeKind::Added,
            file_kind: FileKind::Markdown,
            timestamp: Local.with_ymd_and_hms(2024, 8, 22, 12, 30, 0).unwrap(),
        };

        let entry_modified = RepoEntry {
            id: "124".to_string(),
            path: PathBuf::from("group-a/updated.md"),
            change_kind: ChangeKind::Modified,
            file_kind: FileKind::Markdown,
            timestamp: Local.with_ymd_and_hms(2024, 8, 22, 12, 35, 0).unwrap(),
        };

        let output_added = format!("{}", entry_added);
        assert!(output_added.contains("[md]"));
        assert!(output_added.contains("+"));
        assert!(output_added.contains("group-a/test.md"));
        assert!(output_added.contains("2024-08-22 12:30"));

        let output_modified = format!("{}", entry_modified);
        assert!(output_modified.contains("[md]"));
        assert!(output_modified.contains("~")); // Modified 使用 ~ 符号
        assert!(output_modified.contains("group-a/updated.md"));
        assert!(output_modified.contains("2024-08-22 12:35"));
    }
}
