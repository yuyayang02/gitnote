use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};

/// 表示 Git 仓库中的一次变更操作。
///
/// `RepoEntry` 用于统一描述文件或 GitNote 的添加、删除操作，
/// 可用于构建归档记录、生成提交日志等。
#[derive(Debug)]
pub enum RepoEntry {
    /// 表示某个目录下新增了一个 GitNote 文件（以 `.gitnote.toml` 命名）
    GitNote { group: PathBuf, content: String },
    /// 表示某个目录下删除了 GitNote 文件
    RemoveGitNote { group: PathBuf },
    /// 表示新增了一个普通文件（通常是 `.md` 文件）
    File {
        group: PathBuf,
        name: String,
        datetime: DateTime<Local>,
        content: String,
    },
    /// 表示删除了一个普通文件
    RemoveFile { group: PathBuf, name: String },
}

/// 对变更记录进行清洗：移除先添加后删除的重复变更。
///
/// 在某些场景下（如 rebase 或 squash 后），同一个文件在多个 commit 中被添加然后又立即删除，
/// 这类中间操作是冗余的，对最终状态没有影响。该函数将这类成对出现的“添加-删除”操作剔除。
///
/// 用于简化归档数据，保留对最终状态真正有意义的变更。
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

/// 实现 RepoEntry 的字符串展示逻辑。
///
/// 用于生成归档日志或 CLI 打印时的格式化输出。
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

/// 给数字加上千分位分隔符，提升可读性。
///
/// 示例：`1234567` -> `"1,234,567"`
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
