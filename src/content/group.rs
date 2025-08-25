use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

pub struct Group {
    pub name: String,
    pub meta: GroupMeta,
}

impl Group {
    pub fn new(name: impl AsRef<Path>, gitnote_content: String) -> Result<Self> {
        let meta = toml::from_str(&gitnote_content)?;

        Ok(Self {
            name: name.as_ref().to_string_lossy().to_string(),
            meta,
        })
    }

    pub fn empty(name: impl AsRef<Path>) -> Self {
        Self {
            name: name.as_ref().to_string_lossy().to_string(),
            meta: GroupMeta::default(),
        }
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

        let group = Group::new("group-a", toml_content.to_string()).expect("Failed to parse group");

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
        let group = Group::empty("group-b");

        assert_eq!(group.name, "group-b");
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
}
