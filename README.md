# GitNote

本项目提供一个以 Git 仓库为核心的自托管内容管理系统，通过对 Git 仓库结构与提交历史的抽象，实现文章、分组、配置等语义建模与内容追踪。

---


## 内容编辑规范

使用自定义的格式规范，可扩展，可自定义。详见[content-structure.md](./docs/content-structure.md)

## Git 仓库

GitNote 使用 Git 仓库作为内容源。通过环境变量 `GIT_REPO_PATH` 指定仓库路径，所有文章和分组配置都存储在其中。

当用户推送内容时，Git Hook 会触发同步，将变更信息发送到系统。系统根据分支或标签类型决定是增量同步还是全量重建，然后解析变更文件并更新数据库或存储。

特点：
- 仓库即内容模型，所有操作由 Git 提交驱动  
- 支持增量同步和全量重建
- 解析文件后可直接生成系统行为和 HTML 内容


## 部署

GitNote 提供基于 Docker 的一键运行环境，内置 Git 服务与内容同步机制。

### 构建镜像

在项目根目录执行：

```bash
docker build -t gitnote:v1 .
```

### 使用 docker-compose 启动

示例配置：

```yaml
gitnote:
  image: gitnote:v1
  container_name: gitnote-api
  build:
    context: "./gitnote"
    dockerfile: "Dockerfile"
  ports:
    - "4000:3000"   # 应用接口
    - "4022:22"     # SSH 访问端口
  environment:
    - GITNOTE_LOG=gitnote=info,tower_http=info # 日志级别控制
    - DATABASE_URL=<db_url> # 数据库连接字符串
    - GITHUB_MARKDOWN_RENDER_KEY=<your_github_token> # GitHub Markdown 渲染 token
    - TZ=Asia/Shanghai # 容器时区设置
  volumes:
    - ssh_host_keys:/etc/ssh     # SSH 主机密钥，用于保存主机信息，防止重新构建导致的客户端信任失效
```

容器启动后会自动：

* 创建裸仓库 `/home/git/gitnote.git`
* 初始化 SSH 服务并加载公钥
* 安装并启用钩子脚本，实现内容同步
