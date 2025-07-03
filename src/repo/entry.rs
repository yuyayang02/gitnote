use std::{fmt, path::PathBuf};

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
