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
