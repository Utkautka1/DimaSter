#!/bin/sh
# Build an .rpm package (Fedora / RHEL / openSUSE) from target/release/dimaster.
set -eu

VERSION="${1:-0.4.1}"
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
TOP="$ROOT/target/rpm"

cd -- "$ROOT"
if [ ! -x target/release/dimaster ]; then
    cargo build --release -p dimaster
fi

rm -rf -- "$TOP"
mkdir -p -- "$TOP/BUILD" "$TOP/RPMS" "$TOP/SOURCES" "$TOP/SPECS" "$TOP/SRPMS"

cat > "$TOP/SPECS/dimaster.spec" <<EOF
Name:           dimaster
Version:        ${VERSION}
Release:        1
Summary:        GPU-accelerated Linux terminal emulator with Lua configuration
License:        MIT
URL:            https://github.com/Utkautka1/DimaSter
BuildArch:      $(uname -m)
Requires:       fontconfig, libxkbcommon, wayland-libs-client, libX11, libxcb, vulkan-loader

%description
DimaSter is a Rust GPU-accelerated Linux terminal emulator with modular Lua 5.4
configuration, Kitty/Sixel/iTerm2 graphics protocols, out-of-the-box command
syntax highlighting, ghost autosuggestions, auto-pairing, and fuzzy pickers.

%install
rm -rf %{buildroot}
install -Dm755 "${ROOT}/target/release/dimaster" %{buildroot}/usr/bin/dimaster
install -Dm644 "${ROOT}/assets/dimaster.svg" %{buildroot}/usr/share/icons/hicolor/scalable/apps/dimaster.svg
install -Dm644 "${ROOT}/assets/dimaster.png" %{buildroot}/usr/share/icons/hicolor/128x128/apps/dimaster.png
install -Dm644 "${ROOT}/LICENSE" %{buildroot}/usr/share/licenses/dimaster/LICENSE
mkdir -p %{buildroot}/usr/share/applications
cat > %{buildroot}/usr/share/applications/dimaster.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=DimaSter
Comment=GPU-accelerated terminal emulator with Lua configuration
Exec=dimaster
Icon=dimaster
Categories=System;TerminalEmulator;
StartupWMClass=dimaster
DESKTOP

%files
/usr/bin/dimaster
/usr/share/icons/hicolor/scalable/apps/dimaster.svg
/usr/share/icons/hicolor/128x128/apps/dimaster.png
/usr/share/licenses/dimaster/LICENSE
/usr/share/applications/dimaster.desktop
EOF

rpmbuild --define "_topdir $TOP" --define "debug_package %{nil}" -bb "$TOP/SPECS/dimaster.spec"
echo "Built RPM:"
ls -la "$TOP/RPMS/"*/*.rpm
