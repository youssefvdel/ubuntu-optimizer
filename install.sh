#!/bin/sh
# Ubuntu Optimizer — one-command launcher (no install, runs in-place)
# Usage:  curl -fsSL <url> | sh
set -e

BIN_URL="${UB_OPT_URL:-https://github.com/youssefvdel/ubuntu-optimizer/releases/latest/download/ubuntu-optimizer}"
TMP_BIN="${TMPDIR:-/tmp}/ubuntu-optimizer-$$"

echo "Ubuntu Optimizer & Debloater"
echo "Downloading binary..."

if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$BIN_URL" -o "$TMP_BIN"
elif command -v wget >/dev/null 2>&1; then
    wget -qO "$TMP_BIN" "$BIN_URL"
else
    echo "ERROR: need curl or wget" >&2
    exit 1
fi

chmod +x "$TMP_BIN"

echo "Starting..."
exec "$TMP_BIN"
