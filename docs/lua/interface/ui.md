# Вид интерфейса: `ui.lua`

[← к оглавлению](../README.md)

Всё, что рисуется поверх терминала, задаётся таблицей `ui`. Файл `ui.lua` читается сразу после `settings/`, поэтому профили, `hosts/` и `local.lua` могут его перекрыть. Изменения применяются на лету.

| Раздел | Ключи |
|---|---|
| `panel` | `background`, `color`, `selected_background`, `selected_color`, `radius`, `padding`, `opacity` |
| `title` | `padding`, `format` (`{title}`), `align`, `background`, `color`, `buttons`, `buttons_side` |
| `palette` | `width`, `rows`, `position`, `prompt`, `items` |
| `history` | `width` (доля окна), `rows`, `side` |
| `marks` | `hint_color`, `hint_background`, `found`, `current`, `link` |
| `scrollbar`, `minimap` | `width`, `color`, `command`, `error`, `found`, `side` |
| `notice` | плашка после копирования, см. ниже |
| `monitor` | `side`, `position`, `format` (`{process}`, `{cpu}`, `{memory}`), `waiting` |
| `texts` | `vi`, `hint`, `minimap`, `search`, `search_plain`, `search_regex`, `history`, `history_output`, `history_failed` |
| сам `ui` | `prompt_position`: `"top"` или `"bottom"` |

Кнопки заголовка — список `{ action = "close" | "minimize" | "maximize", text = "…", color = "#…" }`; порядок в списке — порядок слева направо, пустой список убирает кнопки.

## Все значения по умолчанию

```lua
-- Вид всего, что рисуется поверх терминала. Здесь перечислены все ключи со
-- значениями по умолчанию: уберите «--» перед нужной строкой и поменяйте.
-- Цвета — "#rrggbb"; не заданный цвет берётся из темы.
-- Строка статуса настраивается отдельно, в statusbar.lua.

return {
  ui = {
    -- prompt_position = "bottom", -- строка поиска и подсказки: top | bottom

    panel = { -- палитра, история, поиск, монитор, адрес ссылки
      -- background = "#292b31",
      -- color = "#d8dee9",
      -- selected_background = "#d8dee9",
      -- selected_color = "#11131a",
      -- radius = 0,
      -- padding = 0,
      -- opacity = 1.0,
    },

    title = { -- своя полоса заголовка
      -- padding = 8,
      -- format = "{title}",
      -- align = "left", -- left | center | right
      -- background = "#11131a",
      -- color = "#d8dee9",
      -- buttons_side = "right",
      -- buttons = { { action = "minimize", text = " — " }, { action = "maximize", text = " □ " }, { action = "close", text = " ✕ " } },
    },

    palette = {
      -- width = 50,
      -- rows = 11,
      -- position = "top", -- top | center | bottom
      -- prompt = " > ",
      -- items = { { title = "Копировать", action = "copy" } }, -- свой список команд
    },

    history = {
      -- width = 0.6,
      -- rows = 10,
      -- side = "right",
    },

    marks = { -- метки на ссылках и подсветки
      -- hint_color = "#11131a",
      -- hint_background = "#e0af68",
      -- found = "#6b5a1e",
      -- current = "#b0620f",
      -- link = "#2f4a7a",
    },

    scrollbar = {
      -- width = 5,
      -- color = "#9aa5ce",
      -- command = "#7aa2f7",
      -- error = "#f7768e",
      -- found = "#e0af68",
      -- side = "right",
    },

    minimap = {
      -- width = 90,
      -- color = "#9aa5ce",
      -- command = "#7aa2f7",
      -- error = "#f7768e",
      -- side = "right",
    },

    monitor = {
      -- side = "right",
      -- position = "top",
      -- format = " {process}  CPU {cpu}%  {memory} ",
      -- waiting = " ожидание ",
    },

    notice = { -- плашка после копирования
      -- copy = true,
      -- primary = false, -- показывать и при простом выделении мышью
      -- text = " ✓ Скопировано: {count} ",
      -- duration = 1200,
      -- side = "right",
      -- position = "bottom",
      -- margin = { x = 10, y = 4 },
      -- свой вид (иначе как у panel): background, color, radius, padding, opacity
    },

    texts = {
      -- vi = " VI: hjkl w b 0 $ g G · v V · y копировать · / поиск · q выход ",
      -- hint = " Метка — открыть, ЗАГЛАВНАЯ — копировать, Esc ",
      -- minimap = " Карта: j/k прокрутка · g/G края · Enter здесь · Esc назад ",
      -- search = " Поиск: {query}_  {position}/{count}  [{kind}, Ctrl+R] ",
      -- search_plain = "текст",
      -- search_regex = "regex",
      -- history = " j/k выбор · Enter вставить · y вывод · q",
      -- history_output = "вывод",
      -- history_failed = "код {code}: {reason}",
    },
  },
}
```

## Плашка после копирования

```lua
ui = {
  notice = {
    text = " 󰅍 Скопировано \n {preview} ({count}) ",
    duration = 1500,                  -- мс, 200–60000
    side = "center",                  -- left | center | right
    position = "top",                 -- top | center | bottom
    margin = { x = 12, y = 12 },
    background = "#a6e3a1", color = "#11111b", radius = 12, padding = 8, opacity = 0.95,
  },
}
```

| Подстановка | Что вставляется |
|---|---|
| `{count}` | число символов |
| `{lines}` | число строк |
| `{preview}` | начало первой строки, до 30 знаков |
| `{target}` | `clipboard` или `primary` (выделение мышью) |

`copy = false` отключает плашку. Простое выделение мышью кладёт текст только в primary (вставка средней кнопкой), поэтому плашки нет; `primary = true` включает её и там. С `copy_on_select = "clipboard"` выделение копирует в обычный буфер, и плашка показывается.
