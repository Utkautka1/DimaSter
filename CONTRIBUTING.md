# Contributing to DimaSter

**English** · [Русский](CONTRIBUTING.ru.md)

Bug reports, ideas and pull requests are welcome.

## Report a bug

Open an [issue](https://github.com/Utkautka1/DimaSter/issues) and include:

- distribution, desktop or compositor, Wayland or X11;
- DimaSter version (the release you installed, or the commit you built);
- shell and the plugins you use (`zsh-autosuggestions`, Atuin and so on);
- what you pressed, what you expected, what happened; a screenshot helps;
- the log: `RUST_LOG=info dimaster` started from another terminal.

Remove personal data from logs and screenshots before posting.

## Build

You need a Rust toolchain, `fontconfig`, `libxkbcommon`, the `wayland`, `x11` and `xcb` headers and a Vulkan loader or Mesa OpenGL.

```sh
git clone https://github.com/Utkautka1/DimaSter.git
cd DimaSter
cargo run --release -p dimaster
```

## Before a pull request

CI runs these three commands; all must pass:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Rules

- A function is at most 25 lines (clippy enforces it) and does one thing.
- Anything configurable is configurable from Lua: add the key, its default, the generated file entry and a page in [docs/lua](docs/lua/README.md).
- Source strings and comments are in Russian. Every translated string needs an English line in `crates/dimaster-i18n/src/en.tsv`; a test checks it.
- Comments explain why, not what.
- Add a test for a fixed bug. Tests must not start external programs.
- Update [docs/UML.md](docs/UML.md) when you add types or modules.

The project map is in the [README](README.md#architecture).

## License

Contributions are accepted under the [MIT license](LICENSE).
