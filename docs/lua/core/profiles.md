# Профили, машины и `local.lua`

[← к оглавлению](../README.md)

`local.lua` применяется последним и мастером не перезаписывается — свои правки пишите туда.

| Файл | Когда читается |
|---|---|
| `profiles/desktop/<окружение>.lua` | `hyprland`, `gnome`, `kde`, `sway`… |
| `profiles/session/<сессия>.lua` | `wayland` или `x11` |
| `profiles/ssh.lua` | внутри SSH-сессии |
| `profiles/<имя>.lua` | `DIMASTER_PROFILE=<имя> dimaster` |
| `hosts/<имя машины>.lua` | на этой машине |

Каждый файл возвращает таблицу с теми же ключами, что и настройки.

## Примеры

```lua
-- profiles/ssh.lua
return { effects = { cursor_animation = false, smooth_scroll = false } }
```

```lua
-- profiles/presentation.lua, запуск: DIMASTER_PROFILE=presentation dimaster
return { font = { size = 18 }, window = { opacity = 1.0 }, widgets = { statusbar = false } }
```

Имя машины для `hosts/<имя>.lua` видно командой `cat /proc/sys/kernel/hostname`; в имени файла — строчные латинские буквы, цифры, `-` и `_`.

Действие `presentation` (Ctrl+Shift+Z) включает крупный вид на лету, без перезапуска.
