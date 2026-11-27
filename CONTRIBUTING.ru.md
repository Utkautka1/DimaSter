# Как участвовать в DimaSter

**Русский** · [English](CONTRIBUTING.md)

Сообщения об ошибках, идеи и pull request приветствуются.

## Сообщить об ошибке

Откройте [issue](https://github.com/Utkautka1/DimaSter/issues) и укажите:

- дистрибутив, окружение или композитор, Wayland или X11;
- версию DimaSter (установленный релиз или собранный коммит);
- shell и плагины (`zsh-autosuggestions`, Atuin и другие);
- что нажали, чего ждали и что произошло; помогает снимок экрана;
- журнал: запустите `RUST_LOG=info dimaster` из другого терминала.

Перед отправкой уберите из журналов и снимков личные данные.

## Сборка

Нужны Rust, `fontconfig`, `libxkbcommon`, заголовки `wayland`, `x11`, `xcb` и загрузчик Vulkan или Mesa OpenGL.

```sh
git clone https://github.com/Utkautka1/DimaSter.git
cd DimaSter
cargo run --release -p dimaster
```

## Перед pull request

CI запускает эти три команды, все должны проходить:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Правила

- Функция — не длиннее 25 строк (проверяет clippy) и делает одно дело.
- Всё настраиваемое настраивается из Lua: ключ, значение по умолчанию, строка в создаваемом файле и страница в [docs/lua](docs/lua/README.md).
- Строки и комментарии в коде — на русском. Каждой переводимой строке нужна английская в `crates/dimaster-i18n/src/en.tsv`; это проверяет тест.
- Комментарий объясняет «зачем», а не «что».
- На исправленную ошибку — тест. Тесты не запускают внешние программы.
- Добавили типы или модули — обновите [docs/UML.md](docs/UML.md).

Карта проекта — в [README](README.ru.md#разработка).

## Лицензия

Вклад принимается на условиях [MIT](LICENSE).
