# DimaSter

[![CI](https://github.com/Utkautka1/DimaSter/actions/workflows/ci.yml/badge.svg)](https://github.com/Utkautka1/DimaSter/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A fast, GPU-accelerated Linux terminal emulator written in Rust (`wgpu` + `winit`), featuring modular **Lua 5.4** configuration, full **Kitty keyboard & graphics protocols**, **COLR/CBDT color emoji**, **shell integration**, **vi mode**, **command palette**, and a built-in **Lua status bar**.

![DimaSter terminal screenshot](assets/screenshots/main.png)

## Features

- **GPU Rendering (`wgpu`)**: Vulkan primary backend with automatic OpenGL fallback; works on Wayland and X11.
- **Custom Font Rasterizer & Shaping**: `rustybuzz` text shaping with ligatures, OpenType `features`, per-style font families (`bold`, `italic`, `bold_italic`), Nerd Font icon fitting, RTL (bidi), CJK, IME preedit, and **CBDT + COLR v0/v1 color emoji**.
- **Full Keyboard & Input Support**:
  - Complete **Kitty Keyboard Protocol** (all 5 flags: disambiguate, event types, alternate keys, all keys, associated text).
  - **Unsafe Paste Protection**: warns before pasting multi-line text without bracketed paste or dangerous control sequences.
  - **Mouse Selection**: word/line/block (`Alt+drag`), auto-scroll when dragging past viewport edges, primary/clipboard `copy_on_select`, and OSC 8 hyperlinks (`Ctrl+click`).
- **Graphics Protocols**:
  - **Kitty** (`t=d/f/t/s` including POSIX shared memory, `a=f`/`a=a` frame animations, `U=1` Unicode placeholders for `tmux` + `yazi`, `x/y/w/h` cropping).
  - **Sixel**, **iTerm2** (`OSC 1337`), and built-in `dimaster icat` (images and video via `ffmpeg`) + `dimaster fetch`.
- **Memory-Efficient Scrollback**: RLE + style-run packed history compression with full reflow on resize.
- **Layered Lua 5.4 Configuration**: hot-reloaded on save, with built-in migration wizard from Ghostty, Kitty, and Alacritty (`dimaster --setup`) and Ghostty theme support.
- **Productivity Modes & Widgets**:
  - Command Palette (`Ctrl+Shift+P`), Scrollback Search (`Ctrl+Shift+F`), Vi Mode (`Ctrl+Shift+Space`), Link Hints (`Ctrl+Shift+U`), Command History (`Ctrl+Shift+H`), Minimap (`Ctrl+Shift+M`), Resource Monitor (`Ctrl+Shift+O`), Status Bar (`Ctrl+Shift+B`), Sudo Fix (`Ctrl+Shift+S`), Presentation Mode (`Ctrl+Shift+Z`).
  - Lua plugin API (`~/.config/dimaster/plugins/*.lua`) and CLI remote control (`dimaster ctl`).

## Installation

### One-Command Install (Linux x86_64, no Rust required)

```sh
curl -fsSL https://raw.githubusercontent.com/Utkautka1/DimaSter/main/install.sh | sh
```

This downloads the latest release binary to `~/.local/bin/dimaster`, installs the desktop entry and icon, and registers `xterm-dimaster` terminfo if `tic` is available.

To uninstall:

```sh
./install.sh --uninstall
```

### Build from Source

Dependencies: Rust toolchain, `fontconfig`, `libxkbcommon`, `wayland` / `x11` / `xcb` development headers, Vulkan loader (or Mesa OpenGL), and optionally `ffmpeg` for video/WebP/JPEG in `icat`.

```sh
git clone https://github.com/Utkautka1/DimaSter.git
cd DimaSter
./install.sh
```

Or run directly with Cargo:

```sh
cargo run --release -p dimaster
```

### Packaging

- **Arch Linux / CachyOS (AUR)**: see [packaging/aur/PKGBUILD](packaging/aur/PKGBUILD).
- **Debian / Ubuntu (`.deb`)**: run `./packaging/deb/build-deb.sh`.

## Usage

```sh
dimaster                       # open terminal with $SHELL
dimaster -e htop               # run a specific command
dimaster --headless ls -la     # render offscreen and print text
dimaster --setup               # run first-time config & import wizard
dimaster icat image.png        # display image/GIF/video in terminal
dimaster fetch                 # built-in system info (customizable via fetch.lua)
dimaster ctl info              # query or control a running window
dimaster ssh-terminfo u@host   # install xterm-dimaster terminfo on a remote server
```

## Keybindings & Modes

| Shortcut | Action |
|---|---|
| `Ctrl+Shift+C` / `Ctrl+Shift+V` | Copy / Paste |
| Drag mouse / `Alt` + drag | Select (primary) / Block selection |
| Middle click | Paste primary selection |
| `Shift+PageUp` / `PageDown` / Wheel | Scroll history |
| `Shift` + mouse | Bypass program mouse capture |
| `Ctrl+Shift+F` | Search scrollback (`Ctrl+R` toggles regex) |
| `Ctrl+Shift+Space` | Vi mode (`hjkl`, `w b`, `0 $`, `g G`, `v V`, `y`, `/`, `q`) |
| `Ctrl+Shift+P` | Command palette |
| `Ctrl+Click` / `Ctrl+Shift+U` | Open link or file / Link hints overlay |
| `Ctrl+Shift+H` | Command history with output snapshots |
| `Ctrl+Shift+S` | Prefix last command with `sudo` |
| `Ctrl+Shift+Z` | Presentation mode |
| `Ctrl+Shift+M` / `O` / `B` | Minimap / Resource monitor / Status bar |

## Configuration

DimaSter stores modular Lua 5.4 configs under `~/.config/dimaster/` (hot-reloaded automatically):

```text
~/.config/dimaster/
├── init.lua              entry point
├── lib/                  layers.lua, merge.lua
├── settings/             appearance, font, window, effects, behavior, keys
├── themes/               custom and imported themes
├── plugins/*.lua         user Lua plugins
├── profiles/             desktop/<de>.lua, session/<wayland|x11>.lua, ssh.lua, presentation.lua
├── hosts/<hostname>.lua  per-machine overrides
└── local.lua             personal overrides (applied last, untouched by --setup)
```

Layers merge top-to-bottom: `settings` -> `profiles` -> `hosts` -> `local`.
Full Lua reference: [docs/lua/README.md](docs/lua/README.md).

## Development & Tests

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo bench -p dimaster-grid
```

## Architecture

| Crate | Responsibility |
|---|---|
| `dimaster-vt` | Streaming VT/ANSI escape sequence parser |
| `dimaster-grid` | Terminal state, screen, compressed scrollback, reflow, graphics state |
| `dimaster-pty` | PTY spawning and process management |
| `dimaster-font` | Font discovery, `rustybuzz` shaping, outline & COLR/CBDT rasterizer |
| `dimaster-render` | `wgpu` renderer, glyph atlas, bidi, shader pipeline |
| `dimaster-image` | PNG, GIF, and Sixel decoders (+ `ffmpeg` fallback) |
| `dimaster-config` | Lua 5.4 config loader, Ghostty/Kitty/Alacritty importer, plugins |
| `dimaster-i18n` | Built-in localization (`en`, `ru`, extensible via Lua) |
| `dimaster` | Application window, input, modes, CLI, session restore |

Architecture diagrams: [docs/UML.md](docs/UML.md).

## License

[MIT](LICENSE)
