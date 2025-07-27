use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
};

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

/// 过滤并移除成对出现的“添加后删除”的变更记录。
///
/// 对于同一个文件或 GitNote，若出现了先添加（File / GitNote），后删除（RemoveFile / RemoveGitNote）的
/// 操作，会将这对操作都从结果中剔除，保留最终未被删除的添加操作。
///
/// # 参数
/// - `entries`：输入的变更记录列表，按时间顺序排列。
///
/// # 返回值
/// - 返回过滤后的变更记录列表，其中所有成对的“添加-删除”操作已被移除。
///
/// # 具体逻辑
/// 1. 使用两个 HashMap 分别跟踪文件和 GitNote 的“添加”操作的索引位置（栈结构，后进先出）。
/// 2. 遍历输入的变更记录：
///    - 遇到添加操作（File / GitNote），将索引压入对应的栈。
///    - 遇到删除操作（RemoveFile / RemoveGitNote），尝试从对应栈中弹出最近的添加索引，
///      若成功，表示找到了一对“添加-删除”，将二者标记为不保留。
///    - 其他操作不影响栈结构，直接保留。
/// 3. 遍历结束后，将所有未被配对移除的变更保留返回。
pub fn strip_add_then_remove(entries: Vec<RepoEntry>) -> Vec<RepoEntry> {
    // 若条目太少，直接返回，无需处理
    if entries.len() < 2 {
        return entries;
    }

    // HashMap 键为 (目录路径, 文件名) 或目录路径，用于快速查找相同文件或笔记的“添加”操作位置。
    // 值为栈，存储该文件或笔记对应添加操作的索引。
    let mut file_stack: HashMap<(&Path, &str), Vec<usize>> = HashMap::new();
    let mut gitnote_stack: HashMap<&Path, Vec<usize>> = HashMap::new();

    // 与输入 entries 长度相同的布尔数组，标记对应条目是否保留
    let mut keep = vec![true; entries.len()];

    for (i, entry) in entries.iter().enumerate() {
        match entry {
            // 文件添加，压入 file_stack
            RepoEntry::File { group, name, .. } => {
                let key = (group.as_path(), name.as_str());
                file_stack.entry(key).or_default().push(i);
            }
            // 文件删除，尝试弹出最近的对应添加索引，找到即配对删除
            RepoEntry::RemoveFile { group, name } => {
                let key = (group.as_path(), name.as_str());
                if let Some(stack) = file_stack.get_mut(&key) {
                    if let Some(j) = stack.pop() {
                        keep[j] = false; // 移除对应添加操作
                        keep[i] = false; // 移除当前删除操作
                    }
                }
            }
            // GitNote 添加，压入 gitnote_stack
            RepoEntry::GitNote { group, .. } => {
                let key = group.as_path();
                gitnote_stack.entry(key).or_default().push(i);
            }
            // GitNote 删除，尝试弹出最近对应添加索引，找到即配对删除
            RepoEntry::RemoveGitNote { group } => {
                let key = group.as_path();
                if let Some(stack) = gitnote_stack.get_mut(&key) {
                    if let Some(j) = stack.pop() {
                        keep[j] = false; // 移除对应添加操作
                        keep[i] = false; // 移除当前删除操作
                    }
                }
            }
        }
    }

    // 根据 keep 标记过滤原始条目，返回最终结果
    entries
        .into_iter()
        .zip(keep)
        .filter_map(|(entry, k)| if k { Some(entry) } else { None })
        .collect()
}

impl fmt::Display for RepoEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoEntry::GitNote { group, content } => {
                write!(
                    f,
                    "[group] + {}.gitnote.toml  ({} lines, {} bytes)",
                    format!("{}/", group.display()),
                    content.lines().count(),
                    format_with_commas(content.len())
                )
            }
            RepoEntry::RemoveGitNote { group } => {
                write!(f, "[group] - {}/", group.display())
            }
            RepoEntry::File {
                group,
                name,
                datetime,
                content,
            } => {
                write!(
                    f,
                    " [file] + {}/{} @ {}  ({} lines, {} bytes)",
                    group.display(),
                    name,
                    datetime.format("%Y-%m-%d %H:%M"),
                    content.lines().count(),
                    format_with_commas(content.len())
                )
            }
            RepoEntry::RemoveFile { group, name } => {
                write!(f, " [file] - {}/{}", group.display(), name)
            }
        }
    }
}

/// 给数字添加千分位分隔符，比如 1234567 -> "1,234,567"
fn format_with_commas(n: usize) -> String {
    // 先把数字转成字符串
    let s = n.to_string();

    // 预先分配一个容量，稍微大于原字符串长度，减少内存重分配
    // 最多每3个数字加一个逗号，所以容量预留 len + len/3
    let mut result = String::with_capacity(s.len() + s.len() / 3);

    // count 用来记录已经插入多少个数字字符
    let mut count = 0;

    // 从字符串尾部开始遍历字符（即数字的个位开始）
    for c in s.chars().rev() {
        // 每3个数字字符插入一个逗号，第一次不插入
        if count != 0 && count % 3 == 0 {
            result.push(',');
        }
        // 把当前字符插入结果字符串
        result.push(c);
        count += 1;
    }

    // 当前字符串是倒序的，最后反转成正序返回
    result.chars().rev().collect()
}
