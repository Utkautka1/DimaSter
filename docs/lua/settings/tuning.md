# Тонкая настройка поведения: `tuning`

[← к оглавлению](../README.md)

Тайминги, размеры списков, клавиши подсказок. Меняйте только нужные ключи — остальные останутся стандартными:

```lua
-- local.lua
return { tuning = { multi_click = 300 } }
```

Защитные пределы и размеры отрисовки — в другой таблице, [`limits`](limits.md).

## Клавиши подсказок

| Ключ | По умолчанию | Что делает |
|---|---|---|
| `suggest_line` | `{ "right", "end" }` | принять всю призрачную подсказку |
| `suggest_word` | `{ "ctrl+right", "ctrl+end", "ctrl+shift+right", "ctrl+shift+end" }` | принять следующее слово; если подсказки из истории нет — исправить опечатку |

Формат: `модификаторы+клавиша`. Модификаторы: `ctrl`, `shift`, `alt`, `super`. Клавиши: `right`, `left`, `up`, `down` (или `arrowright` и т. д.), `end`, `home`, `tab`, буквы. Модификаторы должны совпадать точно. Запись с ошибкой пропускается, пустой список отключает действие. Клавиши работают, только когда есть что принять и включено `smart.autosuggest`.

```lua
-- принимать всё по Tab с Ctrl, слово по Alt+→
return { tuning = { suggest_line = { "ctrl+tab" }, suggest_word = { "alt+right" } } }
```

## Мышь и клавиатура

| Ключ | Единицы | По умолчанию | Что делает |
|---|---|---|---|
| `multi_click` | мс | `400` | за это время повторный щелчок считается двойным или тройным |
| `drag_scroll_step` | мс, не меньше 10 | `45` | шаг автопрокрутки, когда выделяют за краем окна |
| `double_alt` | мс | `350` | окно двойного `Alt` (при `smart.double_alt_sudo`) |
| `resize_edge` | пиксели | `6` | зона захвата края окна без системной рамки |
| `pairs` | список из двух символов | `"()"`, `"[]"`, `"{}"`, `"\"\""`, `"''"` | пары автозакрытия; только ASCII |
| `hint_letters` | буквы и цифры | `"asdfghjklqwertyuiopzxcvbnm"` | метки на ссылках; нужно не меньше двух разных |

## Эффекты

| Ключ | Единицы | По умолчанию | Что делает |
|---|---|---|---|
| `glow_period` | мс, не меньше 100 | `1600` | период пульсации рамки после долгой команды |
| `bell_flash` | мс | `150` | длительность вспышки звонка |
| `presentation_zoom` | пункты | `4` | прибавка к шрифту в режиме презентации |
| `minimap_margin` | пиксели | `8` | отступ миникарты от края |

## История, память, сессия

| Ключ | Единицы | По умолчанию | Что делает |
|---|---|---|---|
| `history_commands` | команд | `1000` | история подсказок на диске |
| `history_dirs` | каталогов | `300` | частые каталоги для `Ctrl+Shift+G` |
| `memory_commands` | 1–10000 | `100` | команд в окне истории `Ctrl+Shift+H` |
| `memory_lines` | 0–10000 | `200` | строк вывода каждой команды в памяти |
| `autosave` | с, не меньше 5 | `30` | как часто сохраняется сессия (при `session.restore`) |
| `restored_marker` | строка | `"── сессия восстановлена ──"` | разделитель под восстановленным выводом |

Больше `memory_commands` и `memory_lines` — больше памяти занимает каждое окно.

## Выбор файлов (`Ctrl+Shift+T`)

| Ключ | По умолчанию | Что делает |
|---|---|---|
| `picker_files` | `1500` | сколько файлов собирать |
| `picker_depth` | `5` | глубина обхода каталогов |
| `picker_skip` | `".git"`, `"target"`, `"node_modules"`, `".cache"` | имена каталогов, которые не обходятся; список заменяется целиком |

## Видео и Docker

| Ключ | Единицы | По умолчанию | Что делает |
|---|---|---|---|
| `video_width` | пиксели, 16–1920 | `480` | ширина кадра в `dimaster icat` |
| `video_fps` | 1–60 | `15` | кадров в секунду |
| `docker_timeout` | с, 1–60 | `2` | ожидание `docker ps` для списка `Ctrl+Shift+D` |

Ожидания экрана `dimaster docker` задаются в [`docker.lua`](../docker.md).

## Полный пример со стандартными значениями

```lua
return {
  tuning = {
    multi_click = 400,
    drag_scroll_step = 45,
    double_alt = 350,
    glow_period = 1600,
    bell_flash = 150,
    autosave = 30,
    history_commands = 1000,
    history_dirs = 300,
    picker_files = 1500,
    picker_depth = 5,
    picker_skip = { ".git", "target", "node_modules", ".cache" },
    hint_letters = "asdfghjklqwertyuiopzxcvbnm",
    pairs = { "()", "[]", "{}", "\"\"", "''" },
    presentation_zoom = 4,
    restored_marker = "── сессия восстановлена ──",
    suggest_line = { "right", "end" },
    suggest_word = { "ctrl+right", "ctrl+end", "ctrl+shift+right", "ctrl+shift+end" },
    memory_commands = 100,
    memory_lines = 200,
    minimap_margin = 8,
    resize_edge = 6,
    video_width = 480,
    video_fps = 15,
    docker_timeout = 2,
  },
}
```
