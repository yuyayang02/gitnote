use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

mod timeline;
mod wiki;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(tag = "type")]
pub enum GroupKind {
    // 普通组，默认类型
    #[serde(rename = "normal")]
    #[default]
    Normal,
    // #[serde(rename = "timeline")]
    // Timeline { timeline: timeline::TimelineOptions },

    // #[serde(rename = "wiki")]
    // Wiki { column: WikiOptions },
}

/// 表示一个分组（Group），包含名称和元信息。
///
/// [`Group`] 通常用于表示仓库或文件系统中的逻辑分组。
#[derive(Debug, Deserialize)]
pub struct Group {
    /// 分组名称，只会保留路径的父级部分。
    #[serde(skip)]
    pub id: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub public: bool,

    /// 分组的类型
    #[serde(flatten)]
    pub kind: Option<GroupKind>,
}

impl Group {
    pub fn new(id: impl AsRef<Path>, group_content: String) -> Result<Self> {
        let mut group = serde_yaml::from_str::<Group>(&group_content)?;

        let path = id.as_ref();
        let parent = path.parent().unwrap_or(path);
        group.id = parent.to_string_lossy().trim_matches('/').to_string();

        group
            .name
            .is_empty()
            .then(|| group.name = parent.file_name().unwrap().to_string_lossy().to_string());

        group.kind = Some(group.kind.unwrap_or_default());

        Ok(group)
    }

    pub fn empty(id: impl AsRef<Path>) -> Self {
        let path = id.as_ref();
        let parent = path.parent().unwrap_or(path);
        Self {
            id: parent.to_string_lossy().trim_matches('/').to_string(),
            public: Default::default(),
            name: Default::default(),
            kind: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_parsing_normal() {
        let yaml_content = r#"
              type: normal
              name: 我的分组
          "#;

        let path = std::path::Path::new("/path/to/.group.yaml");

        let group = Group::new(path, yaml_content.to_string()).unwrap();

        // 测试 id 是否正确提取
        assert_eq!(group.id, "path/to");

        // 测试 name 是否正确解析
        assert_eq!(group.name, "我的分组");
    }

    #[test]
    fn test_group_default_name() {
        let yaml_content = r#"
              type: normal
          "#;

        let path = std::path::Path::new("/path/to/.group.yaml");

        let group = Group::new(path, yaml_content.to_string()).unwrap();

        // id 应该提取父目录
        assert_eq!(group.id, "path/to");

        // name 为空时使用 id
        assert_eq!(group.name, "to");
    }

    #[test]
    fn test_empty_group() {
        let path = std::path::Path::new("/path/to/.group.toml");

        let group = Group::empty(path);

        assert_eq!(group.id, "path/to");
        assert_eq!(group.name, "");
    }
}
