#!/bin/sh
set -eux

GIT_USER="${GIT_USER:-git}"
GIT_USER_PASSWORD="${GIT_USER_PASSWORD:?必须设置环境变量 GIT_USER_PASSWORD}"
REPO_NAME="${REPO_NAME:?必须设置环境变量 REPO_NAME}"
UPDATE_API="${UPDATE_API:?必须设置环境变量 UPDATE_API，用于 Git update hook}"


REPO_PATH="/git-repo/${REPO_NAME}"
LINK_PATH="/home/${GIT_USER}/${REPO_NAME}"
SSH_DIR="/home/${GIT_USER}/.ssh"
HOOK_PATH="${REPO_PATH}/hooks/update"

# 创建 git 用户（如果不存在）
if ! id "$GIT_USER" >/dev/null 2>&1; then
  echo "🔧 创建用户 $GIT_USER"
  adduser -D -s /bin/sh "$GIT_USER"
  echo "${GIT_USER}:${GIT_USER_PASSWORD}" | chpasswd
  mkdir -p "$SSH_DIR"
  touch "$SSH_DIR/authorized_keys"
  chmod 700 "$SSH_DIR"
  chmod 600 "$SSH_DIR/authorized_keys"
  chown -R "$GIT_USER:$GIT_USER" "/home/${GIT_USER}"
fi

# 初始化 Git 裸仓库
if [ ! -f "$REPO_PATH/HEAD" ]; then
  echo "📦 初始化 Git 裸仓库：$REPO_PATH"
  git init --bare --initial-branch=main "$REPO_PATH"
  chown -R "$GIT_USER:$GIT_USER" "$REPO_PATH"
else
  echo "✅ 仓库已存在：$REPO_PATH"
fi

# 添加 update hook（软链接方式）
if [ ! -L "$HOOK_PATH" ]; then
  echo "🔗 添加 update hook 到 $HOOK_PATH"
  ln -sf /update "$HOOK_PATH"
  chown -h "$GIT_USER:$GIT_USER" "$HOOK_PATH"
else
  echo "✅ update hook 已链接"
fi

# 进入仓库目录以设置其特定的配置
(
  # 切换到 git 用户执行 git config，确保权限和所有权正确
  su - "$GIT_USER" -c "
    cd '$REPO_PATH'
    echo '⚙️ 设置 Git 配置 hooks.updateapi 为: $UPDATE_API'
    git config hooks.updateapi '$UPDATE_API'
  "
)

# 创建软链接到 /home/git/
if [ -e "$LINK_PATH" ] && [ ! -L "$LINK_PATH" ]; then
  echo "⚠️ 跳过软链接：$LINK_PATH 已存在但不是链接"
elif [ ! -L "$LINK_PATH" ]; then
  ln -sf "$REPO_PATH" "$LINK_PATH"
  chown -h "$GIT_USER:$GIT_USER" "$LINK_PATH"
  echo "🔗 已创建软链接：$LINK_PATH -> $REPO_PATH"
fi

# 配置 authorized_keys
if [ -f /ssh_keys/authorized_keys ]; then
  cp /ssh_keys/authorized_keys "$SSH_DIR/authorized_keys"
  chmod 600 "$SSH_DIR/authorized_keys"
  chown "$GIT_USER:$GIT_USER" "$SSH_DIR/authorized_keys"
  echo "🔐 已更新 authorized_keys"
fi

# ✅ 仅在 host key 不存在时生成
if [ ! -f /etc/ssh/ssh_host_rsa_key ]; then
  echo "🔑 首次生成 SSH host keys"
  ssh-keygen -A
else
  echo "✅ SSH host keys 已存在"
fi

# 写入 sshd_config（可扩展）
cat <<EOF > /etc/ssh/sshd_config
Port 22
PermitRootLogin no
PasswordAuthentication yes
PubkeyAuthentication yes
PermitEmptyPasswords no
ChallengeResponseAuthentication no
Subsystem sftp /usr/lib/ssh/sftp-server
EOF

echo "🚀 启动 sshd ..."
exec /usr/sbin/sshd -D -e
