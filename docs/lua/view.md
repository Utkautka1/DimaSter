# Просмотр таблиц

[← к оглавлению](README.md)

```bash
dimaster view users.csv orders.json
```

Открывает CSV, TSV, JSON, JSONL, FWF, Logfmt, HTML, Markdown, базы SQLite и дампы SQL. Каждая таблица — отдельная вкладка, имя вкладки — имя файла. Файлы не изменяются: данные копируются в SQLite в памяти.

| Что передать | Зачем |
|---|---|
| `-f формат` | формат, если расширение его не называет (`logfmt`, `fwf`, …) |
| `--separator знак` | разделитель вместо запятой |
| `--no-header` | первая строка — данные |

Без файлов читается конвейер: `ps aux | dimaster view -f fwf`.

## Клавиши

| Клавиша | Действие (имя для `view.lua`) |
|---|---|
| `j` `k` `h` `l`, стрелки, `b` `w` | `down` `up` `left` `right` |
| `Ctrl+d` `Ctrl+u` | `half_down` `half_up` |
| `Ctrl+f` `Ctrl+b`, PageDown, PageUp | `page_down` `page_up` |
| `g` `G`, Home, End | `top` `bottom` |
| `0` `$` | `first_column` `last_column` |
| Enter | `record`: одна запись; `f` — вид JSON |
| `c` | `copy`: строка в буфер обмена |
| `e` | `fit`: уместить все колонки |
| `t`, Tab, Shift+Tab | `tabs` `next_tab` `previous_tab` |
| `/` | `search`: нечёткий поиск |
| `:` | `command` |
| `s` `o` `f` | `select` `order` `filter` |
| `Ctrl+r` | `reset`: исходная таблица |
| `q` `Q` | `close` (вкладку) и `quit` |

## Команды после `:`

| Команда | Пример |
|---|---|
| `q` — запрос SQL, `_` — то, что на экране | `q SELECT name FROM _ WHERE age > 30` |
| `f` — фильтр | `f age > 30` |
| `o` — порядок | `o age DESC` |
| `s` — колонки | `s name, age` |
| `schema` — все таблицы и колонки | |
| `import файл`, `export файл.csv` | существующий файл не перезаписывается |

## view.lua

Файл `~/.config/dimaster/view.lua` (мастер настройки создаёт его) возвращает таблицу. Всё в ней необязательно; читается при каждом запуске `dimaster view`.

```lua
return {
  enabled = true,        -- false: просмотрщик выключен
  max_width = 40,        -- самая широкая колонка
  gap = 2,               -- пробелы между колонками
  fit = false,           -- сразу уместить все колонки
  header = true,
  row_numbers = true,

  -- "red", "bright_blue", номер 0–255, "#rrggbb"; false — без цвета
  colors = {
    header = "blue", text = false, status = "magenta", numbers = "bright_black",
    selected = "#313244", selected_text = "#cdd6f4", zebra = false,
  },

  -- false снимает назначение
  keys = { x = "quit", ["ctrl+n"] = "down", q = false },

  -- правила для колонок с таким именем в любой таблице
  columns = {
    order = { "id", "name" },   -- эти колонки первыми
    hidden = { "password" },
    id = { title = "№", width = 6, align = "right", color = "yellow" },
  },

  -- правила только для таблицы users (вкладка users)
  tables = {
    users = {
      columns = {
        age = {
          -- row — все значения записи по именам колонок; всё текстом
          format = function(value, row)
            if tonumber(value) and tonumber(value) > 30 then
              return value .. " лет", "red"
            end
            return value
          end,
        },
      },
    },
  },

  -- своя строка состояния
  status = function(info)
    return info.title .. "  " .. info.row .. "/" .. info.rows .. "  " .. info.column
  end,
}
```

`order` и `hidden`, заданные для таблицы, заменяют общие. Остальные поля колонки берутся из правил таблицы, а чего там нет — из общих. В `status` доступны `title`, `row`, `rows`, `column`, `tab`, `tabs`, `sql`.

Цвета по умолчанию — палитра терминала, поэтому таблица меняется вместе с темой и скином.
