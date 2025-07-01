#!/bin/sh
set -eux

GIT_USER="${GIT_USER:-git}"
GIT_USER_PASSWORD="${GIT_USER_PASSWORD:?必须设置环境变量 GIT_USER_PASSWORD}"
REPO_NAME="${REPO_NAME:?必须设置环境变量 REPO_NAME}"

# 创建用户（如果不存在）
if ! id "$GIT_USER" >/dev/null 2>&1; then
  adduser -D -s /bin/sh "$GIT_USER"
  echo "${GIT_USER}:${GIT_USER_PASSWORD}" | chpasswd
  chmod 755 "/home/${GIT_USER}"
else
  echo "User $GIT_USER already exists, skipping creation."
fi


# 移动主程序到 git 用户主目录
mv /app/gitnote "/home/${GIT_USER}/gitnote"
chown "${GIT_USER}:${GIT_USER}" "/home/${GIT_USER}/gitnote"
chmod +x "/home/${GIT_USER}/gitnote"

# 初始化裸仓库（如果不存在）
REPO_PATH="/home/${GIT_USER}/${REPO_NAME}"
if [ ! -d "$REPO_PATH" ]; then
  git init --bare "$REPO_PATH"
  chown -R "${GIT_USER}:${GIT_USER}" "$REPO_PATH"
else
  echo "Repository $REPO_PATH already exists, skipping initialization."
fi

# 安装 update hook（假设update文件已COPY到 /tmp/update）
HOOK_PATH="$REPO_PATH/hooks/update"
if [ -f /tmp/update ]; then
  cp /tmp/update "$HOOK_PATH"
  chmod +x "$HOOK_PATH"
  chown "${GIT_USER}:${GIT_USER}" "$HOOK_PATH"
else
  echo "Warning: /tmp/update hook script not found."
fi

# 处理 SSH 目录和 authorized_keys
SSH_DIR="/home/${GIT_USER}/.ssh"
mkdir -p "$SSH_DIR"

if [ -f /ssh_keys/authorized_keys ]; then
  cp /ssh_keys/authorized_keys "$SSH_DIR/authorized_keys"
else
  touch "$SSH_DIR/authorized_keys"
fi

chmod 700 "$SSH_DIR"
chmod 600 "$SSH_DIR/authorized_keys"
chown -R "${GIT_USER}:${GIT_USER}" "$SSH_DIR"

# 生成 SSH host keys（如果不存在）
if [ ! -f /etc/ssh/ssh_host_ed25519_key ]; then
  ssh-keygen -A
fi

# 写入 sshd_config
cat <<EOF > /etc/ssh/sshd_config
PasswordAuthentication yes
PubkeyAuthentication yes
PermitEmptyPasswords no
ChallengeResponseAuthentication no
EOF

cd "/home/${GIT_USER}"

# 启动 sshd（后台）
/usr/sbin/sshd -D -e &

exec su -s /bin/sh "$GIT_USER" -c "./gitnote"