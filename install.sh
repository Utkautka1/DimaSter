#!/bin/sh
# Установка DimaSter для текущего пользователя, без root.
#   ./install.sh              собрать и установить
#   ./install.sh --uninstall  удалить (настройки в ~/.config/dimaster остаются)
set -eu

bin="$HOME/.local/bin"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"

if [ "${1:-}" = "--uninstall" ]; then
    rm -f -- "$bin/dimaster" "$apps/dimaster.desktop"
    echo "DimaSter удалён"
    exit 0
fi

cd -- "$(dirname -- "$0")"
cargo build --release -p dimaster
mkdir -p -- "$bin" "$apps"
# install заменяет файл целиком: запущенные окна продолжают работать
install -m 755 target/release/dimaster "$bin/dimaster"
cat > "$apps/dimaster.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=DimaSter
Comment=Terminal emulator
Exec=$bin/dimaster
Icon=utilities-terminal
Categories=System;TerminalEmulator;
StartupWMClass=dimaster
DESKTOP

echo "Установлено: $bin/dimaster"
case ":$PATH:" in
    *":$bin:"*) ;;
    *) echo "Добавьте $bin в PATH" ;;
esac
