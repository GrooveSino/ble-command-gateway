#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REMOTE="${YUNDRONE_INSTALL_REMOTE:-self-cloudserver}"
REMOTE_ROOT="${YUNDRONE_WEB_REMOTE_ROOT:-/var/www/tool.yundrone.cn/ble/current}"
VERSION="${VERSION:-$(tr -d '\r\n' <"$ROOT_DIR/VERSION")}"
DIST_DIR="$ROOT_DIR/web-client/dist"
ARCHIVE="$ROOT_DIR/dist/yundrone-web-ble-$VERSION.tar.gz"
SUDO_PASSWORD="${YUNDRONE_INSTALL_REMOTE_SUDO_PASSWORD:-}"

remote_sudo() {
  if [ -n "$SUDO_PASSWORD" ]; then
    ssh "$REMOTE" "printf '%s\n' '$SUDO_PASSWORD' | sudo -S $*"
  else
    ssh "$REMOTE" "sudo $*"
  fi
}

cd "$ROOT_DIR/web-client"
pnpm test
pnpm build

mkdir -p "$ROOT_DIR/dist"
COPYFILE_DISABLE=1 tar -C "$DIST_DIR" -czf "$ARCHIVE" .
scp "$ARCHIVE" "$REMOTE:/tmp/yundrone-web-ble.tar.gz"

remote_sudo "mkdir -p '$REMOTE_ROOT'"
remote_sudo "tar -C '$(dirname "$REMOTE_ROOT")' -czf '/tmp/tool-yundrone-ble-backup-$VERSION.tar.gz' '$(basename "$REMOTE_ROOT")'"
remote_sudo "rm -rf '$REMOTE_ROOT'"
remote_sudo "mkdir -p '$REMOTE_ROOT'"
remote_sudo "tar -C '$REMOTE_ROOT' -xzf /tmp/yundrone-web-ble.tar.gz"
remote_sudo "rm -f /tmp/yundrone-web-ble.tar.gz"

curl -fsSI "https://tool.yundrone.cn/ble/" >/dev/null
echo "deployed web client $VERSION to https://tool.yundrone.cn/ble/"
