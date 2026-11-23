#!/usr/bin/env bash
#
# install.sh — Install backers from the latest GitHub release, or build from
# source if no prebuilt binary is published.
#
# Usage:
#   ./scripts/install.sh
#   INSTALL_DIR=/usr/local/bin ./scripts/install.sh
set -euo pipefail

REPO="hieudoanm/backers"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

echo "Installing backers..."

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64 | amd64) ARCH="x86_64" ;;
    arm64 | aarch64) ARCH="aarch64" ;;
esac

case "$OS" in
    linux)  TRIPLE="${ARCH}-unknown-linux-gnu" ;;
    darwin) TRIPLE="${ARCH}-apple-darwin" ;;
    *)      TRIPLE="x86_64-pc-windows-msvc" ;;
esac

BINARY_URL="https://github.com/${REPO}/releases/latest/download/cli-backers-${TRIPLE}"
[[ "$OS" != "linux" && "$OS" != "darwin" ]] && BINARY_URL="${BINARY_URL}.exe"

if command -v curl >/dev/null 2>&1; then
    if curl -fsSL "$BINARY_URL" -o /tmp/backers 2>/dev/null; then
        chmod +x /tmp/backers
        mkdir -p "$INSTALL_DIR"
        install -m 0755 /tmp/backers "$INSTALL_DIR/backers"
        rm -f /tmp/backers
        echo "backers installed to ${INSTALL_DIR}/backers"
        "$INSTALL_DIR/backers" --help >/dev/null
        exit 0
    fi
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust toolchain not found. Please install Rust from https://rustup.rs"
    exit 1
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
cd "$TMP_DIR"
git clone --depth 1 "https://github.com/${REPO}.git" backers
cd backers
cargo build --release -p backers
mkdir -p "$INSTALL_DIR"
install -m 0755 target/release/backers "$INSTALL_DIR/backers"
echo "backers installed to ${INSTALL_DIR}/backers"
"$INSTALL_DIR/backers" --help >/dev/null
