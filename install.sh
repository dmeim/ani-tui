#!/usr/bin/env bash
set -euo pipefail

REPO="dmeim/ani-tui"
BINARY_NAME="ani-tui"

# Detect platform and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        TARGET="aarch64-apple-darwin"
        INSTALL_DIR="/usr/local/bin"
        ;;
    Linux)
        case "$ARCH" in
            x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
            *)      echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        INSTALL_DIR="/usr/local/bin"
        ;;
    *)
        echo "Unsupported OS: $OS. On Windows, use install.ps1 instead."
        exit 1
        ;;
esac

echo "Detected platform: $TARGET"

# Fetch latest release info from GitHub
echo "Fetching latest release..."
RELEASE_URL="https://api.github.com/repos/$REPO/releases/latest"

if command -v jq &>/dev/null; then
    DOWNLOAD_URL=$(curl -fsSL "$RELEASE_URL" | jq -r ".assets[] | select(.name | contains(\"$TARGET\")) | .browser_download_url")
else
    DOWNLOAD_URL=$(curl -fsSL "$RELEASE_URL" | grep -o "\"browser_download_url\": *\"[^\"]*${TARGET}[^\"]*\"" | head -1 | cut -d'"' -f4)
fi

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not find a release for $TARGET."
    echo "Check https://github.com/$REPO/releases for available builds."
    exit 1
fi

# Download and extract
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading $DOWNLOAD_URL..."
curl -fSL "$DOWNLOAD_URL" -o "$TMPDIR/ani-tui.tar.gz"

echo "Extracting..."
tar xzf "$TMPDIR/ani-tui.tar.gz" -C "$TMPDIR"

echo "Installing to $INSTALL_DIR/$BINARY_NAME..."
sudo install -m 755 "$TMPDIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"

echo ""
echo "ani-tui installed successfully!"
echo ""
echo "Usage:"
echo "  ani-tui              Launch the TUI"
echo "  ani-tui --update     Download and install the latest release"
echo "  ani-tui --uninstall  Remove ani-tui from your system"
echo "  ani-tui --version    Show version"
