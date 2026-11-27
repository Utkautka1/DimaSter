#!/bin/sh
# Build an AppImage from target/release/dimaster.
# Usage: build-appimage.sh [version]
#   APPIMAGETOOL=/path/to/appimagetool skips the pinned download.
set -eu

VERSION="${1:-0.1.0}"
ARCH="$(uname -m)"
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT="$ROOT/target/appimage"
APPDIR="$OUT/DimaSter.AppDir"

TOOL_VERSION=1.9.1
case "$ARCH" in
    x86_64) TOOL_SHA=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0 ;;
    aarch64) TOOL_SHA=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158 ;;
    *) echo "unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

cd -- "$ROOT"
[ -x target/release/dimaster ] || cargo build --release -p dimaster

mkdir -p -- "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
    "$APPDIR/usr/share/icons/hicolor/128x128/apps"
install -m 755 target/release/dimaster "$APPDIR/usr/bin/dimaster"
install -m 644 assets/dimaster.png "$APPDIR/usr/share/icons/hicolor/128x128/apps/dimaster.png"
install -m 644 assets/dimaster.png "$APPDIR/dimaster.png"
ln -sf usr/bin/dimaster "$APPDIR/AppRun"
cat > "$APPDIR/dimaster.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=DimaSter
Comment=GPU-accelerated terminal emulator with Lua configuration
Exec=dimaster
Icon=dimaster
Categories=System;TerminalEmulator;
StartupWMClass=dimaster
DESKTOP
cp -- "$APPDIR/dimaster.desktop" "$APPDIR/usr/share/applications/dimaster.desktop"

TOOL="${APPIMAGETOOL:-}"
if [ -z "$TOOL" ]; then
    TOOL="$OUT/appimagetool-$ARCH.AppImage"
    URL="https://github.com/AppImage/appimagetool/releases/download/$TOOL_VERSION/appimagetool-$ARCH.AppImage"
    [ -f "$TOOL" ] || curl -fsSL -o "$TOOL" "$URL"
    echo "$TOOL_SHA  $TOOL" | sha256sum -c -
    chmod +x "$TOOL"
fi

# extract-and-run: CI runners have no FUSE
ARCH="$ARCH" VERSION="$VERSION" APPIMAGE_EXTRACT_AND_RUN=1 \
    "$TOOL" "$APPDIR" "$OUT/DimaSter-$VERSION-$ARCH.AppImage"
echo "Built: $OUT/DimaSter-$VERSION-$ARCH.AppImage"
