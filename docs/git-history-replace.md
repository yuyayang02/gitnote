# Git 分支历史替换操作指南（基于标签节点）

## 操作目的

使用 `archived/2025-Q1` 分支的历史替换 `main` 分支中 `archive/2025-Q1` 标签所指 commit 及其之前的历史，保留标签之后的提交。

> **注意：本操作会重写 `main` 分支历史，请谨慎操作！**

### 自动存档说明

当你将 `archive/*` (`archive/2025-Q1`) 标签推送到远程仓库（如 GitHub 或 Gitee），服务端会自动生成存档分支 `archived/2025-Q1`。你需要先将其拉取下来：

```bash
git checkout -b archived/2025-Q1 origin/archived/2025-Q1
```
---

## 操作前准备

在开始执行操作前，请确保你具备以下三项：

* ✅ 一个主分支 `main`
* ✅ 一个存档标签，例如：`archive/2025-Q1`
* ✅ 一个已生成的存档分支，例如：`archived/2025-Q1`


---

## 操作步骤

### 1. 确认当前所在分支

确认自己正处于 `main` 分支：

```bash
git status
```

如果不在，切换到 `main`：

```bash
git checkout main
```

---

### 2. 确认分界 tag 信息

查看 `archive/2025-Q1` 标签指向的 commit ID：

```bash
git show archive/2025-Q1
```

查看标签之后的所有提交（将被保留）：

```bash
git log --oneline archive/2025-Q1..main
```
> [!IMPORTANT]
> 确保这些是你想保留的内容，避免误删重要提交！

---

### 3. 备份 main 分支

为防操作失败，先备份当前 `main` 分支：

```bash
git branch backup-main
```

---

### 4. 重置 main 到存档分支

```bash
git reset --hard archived/2025-Q1
```
> [!WARNING]
> ⚠️ 此操作会使 `main` 指向 `archived/2025-Q1` 最新提交，并删除标签之前历史。（当然也有修复方法，但是尽量不要错）

---

### 5. 把标签之后的提交 cherry-pick 迁移到新 main 历史

```bash
git cherry-pick archive/2025-Q1..backup-main
```

> ✅ 这一步将标签之后的变更迁移到简洁后的主分支。

---

### 6. 强制推送 main 到远程仓库

```bash
git push --force origin main
```
> [!CAUTION]
> ⚠️ 警告：该操作会重写远程历史，请确保所有协作者知情并暂停基于旧历史开发。

---

## 附录：实操命令汇总

```bash
git fetch origin

# 拉取存档分支
git checkout -b archived/2025-Q1 origin/archived/2025-Q1

# 切换到 main 并检查当前状态
git checkout main
git show archive/2025-Q1
git log --oneline archive/2025-Q1..main

# 备份 main
git branch backup-main

# 替换历史
git reset --hard archived/2025-Q1
git cherry-pick archive/2025-Q1..backup-main

# 推送更改
git push --force origin main
```

---

## 总结

总之大概就是这么个流程、有什么不懂的可以问AI，git高手可以按照自己的喜好来合并归档分支
