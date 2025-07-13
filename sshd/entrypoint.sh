#!/bin/sh
set -eux

GIT_USER="${GIT_USER:-git}"
GIT_USER_PASSWORD="${GIT_USER_PASSWORD:?必须设置环境变量 GIT_USER_PASSWORD}"
REPO_NAME="${REPO_NAME:?必须设置环境变量 REPO_NAME}"

REPO_PATH="/git-repo/${REPO_NAME}"
LINK_PATH="/home/${GIT_USER}/${REPO_NAME}"
SSH_DIR="/home/${GIT_USER}/.ssh"

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

# 初始化 Git 裸仓库（检查 HEAD）
if [ ! -f "$REPO_PATH/HEAD" ]; then
  echo "📦 初始化 Git 裸仓库：$REPO_PATH"
  git init --bare --initial-branch=main "$REPO_PATH"
  chown -R "$GIT_USER:$GIT_USER" "$REPO_PATH"
else
  echo "✅ 仓库已存在：$REPO_PATH"
fi

# 创建软链接到 /home/git/
if [ -e "$LINK_PATH" ] && [ ! -L "$LINK_PATH" ]; then
  echo "⚠️ 跳过软链接：$LINK_PATH 已存在但不是链接"
elif [ ! -L "$LINK_PATH" ]; then
  ln -sf "$REPO_PATH" "$LINK_PATH"
  echo "🔗 已创建软链接：$LINK_PATH -> $REPO_PATH"
fi

# 配置 authorized_keys
if [ -f /ssh_keys/authorized_keys ]; then
  cp /ssh_keys/authorized_keys "$SSH_DIR/authorized_keys"
  chmod 600 "$SSH_DIR/authorized_keys"
  chown "$GIT_USER:$GIT_USER" "$SSH_DIR/authorized_keys"
  echo "🔐 已更新 authorized_keys"
fi

# 生成 SSH host key（若不存在）
ssh-keygen -A

# 写入 sshd_config（简洁版）
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
