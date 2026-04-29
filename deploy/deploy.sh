#!/usr/bin/env bash
set -euo pipefail

DROPLET_HOST="${DROPLET_HOST:-root@104.248.8.153}"
REMOTE_DIR="/opt/racquet"
TARGET="x86_64-unknown-linux-musl"
BIN_NAME="racquet"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> Building frontend"
(cd frontend && npm ci && npm run build)

echo "==> Building backend ($TARGET)"
cargo build --release --target "$TARGET"

BIN_PATH="target/$TARGET/release/$BIN_NAME"
test -x "$BIN_PATH"

echo "==> Shipping to $DROPLET_HOST:$REMOTE_DIR"
ssh "$DROPLET_HOST" "install -d -o racquet -g racquet $REMOTE_DIR $REMOTE_DIR/dist"
rsync -avz "$BIN_PATH" "$DROPLET_HOST:$REMOTE_DIR/$BIN_NAME.new"
rsync -avz --delete frontend/dist/ "$DROPLET_HOST:$REMOTE_DIR/dist/"
rsync -avz allowed_emails.txt "$DROPLET_HOST:$REMOTE_DIR/"

echo "==> Swapping binary + restarting service"
ssh "$DROPLET_HOST" "bash -s" <<REMOTE
set -euo pipefail
chown -R racquet:racquet $REMOTE_DIR/$BIN_NAME.new $REMOTE_DIR/dist $REMOTE_DIR/allowed_emails.txt
mv $REMOTE_DIR/$BIN_NAME.new $REMOTE_DIR/$BIN_NAME
chmod 755 $REMOTE_DIR/$BIN_NAME
systemctl restart racquet
systemctl --no-pager --lines=10 status racquet
REMOTE

echo "==> Done."
