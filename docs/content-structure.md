# GitNote 仓库内容结构规范

本文档定义 GitNote 系统在 Git 仓库层面的内容组织方式与文件格式约定。其目的是明确“组（Group）”与“文章（Article）”的文件结构与元数据格式，为系统的解析和行为逻辑提供统一的结构基础。

---

## 1. 仓库目录结构

GitNote 以 Git 仓库的目录层级作为内容语义模型的基础，每个目录代表一个组（Group），每个 Markdown 文件代表一篇文章（Article）。

```text
<root>/
├── <group-id>/            # 一个组
│   ├── .group.toml        # 组配置文件
│   ├── first-post.md      # 文章文件
│   └── another.md
├── notes/                 # 另一个组
│   └── ...
```

### 目录组织规则

1. 每个目录可视为一个组，组内可包含文章文件或子组。
2. `.group.toml` 是可选文件，用于定义该组的属性与元信息。
3. 顶层 Markdown 文件视为默认组的文章。
4. 命名建议使用小写字母、连字符或下划线，避免空格与特殊字符。

---

## 2. 组（Group）定义

### 2.1 概述

组（Group）是 GitNote 的内容分组单元，对应于一个目录。组的元信息存放在 `.group.toml` 文件中，用于描述分类、默认属性及扩展信息。

### 2.2 `.group.toml` 示例

```toml
public = true

[category]
id = "notes"
name = "笔记"

[author]
name = "张三"
```

### 2.3 字段说明

| 字段                | 类型  | 说明          |
| ----------------- | --- | ----------- |
| `public`          | 布尔值 | 是否公开该组内容    |
| `[category].id`   | 字符串 | 组的唯一标识符     |
| `[category].name` | 字符串 | 分类显示名称      |
| `[author].name`   | 字符串 | 默认作者，可被文章覆盖 |

---

## 3. 文章（Article）定义

### 3.1 概述

文章对应一个 Markdown 文件（如 `post.md`），使用 TOML 格式的 Front Matter 描述元信息。

### 3.2 示例结构

```markdown
+++
title = "我的第一篇博客"
summary = """
介绍 GitNote 博客系统的设计。
"""
tags = ["rust", "git"]
datetime = "2025-06-26"
+++

正文内容……
```

### 3.3 字段说明

| 字段         | 类型    | 说明      | 必须 |
| ---------- | ----- | ------- | - |
| `title`    | 字符串   | 文章标题    | ✅ |
| `summary`  | 多行字符串 | 简介或摘要   | ✅ |
| `tags`     | 数组    | 标签列表    | ✅ |
| `datetime` | 日期字符串 | 创建或修改时间 | ✅ |
