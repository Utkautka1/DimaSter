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
#
# Also installs JetBrainsMono Nerd Font and starship when missing
# (set DIMASTER_NO_EXTRAS=1 to skip) and creates the default settings.
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

# --- Дополнения: шрифт и starship ---
# DIMASTER_NO_EXTRAS=1 отключает. Сбой загрузки не отменяет установку терминала.

FONT_NAME="JetBrainsMono Nerd Font"
FONT_BASE="https://github.com/ryanoasis/nerd-fonts/releases/latest/download"
STARSHIP_BASE="https://github.com/starship/starship/releases/latest/download"

# check_sum файл ожидаемая_сумма: без суммы файл не принимается
check_sum() {
    [ -n "$2" ] && [ "$(sha256sum "$1" | cut -d' ' -f1)" = "$2" ]
}

fetch_font() {
    curl -fsSL "$FONT_BASE/JetBrainsMono.tar.xz" -o "$1/font.tar.xz" || return 1
    curl -fsSL "$FONT_BASE/SHA-256.txt" -o "$1/sums" || return 1
    sum="$(awk '$2 == "JetBrainsMono.tar.xz" { print $1 }' "$1/sums")"
    check_sum "$1/font.tar.xz" "$sum" || return 1
    mkdir -p -- "$DATA_DIR/fonts/JetBrainsMonoNerd" || return 1
    tar -xJf "$1/font.tar.xz" -C "$DATA_DIR/fonts/JetBrainsMonoNerd" || return 1
    if command -v fc-cache >/dev/null 2>&1; then
        fc-cache -f "$DATA_DIR/fonts" >/dev/null 2>&1 || true
    fi
}

install_font() {
    if command -v fc-list >/dev/null 2>&1 && fc-list | grep -q "$FONT_NAME"; then
        return 0
    fi
    echo "Downloading $FONT_NAME..."
    dir="$(mktemp -d)"
    fetch_font "$dir" || echo "Font not installed: download or checksum failed." >&2
    rm -rf -- "$dir"
}

fetch_starship() {
    name="starship-$2-unknown-linux-musl.tar.gz"
    curl -fsSL "$STARSHIP_BASE/$name" -o "$1/$name" || return 1
    curl -fsSL "$STARSHIP_BASE/$name.sha256" -o "$1/sum" || return 1
    check_sum "$1/$name" "$(cut -d' ' -f1 "$1/sum")" || return 1
    tar -xzf "$1/$name" -C "$1" starship || return 1
    install -m 755 "$1/starship" "$BIN_DIR/starship"
}

install_starship() {
    if command -v starship >/dev/null 2>&1 || [ -x "$BIN_DIR/starship" ]; then
        return 0
    fi
    case "$(uname -m)" in
        x86_64|amd64) cpu="x86_64" ;;
        aarch64|arm64) cpu="aarch64" ;;
        *) return 0 ;;
    esac
    echo "Downloading starship..."
    dir="$(mktemp -d)"
    if fetch_starship "$dir" "$cpu"; then
        echo "starship installed. To use its prompt add to your shell config, e.g. for zsh:"
        echo '  eval "$(starship init zsh)"'
    else
        echo "starship not installed: download or checksum failed." >&2
    fi
    rm -rf -- "$dir"
}

install_extras() {
    [ -z "${DIMASTER_NO_EXTRAS:-}" ] || return 0
    install_font
    install_starship
}

# Настройки по умолчанию сразу, не дожидаясь первого окна; свои не трогаются.
create_config() {
    config="${XDG_CONFIG_HOME:-$HOME/.config}/dimaster/init.lua"
    [ ! -e "$config" ] || return 0
    # при `curl | sh` на входе сам скрипт: ответы мастера читаются из терминала.
    # Без терминала мастер, которому есть что спросить, ничего не запишет
    # и повторит вопрос при первом открытии окна
    answers=/dev/null
    if (: </dev/tty) 2>/dev/null; then
        answers=/dev/tty
    fi
    "$BIN_DIR/dimaster" --setup <"$answers" ||
        echo "Settings will be created on first launch." >&2
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
install_extras
create_config
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "Add $BIN_DIR to your PATH to run 'dimaster' directly." ;;
esac
