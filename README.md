# DimaSter

Терминал для Linux на Rust. Готовы этапы 1 (ядро), 2 (окно, GPU, ввод) 3 (совместимость) и 4 (лигатуры, цветные эмодзи, CJK, RTL, IME) 5 (настройки на Lua) и 6 (графика).

## Запуск

Окно с вашим shell:

```bash
cargo run --release -p dimaster
```

Другая программа в окне:

```bash
cargo run --release -p dimaster -- -e htop
```

Без окна (печатает получившийся экран):

```bash
cargo run --release -p dimaster -- --headless ls -la
```

| Сочетание | Действие |
|---|---|
| Ctrl+Shift+C / Ctrl+Shift+V | копировать / вставить |
| Потянуть мышью | выделить (сразу попадает в primary selection) |
| Двойной / тройной клик | слово / строка |
| Alt + потянуть | прямоугольный блок |
| Средняя кнопка | вставить primary selection |
| Shift+PageUp / PageDown, колесо | прокрутка истории |
| Shift + мышь | мышь для терминала, даже если программа её захватила |

Своя запись terminfo включается переменной `DIMASTER_TERM=xterm-dimaster`.

Шрифт можно временно сменить переменной `DIMASTER_FONT`.

## Настройки

Файл `~/.config/dimaster/init.lua`; изменения применяются на лету. При первом запуске мастер предложит перенести настройки из Ghostty, kitty или Alacritty. Запустить его снова:

```bash
cargo run --release -p dimaster -- --setup
```

```lua
return {
  theme = "catppuccin-mocha",          -- или gruvbox-dark, nord, свой themes/<имя>.lua
  colors = { background = "#101010" }, -- поверх темы: foreground, cursor, selection, selection_text, ansi
  font = { family = "JetBrainsMono Nerd Font", size = 12, dirs = {} },
  padding = { x = 10, y = 8 },
  cursor = "bar",                      -- block, underline, bar
  scrollback = 10000,
  shell = { "/usr/bin/zsh" },
  rtl = true,
  term = "xterm-256color",             -- или xterm-dimaster
  keys = { { key = "k", mods = "ctrl+shift", action = "scroll_top" } },
}
```

Действия: `copy`, `paste`, `paste_primary`, `scroll_page_up`, `scroll_page_down`, `scroll_top`, `scroll_bottom`, `font_increase`, `font_decrease`, `font_reset`, `reload_config`, `none`.

По умолчанию кегль меняется через Ctrl `=` / `-` / `0`, конфиг перечитывается через Ctrl+Shift+`,`.

## Картинки

Поддерживаются протоколы Kitty, Sixel, iTerm2 и собственный. PNG, GIF и Sixel декодируются своим кодом, остальное через `ffmpeg`.

Внутри DimaSter (картинки, анимации и видео без звука):

```bash
cargo run --release -p dimaster -- icat picture.png movie.mp4
```

Сведения о системе, `--logo` заменяет текстовый логотип картинкой:

```bash
cargo run --release -p dimaster -- fetch --logo picture.png
```

Собственный протокол: `ESC _ D a=show,c=<колонки>,r=<строки>,C=1;<base64 файла> ESC \`. Параметры `c`, `r`, `C` необязательны; `C=1` не двигает курсор.

`images_scroll = false` в конфиге убирает картинки, ушедшие в историю.

## Проверки

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets
```

```bash
cargo bench -p dimaster-grid
```

## Структура

| Крейт | Зачем |
|---|---|
| `dimaster-vt` | разбор escape-последовательностей |
| `dimaster-grid` | экран, история, режимы |
| `dimaster-pty` | запуск shell |
| `dimaster-font` | поиск шрифтов, свой растеризатор |
| `dimaster-render` | отрисовка на wgpu |
| `dimaster-image` | декодеры PNG, GIF, Sixel; остальное через ffmpeg |
| `dimaster-config` | Lua-конфиг, темы, импорт |
| `dimaster` | приложение |

Диаграммы: [docs/UML.md](docs/UML.md). Требования: [docs/NOTES.md](docs/NOTES.md).
