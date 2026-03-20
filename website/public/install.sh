#!/bin/sh
# Tunelo installer — curl -fsSL https://tunelo.net/install.sh | sh
set -eu

REPO="jiweiyuan/tunelo"
BINARY="tunelo"
INSTALL_DIR="/usr/local/bin"

# ── Detect platform ────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  os="linux" ;;
  Darwin) os="macos" ;;
  *)      echo "Error: unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)   arch="amd64" ;;
  aarch64|arm64)   arch="arm64" ;;
  *)               echo "Error: unsupported architecture: $ARCH"; exit 1 ;;
esac

PLATFORM="${os}-${arch}"

# ── Get latest version ─────────────────────────────────────────
echo "→ Detecting latest version..."
VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')

if [ -z "$VERSION" ]; then
  echo "Error: could not detect latest version. Check https://github.com/${REPO}/releases"
  exit 1
fi

echo "→ Installing tunelo ${VERSION} (${PLATFORM})..."

# ── Download ───────────────────────────────────────────────────
URL="https://github.com/${REPO}/releases/download/${VERSION}/tunelo-${PLATFORM}"
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

curl -fsSL "$URL" -o "$TMPFILE"
chmod +x "$TMPFILE"

# ── Install ────────────────────────────────────────────────────
if [ -w "$INSTALL_DIR" ]; then
  mv "$TMPFILE" "${INSTALL_DIR}/${BINARY}"
else
  echo "→ Need sudo to install to ${INSTALL_DIR}"
  sudo mv "$TMPFILE" "${INSTALL_DIR}/${BINARY}"
fi

echo ""
echo "  ✔ tunelo ${VERSION} installed to ${INSTALL_DIR}/${BINARY}"
echo ""
echo "  Get started:"
echo "    tunelo http 3000        expose a local service"
echo "    tunelo serve .          share files"
echo "    tunelo --help           see all commands"
echo ""
