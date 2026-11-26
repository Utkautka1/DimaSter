# Свободное рисование: `draw.lua`

[← к оглавлению](../README.md)

`dimaster.draw(function(info) … end)` возвращает список фигур поверх окна. Функция вызывается при обновлении данных (`widgets.interval`), смене режима и размера окна — покадровой анимации здесь нет.

`info`: `width`, `height`, `cell_width`, `cell_height` и все поля `statusbar.lua`. Координаты — пиксели; отрицательные отсчитываются от правого или нижнего края.

| Фигура | Поля |
|---|---|
| `{ kind = "rect" }` | `x, y, width, height, radius, color, opacity` |
| `{ kind = "line" }` | `x1, y1, x2, y2, width, color, opacity` |
| `{ kind = "polygon" }` | `points = { x1, y1, x2, y2, … }` (до 64 точек), `color, opacity` |
| `{ kind = "text" }` | `x, y, text, color` |
| `dimaster.circle { }` | `x, y, r, color, opacity` |
| `dimaster.graph { }` | `x, y, width, height, values, max, color, line_width` |

Все фигуры лежат под панелями, а тексты рисуются после всех фигур. Не больше 2000 фигур.

```lua
local history = {}
dimaster.draw(function(info)
  history[#history + 1] = tonumber(info.cpu) or 0
  if #history > 40 then table.remove(history, 1) end
  return {
    { kind = "rect", x = -230, y = 60, width = 210, height = 70, radius = 12, color = "#11111b", opacity = 0.8 },
    dimaster.graph { x = -220, y = 70, width = 190, height = 50, values = history, max = 100, color = "#a6e3a1" },
  }
end)
```
