#!/bin/sh
# Build an Arch Linux / CachyOS pacman package (.pkg.tar.zst) from target/release/dimaster.
set -eu

VERSION="${1:-0.1.0}"
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT="$ROOT/target/arch"

cd -- "$ROOT"
if [ ! -x target/release/dimaster ]; then
    cargo build --release -p dimaster
fi

rm -rf -- "$OUT"
mkdir -p -- "$OUT/files"
cp target/release/dimaster "$OUT/files/"
cp assets/dimaster.svg assets/dimaster.png LICENSE "$OUT/files/"

cat > "$OUT/PKGBUILD" <<EOF
pkgname=dimaster
pkgver=${VERSION}
pkgrel=1
pkgdesc="GPU-accelerated Linux terminal emulator with Lua configuration and graphics protocols"
arch=('x86_64')
url="https://github.com/Utkautka1/DimaSter"
license=('MIT')
options=('!debug')
depends=('fontconfig' 'libxkbcommon' 'wayland' 'libx11' 'libxcb' 'vulkan-icd-loader')
optdepends=('ffmpeg: video and extra image formats in icat / Kitty protocol')

package() {
  cd "\$startdir/files"
  install -Dm755 dimaster "\$pkgdir/usr/bin/dimaster"
  install -Dm644 dimaster.svg "\$pkgdir/usr/share/icons/hicolor/scalable/apps/dimaster.svg"
  install -Dm644 dimaster.png "\$pkgdir/usr/share/icons/hicolor/128x128/apps/dimaster.png"
  install -Dm644 LICENSE "\$pkgdir/usr/share/licenses/dimaster/LICENSE"
  install -Dm644 /dev/stdin "\$pkgdir/usr/share/applications/dimaster.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=DimaSter
Comment=GPU-accelerated terminal emulator with Lua configuration
Exec=dimaster
Icon=dimaster
Categories=System;TerminalEmulator;
StartupWMClass=dimaster
DESKTOP
}
EOF

if command -v makepkg >/dev/null 2>&1; then
    (cd "$OUT" && PKGDEST="$OUT" BUILDDIR="$OUT/build" makepkg -fd --nodeps --skipinteg)
else
    docker run --rm -v "$OUT:/work" archlinux:latest sh -ec '
        pacman -Sy --noconfirm --needed base-devel zstd >/dev/null
        useradd -m builder
        chown -R builder:builder /work
        su builder -c "cd /work && PKGDEST=/work BUILDDIR=/work/build makepkg -fd --nodeps --skipinteg"
    '
fi

echo "Built Arch package(s):"
ls -la "$OUT"/*.pkg.tar.zst
