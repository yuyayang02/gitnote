# 📘 GitNote 内容管理机制说明

本项目提供一个以 Git 仓库为核心的自托管内容管理系统，通过对 Git 仓库结构与提交历史的抽象，实现文章、分组、配置等语义建模与内容追踪。

---

## 📁 内容结构约定

```text
<repo-root>/
├── <category>/            # 一个组
│   ├── .gitnote.toml      # 组配置（可选）
│   ├── first-post.md      # 文章文件
│   └── another.md
├── notes/                 # 另一个组
│   └── ...                    
```

---

## 🧠 内容单元：组 与 文章

### ✅ 组（Group）

- 对应目录
- 可选 `.gitnote.toml` 用于配置该目录下内容的分类、默认标签、可见性等

示例 `.gitnote.toml`：

```toml
public = true

[category]
id = "notes"
name = "笔记"

[author]
name = "xxx"
```

---

### ✅ 文章（File）

- Markdown 文件（如 `xxx.md`）
- 元信息写在 Front Matter（推荐使用 TOML 格式）

示例 markdown文件

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

## 🚧 Future Plans

- 重构 `rebuild` 功能为后台异步任务，支持任务状态跟踪和异步触发。
