#!/bin/sh
set -eu
warn() { echo "warning: ${1}"; }

# --- 1. 创建 git 用户并固定 UID/GID ---
# 删除已存在用户和组（容器可能重启）
deluser "git" 2>/dev/null || true
delgroup "git" 2>/dev/null || true

# 创建组和用户
addgroup -g 1001 git
adduser --gecos 'Git User' --shell $(which git-shell) \
    --uid 1001 --ingroup git \
    --disabled-password \
    --no-create-home git


# 生成随机密码
echo "git:$(openssl rand -hex 16)" | chpasswd


# --- 2. 固定 SSH host key ---
# 检查 /etc/ssh 下 host key 是否存在
if ls /etc/ssh/ssh_host_* >/dev/null 2>&1; then
    echo "SSH host keys exist, skipping ssh-keygen -A"
else
    echo "Generating SSH host keys..."
    ssh-keygen -A
fi


# --- 3. 固定 authorized_keys ---
GIT_HOME=/home/git
if [ -f "${SSH_AUTHORIZED_KEYS_FILE}" ]; then
    mkdir -p "${GIT_HOME}/.ssh"
    cp "${SSH_AUTHORIZED_KEYS_FILE}" "${GIT_HOME}/.ssh/authorized_keys"
    chown -R "git:git" "${GIT_HOME}/.ssh"
    chmod 700 "${GIT_HOME}/.ssh"
    chmod 600 "${GIT_HOME}/.ssh/authorized_keys"
else
    warn "authorized_keys file not found"
fi


# --- 4. 初始化裸仓库 ---
REPO_NAME="${GIT_REPO_NAME}"

# 仓库路径固定在 git 用户目录下
GIT_REPO_PATH="${GIT_HOME}/${REPO_NAME}"

# 如果仓库不存在，则创建裸仓库
if [ ! -d "$GIT_REPO_PATH" ]; then
    echo "Repository not found, creating bare repository at $GIT_REPO_PATH..."
    mkdir -p "$GIT_REPO_PATH"
    git init -b main --bare "$GIT_REPO_PATH"
fi

# 设置权限
chown -R "git:git" "$GIT_REPO_PATH"
find "$GIT_REPO_PATH" -type f -exec chmod u=rwX,go=rX '{}' \;
find "$GIT_REPO_PATH" -type d -exec chmod u=rwx,go=rx '{}' \;

# 导出完整路径供后续使用
export GIT_REPO_PATH
echo "Repository full path: $GIT_REPO_PATH"



# --- 5. 限制 SSH 只用公钥登录 ---
SSHD_CONFIG_FILE='/etc/ssh/sshd_config'
cat >> "$SSHD_CONFIG_FILE" <<'EOF'
PubkeyAuthentication yes
PasswordAuthentication no
EOF


# --- 6. 为指定裸仓库添加 hook ---
HOOKS_DIR="/srv/hooks"
if [ -d "$GIT_REPO_PATH" ]; then
    HOOKS_DIR_REPO="$GIT_REPO_PATH/hooks"
    mkdir -p "$HOOKS_DIR_REPO"

    if [ -d "$HOOKS_DIR" ]; then
        for hook_script in "$HOOKS_DIR"/*; do
            [ -f "$hook_script" ] || continue
            hook_name=$(basename "$hook_script")

            # 确保公共 hook 可执行
            chmod +x "$hook_script"
            chown git:git "$hook_script"

            # 在仓库 hooks 中创建调用器
            cat > "$HOOKS_DIR_REPO/$hook_name" <<EOF
#!/bin/sh
exec "$HOOKS_DIR/$hook_name" "\$@"
EOF

            chmod +x "$HOOKS_DIR_REPO/$hook_name"
            chown git:git "$HOOKS_DIR_REPO/$hook_name"
        done
    else
        echo "warning: hooks directory $HOOKS_DIR does not exist"
    fi
fi



# --- 启动 SSH 服务 ---
# /usr/sbin/sshd 默认会以守护进程模式启动（除非使用 -D 前台模式）
/usr/sbin/sshd

# --- 执行容器 CMD 或传入的命令 ---
# exec 会用传入的命令替换当前 shell 进程
# "$@" 会将 CMD 的参数完整传入
exec "$@"