#!/bin/sh
# DimaSter installer for the current user (no root required).
#
# One-command install (downloads prebuilt release binary, no Rust needed):
#   curl -fsSL https://raw.githubusercontent.com/Utkautka1/DimaSter/main/install.sh | sh
#
# Local checkout options:
#   ./install.sh              build from source and install
#   ./install.sh --binary     download prebuilt release binary
#   ./install.sh --uninstall  remove binary, desktop entry and icon (keeps ~/.config/dimaster)
set -eu

REPO="Utkautka1/DimaSter"
BIN_DIR="$HOME/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/scalable/apps"

uninstall() {
    rm -f -- "$BIN_DIR/dimaster" "$APPS_DIR/dimaster.desktop" "$ICON_DIR/dimaster.svg"
    echo "DimaSter removed (config in ~/.config/dimaster kept)."
}

write_desktop() {
    mkdir -p -- "$APPS_DIR"
    cat > "$APPS_DIR/dimaster.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=DimaSter
Comment=GPU-accelerated terminal emulator with Lua configuration
Exec=$BIN_DIR/dimaster
Icon=dimaster
Categories=System;TerminalEmulator;
StartupWMClass=dimaster
DESKTOP
}

install_from_source() {
    src_dir="$1"
    cd -- "$src_dir"
    cargo build --release -p dimaster
    mkdir -p -- "$BIN_DIR" "$ICON_DIR"
    install -m 755 target/release/dimaster "$BIN_DIR/dimaster"
    install -m 644 assets/dimaster.svg "$ICON_DIR/dimaster.svg"
    if command -v tic >/dev/null 2>&1 && [ -f assets/dimaster.terminfo ]; then
        tic -x assets/dimaster.terminfo 2>/dev/null || true
    fi
}

install_from_release() {
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64) target="linux-x86_64" ;;
        aarch64|arm64) target="linux-aarch64" ;;
        *)
            echo "Unsupported architecture for prebuilt binary: $arch (build from source with cargo)" >&2
            exit 1
            ;;
    esac
    tmp="$(mktemp -d)"
    trap 'rm -rf -- "$tmp"' EXIT
    base="https://github.com/$REPO/releases/latest/download"
    archive="dimaster-$target.tar.gz"
    echo "Downloading DimaSter ($target) from GitHub Releases..."
    curl -fsSL "$base/$archive" -o "$tmp/$archive"
    if curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS" 2>/dev/null; then
        (cd "$tmp" && sha256sum -c --ignore-missing SHA256SUMS)
    fi
    tar -xzf "$tmp/$archive" -C "$tmp"
    pkg="$tmp/dimaster-$target"
    mkdir -p -- "$BIN_DIR" "$ICON_DIR"
    install -m 755 "$pkg/dimaster" "$BIN_DIR/dimaster"
    if [ -f "$pkg/dimaster.svg" ]; then
        install -m 644 "$pkg/dimaster.svg" "$ICON_DIR/dimaster.svg"
    fi
    if command -v tic >/dev/null 2>&1 && [ -f "$pkg/dimaster.terminfo" ]; then
        tic -x "$pkg/dimaster.terminfo" 2>/dev/null || true
    fi
}

case "${1:-}" in
    --uninstall)
        uninstall
        exit 0
        ;;
    --binary)
        install_from_release
        ;;
    *)
        script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd || pwd)"
        if [ -f "$script_dir/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
            install_from_source "$script_dir"
        else
            install_from_release
        fi
        ;;
esac

write_desktop
echo "Installed: $BIN_DIR/dimaster"
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "Add $BIN_DIR to your PATH to run 'dimaster' directly." ;;
esac
