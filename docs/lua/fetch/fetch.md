# fetch

[← к оглавлению](../README.md)

`dimaster fetch` собирает сведения на Rust, а оформление берёт из `fetch.lua`. Без файла или при ошибке в нём используется встроенный вариант.

Доступно в скрипте:

- `info`: `user`, `host`, `os`, `kernel`, `uptime`, `packages`, `shell`, `desktop`, `session`, `terminal`, `cpu`, `gpu`, `memory`, `disk`. Неизвестное — `nil`.
- `tr(текст)`: перевод на язык терминала.
- `color(имя, текст)`: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `bold`.
- `dimaster` — контекст; весь стандартный Lua, включая `io.popen` для своих сведений.

Скрипт возвращает:

| Поле | Содержимое |
|---|---|
| `lines` | строки справа |
| `logo` | строки текстового логотипа слева |
| `image` | путь к картинке вместо логотипа |
| `logo_width` | ширина левой колонки в ячейках (по умолчанию 22) |

```lua
local branch = io.popen("git branch --show-current 2>/dev/null"):read("l")
return {
  lines = { color("bold", info.os), "RAM  " .. (info.memory or "?"), "git  " .. (branch or "-") },
  image = "~/Pictures/me.png",
  logo_width = 30,
}
```

`dimaster fetch --logo файл` перекрывает `image`. Чтобы fetch показывался при открытии терминала, добавьте `dimaster fetch` в конец `~/.zshrc`.

## Картинка: случайная, размер, положение, рамка

```lua
return {
  lines = lines,
  random = "~/avatars",   -- только путь к папке: png, jpg, jpeg, webp, gif
  picture = {
    width = 34, height = 17,   -- в ячейках
    position = "left",         -- left | right | top
    gap = 4, top = 1, left = 0,
    fit = "cover",             -- cover (обрезать) | contain (вписать) | stretch
    border = "rounded",        -- rounded | single | double | heavy | none | 6 своих символов
    border_color = "magenta",
  },
}
```

`random` берёт новую картинку при каждом запуске (прошлая не повторяется) и важнее `image`. Без `picture` ширина берётся из `logo_width`, высота — половина ширины. Шесть своих символов рамки идут так: левый верхний, правый верхний, левый нижний, правый нижний, горизонтальный, вертикальный. Картинки вписывает сам DimaSter, в других терминалах `fit` не действует.

Чтобы fetch показывался при открытии DimaSter вместо fastfetch, в `~/.zshrc`:

```bash
if [[ $TERM_PROGRAM == dimaster ]]; then dimaster fetch; else fastfetch; fi
```
