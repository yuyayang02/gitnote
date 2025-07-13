#!/bin/sh
set -eux

REPO_NAME="${REPO_NAME:?必须设置环境变量 REPO_NAME}"

SOURCE_PATH="/git-repo/$REPO_NAME"
TARGET_PATH="/app/$REPO_NAME"

if [ -d "$SOURCE_PATH" ]; then
  ln -sf "$SOURCE_PATH" "$TARGET_PATH"
  echo "🔗 已软链接 $SOURCE_PATH -> $TARGET_PATH"
else
  echo "❌ 裸仓库目录 $SOURCE_PATH 不存在！"
  exit 1
fi

HOOK_PATH="$TARGET_PATH/hooks/update"
if [ -f /tmp/update ]; then
  cp /tmp/update "$HOOK_PATH"
  chmod +x "$HOOK_PATH"
  echo "✅ 已安装 update hook"
else
  echo "⚠️ /tmp/update hook 脚本不存在，跳过"
fi

cd /app
exec ./gitnote