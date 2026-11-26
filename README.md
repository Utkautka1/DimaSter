# DimaSter

Терминал для Linux на Rust. Готовы этапы 1 (ядро), 2 (окно, GPU, ввод) 3 (совместимость) и 4 (лигатуры, цветные эмодзи, CJK, RTL, IME) 5 (настройки на Lua), 6 (графика), 7 (интерфейс) и 8 (плагины, сессии, управление).

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

Все настройки — Lua в `~/.config/dimaster/`; изменения применяются на лету. При первом запуске мастер создаёт дерево и предлагает перенести настройки из Ghostty, kitty или Alacritty. Повторный запуск сохраняет изменённые файлы в `backups/`:

```bash
cargo run --release -p dimaster -- --setup
```

```
~/.config/dimaster/
├── init.lua              загрузчик
├── lib/                  layers.lua (порядок слоёв), merge.lua (объединение)
├── settings/             appearance, font, window, effects, behavior, keys
├── themes/               свои и перенесённые темы
├── plugins/*.lua         плагины терминала
├── profiles/
│   ├── desktop/<de>.lua  по окружению (hyprland, gnome, …)
│   ├── session/<s>.lua   wayland или x11
│   ├── ssh.lua           внутри SSH-сессии
│   └── presentation.lua  DIMASTER_PROFILE=presentation
├── hosts/<имя>.lua       для конкретной машины
└── local.lua             личные правки, применяются последними
```

Слои накладываются сверху вниз в этом порядке: `settings` → `profiles` → `hosts` → `local`. Словари объединяются, списки заменяются, сочетания `keys` складываются. Файлы `local.lua` и `profiles/` мастер не перезаписывает.

Действия для `keys`: `copy`, `paste`, `paste_primary`, `scroll_page_up`, `scroll_page_down`, `scroll_top`, `scroll_bottom`, `font_increase`, `font_decrease`, `font_reset`, `reload_config`, `search`, `vi_mode`, `command_palette`, `plugin:<имя>`, `none`.

## Режимы

| Сочетание | Что делает |
|---|---|
| Ctrl+Shift+F | поиск; Enter / Shift+Enter, Ctrl+R — regex, Esc |
| Ctrl+Shift+Space | vi-режим: hjkl, w b, 0 $, g G, v V Ctrl+V, y, /, q |
| Ctrl+Shift+P | палитра команд |
| Ctrl+клик | открыть ссылку или файл |

## Плагины

Каждый файл `~/.config/dimaster/plugins/*.lua` загружается при старте и после изменения настроек. Команды плагинов появляются в палитре.

```lua
dimaster.command("insert_date", "Вставить дату", function()
  dimaster.send(os.date("%Y-%m-%d"))
end)

dimaster.on("cwd", function(path) dimaster.log(path) end)
```

События: `startup`, `title`, `cwd`, `focus`, `notification`. Действия: `send`, `action`, `notify`, `log`. Чтение: `title`, `cwd`, `selection`, `screen_text`. Один вызов ограничен 100 мс. Плагин выполняется с вашими правами — ставьте только те, чей код вы прочли.

## Управление из командной строки

```bash
dimaster ctl info
```

Команды: `list`, `info`, `get-text`, `send-text <текст>`, `action <имя>`, `set-title <текст>`, `reload`, `new-window`. Внутри окна используется его сокет, снаружи — единственного запущенного. Отключается `remote_control = false`.

## Сессии

Выключены по умолчанию: вывод терминала записывается на диск. Включить в `local.lua`:

```lua
return { session = { restore = true, history_lines = 2000 } }
```

Восстанавливаются окна, каталоги и текст истории без цветов. Запущенные программы не восстанавливаются.

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
