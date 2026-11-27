# Анимации

[← к оглавлению](../README.md)

| Ключ | Что делает | По умолчанию |
|---|---|---|
| `cursor_blink` | мигание курсора | `false` |
| `cursor_animation` | плавное перемещение | `true` |
| `cursor_trail` | «хвост» при прыжках | `false` |
| `smooth_scroll` | плавная прокрутка | `true` |

Скорость, мигание и хвост курсора настраиваются в [`cursor`](../interface/cursor.md).

## Примеры

```lua
return {
  effects = { cursor_trail = true, cursor_blink = false, smooth_scroll = true },
  cursor = { speed = 18, trail_length = 12, trail_opacity = 0.25 },
}
```

Постоянные анимации (пульсация курсора, GIF, видео) перерисовывают окно непрерывно и расходуют заряд батареи.
