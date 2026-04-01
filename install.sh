#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="ani-tui"
REPO_DIR="$(cd "$(dirname "$0")" && pwd)"

# Platform-appropriate data and install directories
OS="$(uname -s)"
case "$OS" in
    Darwin)
        DATA_DIR="$HOME/Library/Application Support/ani-tui"
        INSTALL_DIR="/usr/local/bin"
        ;;
    Linux)
        DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/ani-tui"
        INSTALL_DIR="/usr/local/bin"
        ;;
    *)
        echo "Unsupported OS: $OS. On Windows, use install.ps1 instead."
        exit 1
        ;;
esac

echo "Building ani-tui (release)..."
cargo build --release --manifest-path "$REPO_DIR/Cargo.toml"

echo "Installing to $INSTALL_DIR/$BINARY_NAME..."
sudo cp "$REPO_DIR/target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Store repo path so --update knows where to rebuild from
mkdir -p "$DATA_DIR"
echo "$REPO_DIR" > "$DATA_DIR/.repo-path"

echo ""
echo "ani-tui installed successfully!"
echo ""
echo "Usage:"
echo "  ani-tui              Launch the TUI"
echo "  ani-tui --update     Pull latest changes and reinstall"
echo "  ani-tui --uninstall  Remove ani-tui from your system"
echo "  ani-tui --version    Show version"
