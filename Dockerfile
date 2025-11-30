# ====== 第一阶段：构建环境准备 ======
# 使用带musl工具链的Rust基础镜像
FROM clux/muslrust:1.85.0-stable-2025-03-18 AS chef

# 设置工作目录
WORKDIR /app

# 配置Cargo国内镜像源加速下载
RUN cat <<EOF > /root/.cargo/config.toml
[source.crates-io]
replace-with = "rsproxy-sparse"  # 使用稀疏索引加速

[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"  # 推荐稀疏协议

[registries.rsproxy]
index = "https://rsproxy.cn/crates.io-index"

[net]
git-fetch-with-cli = true  # 强制使用CLI处理git

# 静态编译配置
[target.x86_64-unknown-linux-musl]
rustflags = ["-C", "target-feature=+crt-static"]
EOF

# 安装cargo-chef构建工具（缓存依赖加速构建）
RUN cargo install cargo-chef --locked

# 设置静态链接环境变量
ENV PKG_CONFIG_ALL_STATIC=1
ENV OPENSSL_STATIC=1

# ====== 第二阶段：依赖分析 ======
FROM chef AS planner
# 复制项目文件（用于生成依赖清单）/可以不完全复制
COPY . .

# 分析项目依赖并生成recipe.json
RUN cargo chef prepare --recipe-path recipe.json

# ====== 第三阶段：构建应用 ======
FROM chef AS builder

# 从planner阶段复制依赖清单
COPY --from=planner /app/recipe.json recipe.json

# 仅构建依赖项（利用Docker层缓存）
RUN cargo chef cook --release --recipe-path recipe.json

# 复制全部源代码（依赖变更后才会执行此层）
COPY . .

ENV REPO_PATH=/home/git/repo.git
# 构建主应用（musl静态链接）
RUN cargo build --release --target x86_64-unknown-linux-musl


# ====== 第四阶段：运行环境 ======
# 使用自定义git-ssh镜像运行时
FROM gitnote-ssh:latest AS runtime

WORKDIR /app

RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.aliyun.com/g' /etc/apk/repositories \
    && apk update

# 安装应用依赖（如果有额外需求）
RUN apk add --no-cache \
    tzdata \
    bash \
    curl \
    openssl

# 设置时区
RUN cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime \
    && echo "Asia/Shanghai" > /etc/timezone
ENV TZ=Asia/Shanghai

# 使用 Rust 多阶段构建的二进制文件
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/gitnote ./

# 暴露端口（如果 Git SSH 已在基底镜像中暴露可选）
EXPOSE 22
EXPOSE 3000

# CMD 可以作为默认参数传入 Rust 程序
# 例如默认启动 gitnote，用户也可以覆盖
CMD ["sh", "-c", "/usr/sbin/sshd & exec /app/gitnote"]
