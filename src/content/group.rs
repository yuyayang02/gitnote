use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// 表示一个分组（Group），包含名称和元信息。
///
/// [`Group`] 通常用于表示仓库或文件系统中的逻辑分组。
pub struct Group {
    /// 分组名称，只会保留路径的父级部分。
    pub name: String,
    /// 分组的元信息
    pub meta: GroupMeta,
}

impl Group {
    /// 基于名称路径和 GitNote 内容创建新的 [`Group`]。
    ///
    /// 路径仅取父级目录作为分组名称，子级路径会被忽略。
    /// `gitnote_content` 会解析为 [`GroupMeta`]。
    ///
    pub fn new(name: impl AsRef<Path>, gitnote_content: String) -> Result<Self> {
        let meta = toml::from_str(&gitnote_content)?;

        Ok(Self {
            name: Self::extract_group_name(name.as_ref()),
            meta,
        })
    }

    /// 创建一个空分组，元信息为默认值。
    ///
    /// 仅设置分组名称，同样只保留父级路径。
    pub fn empty(name: impl AsRef<Path>) -> Self {
        Self {
            name: Self::extract_group_name(name.as_ref()),
            meta: GroupMeta::default(),
        }
    }

    /// 从路径中提取分组名称，仅保留父级目录。
    ///
    /// 去掉路径前后的 `/`，子级路径会被忽略。
    fn extract_group_name(path: &Path) -> String {
        let parent = path.parent().unwrap_or(path);
        parent.to_string_lossy().trim_matches('/').to_string()
    }
}

#[derive(Deserialize, Serialize)]
pub struct GroupCategory {
    /// 分类id，一般是这个组路径的最后一部分
    id: String,
    /// 分类名，一般是中文，用于展示
    name: String,
}

#[derive(Deserialize, Serialize)]
pub struct GroupAuthor {
    /// 组的所有者，仅起显示作用
    pub name: String,
}

#[derive(Default, Deserialize, Serialize)]
pub struct GroupMeta {
    pub public: bool, // 默认不公开
    pub category: Option<GroupCategory>,
    pub author: Option<GroupAuthor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_new_parses_meta() {
        let toml_content = r#"
            public = true
            [category]
            id = "rust"
            name = "Rust语言"
            [author]
            name = "Alice"
        "#;

        let group =
            Group::new("group-a/xxx", toml_content.to_string()).expect("Failed to parse group");

        assert_eq!(group.name, "group-a");
        assert!(group.meta.public);
        assert!(group.meta.category.is_some());
        let category = group.meta.category.unwrap();
        assert_eq!(category.id, "rust");
        assert_eq!(category.name, "Rust语言");

        assert!(group.meta.author.is_some());
        let author = group.meta.author.unwrap();
        assert_eq!(author.name, "Alice");
    }

    #[test]
    fn test_group_empty_uses_default_meta() {
        let group = Group::empty("aaa/aaa");

        assert_eq!(group.name, "aaa");
        assert!(!group.meta.public); // 默认 false
        assert!(group.meta.category.is_none());
        assert!(group.meta.author.is_none());
    }

    #[test]
    fn test_group_meta_serde() {
        let meta = GroupMeta {
            public: true,
            category: Some(GroupCategory {
                id: "rust".to_string(),
                name: "Rust语言".to_string(),
            }),
            author: Some(GroupAuthor {
                name: "Alice".to_string(),
            }),
        };

        // Serialize
        let serialized = toml::to_string(&meta).expect("Failed to serialize");
        assert!(serialized.contains("public"));
        assert!(serialized.contains("category"));
        assert!(serialized.contains("author"));

        // Deserialize
        let deserialized: GroupMeta = toml::from_str(&serialized).expect("Failed to deserialize");
        assert!(deserialized.public);
        assert_eq!(deserialized.category.unwrap().id, "rust");
        assert_eq!(deserialized.author.unwrap().name, "Alice");
    }

    #[test]
    fn test_extract_group_name_various_cases() {
        let cases = [
            // (输入路径, 期望结果)
            ("foo/bar/.group.toml", "foo/bar"),
            ("foo/bar/baz.txt", "foo/bar"),
            ("foo/bar/", "foo"),
            ("foo/", ""),
            ("/", ""),
            ("./foo/bar/file.txt", "./foo/bar"),
        ];

        for (input, expected) in cases {
            let path = Path::new(input);
            let name = Group::extract_group_name(path);
            assert_eq!(name, expected, "failed for path: {}", input);
        }
    }
}
