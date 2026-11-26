# Lua в DimaSter: полный справочник

Всё, что настраивается и расширяется, пишется на Lua 5.4 (встроен в терминал, системный Lua не нужен). Внутренности — на Rust. Каталог: `$XDG_CONFIG_HOME/dimaster`, обычно `~/.config/dimaster`.

- [Где что лежит](#где-что-лежит)
- [Как складываются слои](#как-складываются-слои)
- [Контекст `dimaster`](#контекст-dimaster)
- [Все настройки](#все-настройки)
- [Темы](#темы)
- [Клавиши и действия](#клавиши-и-действия)
- [Плагины](#плагины)
- [fetch](#fetch)
- [Рецепты](#рецепты)
- [Ошибки и отладка](#ошибки-и-отладка)
- [Чего пока нет по сравнению с Ghostty и kitty](#чего-пока-нет-по-сравнению-с-ghostty-и-kitty)

## Где что лежит

| Путь | Назначение | Перезаписывает мастер |
|---|---|---|
| `init.lua` | точка входа, возвращает таблицу настроек | да |
| `lib/layers.lua` | какие файлы и в каком порядке читаются | да |
| `lib/merge.lua` | как объединяются таблицы | да |
| `settings/appearance.lua` | тема, отступы, курсор | да |
| `settings/font.lua` | шрифт | да |
| `settings/window.lua` | окно | да |
| `settings/effects.lua` | анимации | да |
| `settings/behavior.lua` | история, shell, мышь, буфер, сессии | да |
| `settings/keys.lua` | сочетания клавиш | да |
| `themes/<имя>.lua` | цветовые темы | только `imported.lua` |
| `profiles/...` | настройки по обстоятельствам | нет |
| `hosts/<имя машины>.lua` | настройки одной машины | нет |
| `local.lua` | ваши правки, самый сильный слой | нет |
| `plugins/*.lua` | плагины | нет |
| `fetch.lua` | оформление `dimaster fetch` | нет |

Перед перезаписью изменённого файла мастер (`dimaster --setup`) кладёт копию в `backups/<время>/`.

Изменения настроек, тем и плагинов применяются на лету. `fetch.lua` читается при каждом запуске `dimaster fetch`.

Свои правки пишите в `local.lua`: так повторный запуск мастера их не затронет.

## Как складываются слои

Порядок (каждый следующий перекрывает предыдущий):

1. `settings/appearance`, `font`, `window`, `effects`, `behavior`, `keys`
2. `profiles/desktop/<окружение>` — `hyprland`, `gnome`, `kde`, `sway`…
3. `profiles/session/<сессия>` — `wayland` или `x11`
4. `profiles/ssh` — внутри SSH-сессии
5. `profiles/<имя>` — при запуске `DIMASTER_PROFILE=<имя> dimaster`
6. `hosts/<имя машины>`
7. `local`

Отсутствующий файл пропускается. Каждый слой обязан вернуть таблицу.

Объединение: вложенные словари (`font`, `window`, `colors`…) объединяются по ключам, списки (`shell`, `ansi`, `dirs`) заменяются целиком, а `keys` складываются.

`require` ищет модули только в каталоге настроек: `require("lib.merge")`, `require("settings.font")`. Свои модули кладите туда же.

Весь конфиг можно держать и одним файлом: достаточно, чтобы `init.lua` вернул таблицу.

## Контекст `dimaster`

Доступен в настройках, плагинах и `fetch.lua`:

| Поле | Содержимое |
|---|---|
| `dimaster.hostname` | имя машины |
| `dimaster.desktop` | `hyprland`, `gnome`… (строчными) |
| `dimaster.session` | `wayland`, `x11` |
| `dimaster.ssh` | `true` внутри SSH |
| `dimaster.profile` | значение `DIMASTER_PROFILE` |
| `dimaster.config_dir` | каталог настроек |

Это обычный Lua, поэтому настройки могут быть любыми выражениями:

```lua
local night = tonumber(os.date("%H")) >= 20
return { theme = night and "catppuccin-mocha" or "gruvbox-dark" }
```

Условие вычисляется при загрузке и перезагрузке конфига, а не по таймеру.

## Все настройки

### Внешний вид

| Ключ | Тип и значения | По умолчанию |
|---|---|---|
| `theme` | имя темы | нет |
| `colors` | таблица цветов поверх темы (см. [Темы](#темы)) | нет |
| `padding` | число или `{ x = , y = }`, пиксели, 0–500 | `6` |
| `cursor` | `"block"`, `"underline"`, `"bar"` | `"block"` |

### Шрифт: `font = { ... }`

| Ключ | Тип и значения | По умолчанию |
|---|---|---|
| `family` | имя семейства | из fontconfig |
| `size` | пункты, 4–200 | `12` |
| `line_height` | множитель, 0.8–2.0 | `1.0` |
| `ligatures` | `true` / `false` | `true` |
| `dirs` | список каталогов со своими шрифтами | `{}` |

### Окно: `window = { ... }`

| Ключ | Тип и значения | По умолчанию |
|---|---|---|
| `decorations` | `"own"`, `"system"`, `"none"` | `"own"` |
| `opacity` | 0.0–1.0 | `1.0` |
| `blur` | просить композитор размыть фон | `false` |
| `background_image` | путь, понимает `~/` | нет |
| `background_dir` | каталог: случайный фон из него | нет |
| `background_opacity` | заметность картинки, 0.0–1.0 | `0.3` |
| `columns`, `rows` | начальный размер в ячейках (нужны оба) | нет |

`blur` зависит от композитора; в Hyprland размытие задаётся его правилами. Тайловые композиторы игнорируют `columns` и `rows`.

### Анимации: `effects = { ... }`

| Ключ | Что делает | По умолчанию |
|---|---|---|
| `cursor_blink` | мигание курсора | `false` |
| `cursor_animation` | плавное перемещение | `true` |
| `cursor_trail` | «хвост» при прыжках | `false` |
| `smooth_scroll` | плавная прокрутка | `true` |

### Поведение

| Ключ | Тип и значения | По умолчанию |
|---|---|---|
| `scrollback` | строк истории, до 1 000 000 | `10000` |
| `scrollbar` | полоса прокрутки с метками | `true` |
| `smart` | см. [Умная оболочка](#умная-оболочка) | включено |
| `shell` | команда списком: `{ "/usr/bin/fish", "-l" }` | `$SHELL` |
| `term` | значение `TERM`; свой — `"xterm-dimaster"` | `"xterm-256color"` |
| `rtl` | переставлять иврит и арабский | `true` |
| `images_scroll` | картинки уходят в историю с текстом | `true` |
| `copy_on_select` | `"primary"`, `"clipboard"`, `"off"` | `"primary"` |
| `scroll_multiplier` | строк на щелчок колеса, 0.1–50 | `3` |
| `clipboard_write` | программы пишут в буфер обмена (OSC 52) | `true` |
| `remote_control` | разрешить `dimaster ctl` (применяется при запуске) | `true` |
| `session` | `{ restore = false, history_lines = 2000 }` | выключено |

Читать буфер обмена программам нельзя ни при каких настройках. `session.restore` записывает вывод терминала на диск.

## Умная оболочка

DimaSter сам подключает метки команд к zsh, bash и fish, не меняя ваши dotfiles. Это работает, когда запускается сам shell без аргументов (не для `-e`, не для shell с флагами и не для сессии, начатой мастером первого запуска). На удалённых серверах метки не появляются.

```lua
return {
  scrollbar = true,          -- полоса прокрутки: синие метки команд, красные ошибки, жёлтые находки
  smart = {
    integration = true,
    ok = "➜ OK",             -- под успешной командой без вывода; "" отключает
    empty = "∅ Ничего не найдено",
    errors = true,           -- расшифровка кода ошибки
    search_tools = { "grep", "rg", "find", "fd", "ls", "locate" },
    long_command = 10,       -- секунды; 0 отключает
    glow = true,             -- пульсация рамки, если окно не в фокусе
    glow_color = "#7aa2f7",
    notify = true,           -- уведомление рабочего стола
    double_alt_sudo = false, -- осторожно: может срабатывать от Alt+Tab
  },
}
```

`empty` показывается, когда программа из `search_tools` ничего не вывела и вернула 0 или 1 (так `grep` сообщает об отсутствии совпадений).

История команд (Ctrl+Shift+H) хранится в памяти окна: до 100 команд и 200 строк вывода каждой, на диск не пишется. `j`/`k` выбор, Enter вставляет команду, `y` копирует вывод.

В окне настроек есть `window.background_dir`: фон выбирается случайно из картинок каталога при запуске и перечитывании настроек.

## Темы

Тема — файл `themes/<имя>.lua`, возвращающий цвета:

```lua
return {
  foreground = "#cdd6f4",
  background = "#1e1e2e",
  cursor = "#f5e0dc",
  selection = "#585b70",
  selection_text = "#cdd6f4", -- необязательно
  ansi = {
    "#45475a", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de",
    "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8",
  },
}
```

Встроенные: `catppuccin-mocha`, `gruvbox-dark`, `nord`. Ваш файл с тем же именем важнее встроенной темы. Цвета: `#rrggbb` или `#rgb`. Отдельные цвета поверх темы задаются в `colors` теми же ключами.

Программы могут временно менять цвета (OSC 4, 10, 11, 12); сброс возвращает тему.

## Клавиши и действия

```lua
return {
  keys = {
    { key = "k", mods = "ctrl+shift", action = "scroll_top" },
    { key = "g", mods = "ctrl+shift", action = "send:git status\r" },
    { key = "d", mods = "ctrl+shift", action = "plugin:insert_date" },
    { key = "enter", mods = "ctrl", action = "none" }, -- отдать программе
  },
}
```

- `key`: символ (`"a"`, `"="`, `","`) или имя: `enter`, `space`, `tab`, `escape`, `backspace`, `insert`, `delete`, `home`, `end`, `pageup`, `pagedown`, `arrowup`, `arrowdown`, `arrowleft`, `arrowright`, `f1`…`f12`.
- `mods`: `ctrl`, `shift`, `alt`, `super` через `+`.
- С Ctrl и Super буквы распознаются по физической клавише, поэтому работают и на русской раскладке.

| Действие | Что делает | Клавиши по умолчанию |
|---|---|---|
| `copy` | копировать | Ctrl+Shift+C |
| `paste` | вставить | Ctrl+Shift+V |
| `paste_primary` | вставить primary | Shift+Insert |
| `scroll_page_up` | страница вверх | Shift+PageUp |
| `scroll_page_down` | страница вниз | Shift+PageDown |
| `scroll_top` | начало истории | Shift+Home |
| `scroll_bottom` | конец истории | Shift+End |
| `prompt_previous` | предыдущее приглашение | Ctrl+Shift+↑ |
| `prompt_next` | следующее приглашение | Ctrl+Shift+↓ |
| `font_increase` | крупнее | Ctrl+= |
| `font_decrease` | мельче | Ctrl+- |
| `font_reset` | кегль из настроек | Ctrl+0 |
| `search` | поиск | Ctrl+Shift+F |
| `vi_mode` | vi-режим | Ctrl+Shift+Space |
| `command_palette` | палитра команд | Ctrl+Shift+P |
| `new_window` | новое окно в том же каталоге | Ctrl+Shift+N |
| `toggle_fullscreen` | полный экран | Ctrl+Enter |
| `reload_config` | перечитать настройки | Ctrl+Shift+, |
| `hint_mode` | буквенные метки на ссылках и путях | Ctrl+Shift+U |
| `command_history` | история команд с выводом | Ctrl+Shift+H |
| `sudo_fix` | добавить `sudo` | Ctrl+Shift+S |
| `presentation` | режим презентации | Ctrl+Shift+Z |
| `none` | отключить сочетание | |
| `send:<текст>` | отправить текст программе; работают `\r`, `\27` | |
| `plugin:<имя>` | команда плагина | |

`prompt_previous` и `prompt_next` работают, только если shell помечает приглашения (OSC 133). Для zsh:

```bash
autoload -Uz add-zsh-hook
dimaster_mark() { print -n "\e]133;A\a" }
add-zsh-hook precmd dimaster_mark
```

## Плагины

Каждый файл `plugins/*.lua` загружается по алфавиту, все — в одно общее состояние Lua.

### Регистрация

| Функция | Что делает |
|---|---|
| `dimaster.on(событие, function(value) end)` | подписка |
| `dimaster.command(имя, название, function() end)` | команда: видна в палитре и вызывается как `plugin:<имя>` |

### События

| Событие | `value` | Когда |
|---|---|---|
| `startup` | пусто | после загрузки плагинов |
| `title` | новый заголовок | программа сменила заголовок |
| `cwd` | каталог | shell сообщил новый каталог (OSC 7) |
| `command` | текст команды | команда завершилась |
| `focus` | `"true"` / `"false"` | окно получило или потеряло фокус |
| `notification` | текст | программа прислала уведомление |

### Действия и чтение

| Функция | Что делает |
|---|---|
| `dimaster.send(текст)` | ввести текст в программу |
| `dimaster.action(имя)` | выполнить действие из таблицы выше |
| `dimaster.notify(заголовок, текст)` | уведомление рабочего стола |
| `dimaster.log(текст)` | запись в лог |
| `dimaster.title()` | заголовок |
| `dimaster.cwd()` | текущий каталог |
| `dimaster.selection()` | выделенный текст |
| `dimaster.screen_text()` | видимый экран |

Функции чтения возвращают снимок на момент вызова обработчика. Действия выполняются после того, как обработчик вернулся. Один вызов ограничен 100 мс.

```lua
-- уведомить, когда открыт каталог с секретами
dimaster.on("cwd", function(path)
  if path:find("secrets") then dimaster.notify("DimaSter", "Осторожно: " .. path) end
end)

-- команда в палитре: повторить последнюю команду
dimaster.command("again", "Повторить команду", function()
  dimaster.send("\27[A\r")
end)
```

Плагин выполняется с вашими правами и полной стандартной библиотекой Lua (`io`, `os`). Ставьте только те, чей код прочли.

## fetch

`dimaster fetch` собирает сведения на Rust, а оформление берёт из `fetch.lua`. Без файла или при ошибке в нём используется встроенный вариант.

Доступно в скрипте:

- `info`: `user`, `host`, `os`, `kernel`, `uptime`, `packages`, `shell`, `desktop`, `session`, `terminal`, `cpu`, `gpu`, `memory`, `disk`. Неизвестное — `nil`.
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

## Рецепты

```lua
-- profiles/desktop/gnome.lua: в GNOME нужна своя рамка
return { window = { decorations = "own" } }
```

```lua
-- hosts/laptop.lua: на ноутбуке крупнее и без эффектов
return { font = { size = 13 }, effects = { cursor_animation = false, smooth_scroll = false } }
```

```lua
-- local.lua: клавиши tmux одним нажатием
return {
  keys = {
    { key = "t", mods = "alt", action = "send:\2c" },     -- Ctrl+B, c
    { key = "arrowright", mods = "alt", action = "send:\2n" },
  },
}
```

## Ошибки и отладка

- Ошибка в настройках: действуют прежние, в заголовке окна «ошибка в init.lua», подробности в stderr.
- Подробный лог: `RUST_LOG=info dimaster`.
- Проверить, что окно видит: `dimaster ctl info`, `dimaster ctl reload`.
- Переменные окружения: `DIMASTER_PROFILE` (профиль), `DIMASTER_FONT` (шрифт, если не задан в настройках), `DIMASTER_TERM` (значение `TERM`).

## Чего пока нет по сравнению с Ghostty и kitty

| Область | Нет |
|---|---|
| Окна | вкладки и сплиты (не планировались по требованиям) |
| Текст | волнистые и цветные подчёркивания, OSC 8 (гиперссылки), COLR-эмодзи, эмодзи-кластеры (режим 2027) |
| Клавиатура | полный протокол Kitty (поддержан базовый уровень), привязки из нескольких нажатий |
| Картинки | анимация протокола Kitty, Unicode-плейсхолдеры, z-index, обрезка |
| Настройки | отдельные шрифты для bold и italic, выбор OpenType-фич, правила замены шрифта по диапазонам, минимальный контраст |
| Shell | автоматическая shell-интеграция, уведомление о долгой команде |
| Прочее | настройка мыши, подтверждение при закрытии, quick terminal, режим 132 колонок |
