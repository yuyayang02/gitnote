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

# # ====== 第二阶段：依赖分析 ======
FROM chef AS planner
# 复制项目文件（用于生成依赖清单）/可以不完全复制
COPY . .

# # 分析项目依赖并生成recipe.json
RUN cargo chef prepare --recipe-path recipe.json

# ====== 第三阶段：构建应用 ======
FROM chef AS builder

# 从planner阶段复制依赖清单
COPY --from=planner /app/recipe.json recipe.json

# 仅构建依赖项（利用Docker层缓存）
RUN cargo chef cook --release --recipe-path recipe.json

# 复制全部源代码（依赖变更后才会执行此层）
COPY . .

# 获取外部参数
ARG UPDATE_API
ENV UPDATE_API=$UPDATE_API

# 构建主应用（musl静态链接）
RUN cargo build --release --target x86_64-unknown-linux-musl --bin gitnote
RUN cargo build --release --target x86_64-unknown-linux-musl --bin update

# ====== 第四阶段：运行环境 ======
# 使用轻量Alpine运行时
FROM alpine:latest AS runtime
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/gitnote ./
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/update /tmp/update

COPY --from=builder /app/entrypoint.sh ./entrypoint.sh
RUN chmod +x ./entrypoint.sh

CMD ["./entrypoint.sh"]
