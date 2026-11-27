# Строка статуса и виджеты

[← к оглавлению](../README.md)

## Виджеты

```lua
return {
  widgets = {
    statusbar = true,
    interval = 3,      -- секунды между обновлениями, 1–60
    docker = true,     -- считать контейнеры
    git_changes = false, -- число изменённых файлов, см. ниже
    monitor = false,   -- CPU и память команды в углу
    minimap = false,   -- карта буфера видна постоянно
  },
}
```

## `statusbar.lua`

Строку статуса целиком описывает этот файл. `statusbar.lua`:

```lua
dimaster.statusbar(function(info)
  return {
    left = { { text = " " .. info.user .. " ", color = "#11111b", background = "#cba6f7", rounded = true } },
    center = { info.git_branch or "" },
    right = { os.date("%H:%M") },
  }
end)
```

### Элемент

Строка или таблица:

| Ключ | Что задаёт |
|---|---|
| `text` | текст, значки, эмодзи |
| `color`, `background` | цвет текста и подложки |
| `rounded` | полукруги по краям (нужен Nerd Font) |
| `left_cap`, `right_cap` | любые свои символы по краям: `""`, `""`, `"["`; пустая строка — ровный край |
| `radius` | скругление подложки в пикселях — без символов и шрифта |
| `opacity` | непрозрачность подложки, 0.0–1.0 |

### Форма бара

Эти ключи кладутся в ту же таблицу, что и `left`:

| Ключ | Что задаёт | По умолчанию |
|---|---|---|
| `position` | `"top"` или `"bottom"` | `"bottom"` |
| `margin` | отступ бара от краёв окна: число или `{ x, y }`, пиксели | `{ 12, 10 }` |
| `padding` | поля внутри подложки | `0` |
| `background` | общая подложка бара | нет |
| `radius` | её скругление | `0` |
| `opacity` | её непрозрачность | `1.0` |
| `rows` | до 5 строк, каждая со своими `left`, `center`, `right` | одна |
| `gap` | зазор между строками | `4` |

Значения в пикселях — физические и ограничены 200.

```lua
-- парящий бар сверху: скруглённая стеклянная подложка, две строки
dimaster.statusbar(function(info)
  local function chip(text) return { text = " " .. text .. " ", background = "#313244", radius = 8 } end
  return {
    position = "top",
    margin = { x = 24, y = 8 },
    padding = { x = 10, y = 6 },
    background = "#11111b",
    radius = 16,
    opacity = 0.85,
    rows = {
      { left = { chip(info.mode) }, right = { chip(os.date("%H:%M")) } },
      { center = { { text = " " .. (info.cwd or "") .. " ", color = "#11111b", background = "#a6e3a1", left_cap = "", right_cap = "" } } },
    },
  }
end)
```

| Поле `info` | Содержимое |
|---|---|
| `mode` | `NORMAL`, `VI`, `ПОИСК`… |
| `host`, `user`, `cwd` | машина, пользователь, каталог |
| `git_branch`, `git_changes` | ветка и число изменённых файлов |
| `node`, `python`, `go` | версии, если в каталоге есть проект этого языка |
| `process`, `cpu`, `memory` | команда в терминале: имя, загрузка в %, память |
| `system_cpu`, `system_memory` | вся машина: загрузка в % и занятая память |
| `docker_running`, `docker_stopped` | контейнеры |

Поля без данных равны `nil`. Пинг SSH-сервера из ТЗ не реализован.

Ветка читается из `.git/HEAD`, программа `git` не запускается. `git_changes = true` включает `git status`: учтите, что чужой репозиторий (например, из скачанного архива) может через фильтры в `.git/config` выполнить свою программу, стоит лишь зайти в его каталог.

## Рецепты

```lua
-- минимальный бар: каталог и время
dimaster.statusbar(function(info)
  return { left = { info.cwd or "" }, right = { os.date("%H:%M") } }
end)
```

```lua
-- режим показывается, только когда он не обычный
dimaster.statusbar(function(info)
  local left = {}
  if info.mode ~= "NORMAL" then
    left[#left + 1] = { text = " " .. info.mode .. " ", color = "#11111b", background = "#f9e2af", radius = 8 }
  end
  if info.key_table then left[#left + 1] = " [" .. info.key_table .. "]" end
  return { left = left, right = { os.date("%d.%m %H:%M") } }
end)
```

Дополнительные поля `info`: `key_table` (включённая таблица клавиш), `sequence` (сколько клавиш цепочки нажато), `progress` (проценты от программы).

Отключить бар: `widgets = { statusbar = false }` или `Ctrl+Shift+B`.
