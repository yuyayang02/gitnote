# 📘 GitNote 内容管理机制说明

本项目提供一个以 Git 仓库为核心的自托管内容管理系统，通过对 Git 仓库结构与提交历史的抽象，实现文章、分组、配置等语义建模与内容追踪。

---

## 🛠️ 架构变更说明（2025-07-14）

GitNote 项目将 Git Server 功能拆分为独立项目：

- 原本内嵌于 Rust API 的 Git Server（基于 SSH 和裸仓库）已 **完全迁移** 到独立的仓库：[GitNote Git Server](https://github.com/yuyayang02/gitnote-git-server)。
- Rust API 不再负责容器内初始化 git 仓库、用户、公钥配置等逻辑，相关任务交由 Git Server 独立容器完成。
- 两者之间通过挂载共享卷（或 API 通信）协同工作。
- 原有仓库中的 `docker-compose.yml`、`sshd` 目录、`update` hook 等已删除。

🎯 **拆分目标**：解耦职责、便于维护、增强容器可组合性。

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

## 📂 Git 历史管理

本项目在需要对主分支历史进行大规模调整（例如替换 main 分支历史为精简的存档分支历史）时，采用了规范的操作流程。

具体操作步骤、注意事项、命令汇总详见： [docs/git-history-replace.md](docs/git-history-replace.md)

> 该文档说明了如何使用 archived/2025-Q1 分支替换 main 分支中指定 tag 及其之前历史，同时保留后续提交，保证 Git 仓库历史的整洁与安全。


## 🚧 Future Plans

- 重构 `rebuild` 功能为后台异步任务，支持任务状态跟踪和异步触发。
