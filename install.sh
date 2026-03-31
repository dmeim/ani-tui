#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="/usr/local/bin"
BINARY_NAME="ani-tui"
REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/ani-tui"

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
