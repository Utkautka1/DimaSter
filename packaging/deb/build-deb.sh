#!/bin/sh
# Build a .deb package for Debian/Ubuntu from target/release/dimaster.
set -eu

VERSION="${1:-0.1.0}"
ARCH="$(dpkg --print-architecture 2>/dev/null || echo amd64)"
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
PKG="$ROOT/target/deb/dimaster_${VERSION}_${ARCH}"

cd -- "$ROOT"
cargo build --release -p dimaster

rm -rf -- "$PKG"
mkdir -p -- \
    "$PKG/DEBIAN" \
    "$PKG/usr/bin" \
    "$PKG/usr/share/applications" \
    "$PKG/usr/share/icons/hicolor/scalable/apps" \
    "$PKG/usr/share/icons/hicolor/128x128/apps" \
    "$PKG/usr/share/doc/dimaster"

install -m 755 target/release/dimaster "$PKG/usr/bin/dimaster"
install -m 644 assets/dimaster.svg "$PKG/usr/share/icons/hicolor/scalable/apps/dimaster.svg"
install -m 644 assets/dimaster.png "$PKG/usr/share/icons/hicolor/128x128/apps/dimaster.png"
install -m 644 LICENSE "$PKG/usr/share/doc/dimaster/copyright"

cat > "$PKG/usr/share/applications/dimaster.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=DimaSter
Comment=GPU-accelerated terminal emulator with Lua configuration
Exec=dimaster
Icon=dimaster
Categories=System;TerminalEmulator;
StartupWMClass=dimaster
EOF

cat > "$PKG/DEBIAN/control" <<EOF
Package: dimaster
Version: $VERSION
Section: x11
Priority: optional
Architecture: $ARCH
Depends: libc6, libfontconfig1, libxkbcommon0, libwayland-client0, libx11-6, libxcb1, libvulkan1
Recommends: ffmpeg
Maintainer: Utkautka1 <https://github.com/Utkautka1>
Description: GPU-accelerated Linux terminal emulator with Lua configuration
 DimaSter is a Rust terminal emulator using wgpu with modular Lua
 configuration, Kitty/Sixel/iTerm2 graphics, ligatures, COLR/CBDT emoji,
 shell integration, vi-mode, and command palette.
EOF

dpkg-deb --build "$PKG"
echo "Built: ${PKG}.deb"
