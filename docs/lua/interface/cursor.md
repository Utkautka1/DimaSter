# Курсор

[← к оглавлению](../README.md)

Ключ `cursor` — имя формы или таблица. Пишите в `local.lua`:

```lua
return {
  effects = { cursor_blink = true, cursor_animation = true, cursor_trail = true },
  cursor = {
    shape = "block",
    color = "#f5c2e7", radius = 4, opacity = 0.9,
    speed = 18, trail_length = 12, trail_opacity = 0.2,
    hollow_unfocused = true,
  },
}
```

## Форма и вид

| Ключ | Что задаёт | По умолчанию |
|---|---|---|
| `shape` | `"block"`, `"underline"`, `"bar"` | `"block"` |
| `thickness` | толщина черты и подчёркивания, пиксели, 0–100; 0 — по шрифту | `0` |
| `width` | доля ширины ячейки, 0.1–4 (не для `bar`) | `1.0` |
| `height` | доля высоты ячейки, 0.1–2 (не для `underline`) | `1.0` |
| `radius` | скругление, пиксели | `0` |
| `opacity` | 0.05–1 | `1.0` |
| `color` | цвет курсора | из темы |
| `text_color` | цвет символа под блоком | цвет фона |
| `hollow_unfocused` | рамкой, пока окно не в фокусе | `false` |

## Движение

| Ключ | Что задаёт | По умолчанию |
|---|---|---|
| `easing` | `"smooth"` — быстро и с торможением, `"linear"` — равномерно | `"smooth"` |
| `speed` | 1–2000: для `smooth` резкость, для `linear` ячеек в секунду | `28` |
| `blink_interval` | миллисекунды в каждой фазе мигания | `530` |
| `trail_length` | копий в хвосте, 1–60 | `5` |
| `trail_distance` | с какого прыжка (в ячейках) появляется хвост | `1.5` |
| `trail_opacity` | непрозрачность хвоста | `0.35` |
| `trail_color` | цвет хвоста | как у курсора |

Включают и выключают движение, мигание и хвост ключи `effects` ([Анимации](../settings/effects.md)). Программы вроде vim могут сами менять форму курсора.
