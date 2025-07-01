use crate::error::Result;

use serde::{Deserialize, Serialize};

pub struct Group {
    pub name: String,
    pub meta: GroupMeta,
}

impl Group {
    pub fn new(name: impl Into<String>, gitnote_content: String) -> Result<Self> {
        let meta = toml::from_str(&gitnote_content)?;

        Ok(Self {
            name: name.into(),
            meta,
        })
    }

    pub fn new_with_meta(name: impl Into<String>, meta: GroupMeta) -> Self {
        Self {
            name: name.into(),
            meta,
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
