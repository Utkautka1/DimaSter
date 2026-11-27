# Слои и контекст `dimaster`

[← к оглавлению](../README.md)

Порядок (каждый следующий перекрывает предыдущий):

1. `settings/appearance`, `font`, `window`, `effects`, `behavior`, `keys`
2. `ui`
3. `profiles/desktop/<окружение>` — `hyprland`, `gnome`, `kde`, `sway`…
4. `profiles/session/<сессия>` — `wayland` или `x11`
5. `profiles/ssh` — внутри SSH-сессии
6. `profiles/<имя>` — при запуске `DIMASTER_PROFILE=<имя> dimaster`
7. `hosts/<имя машины>`
8. `local`

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

## Как объединяются слои: пример

```lua
-- settings/font.lua
return { font = { family = "Fira Code", size = 11 } }
```

```lua
-- local.lua
return { font = { size = 14 } }
```

Итог: `family = "Fira Code"` из первого файла, `size = 14` из второго. Списки так не складываются: `shell`, `ansi`, `dirs`, `ui.title.buttons` из более позднего слоя заменяют прежние целиком. Привязки `keys`, наоборот, накапливаются; чтобы убрать стандартные, задайте `clear_default_keys = true`.

## Свой модуль

```lua
-- colors.lua в каталоге настроек
return { accent = "#cba6f7" }
```

```lua
-- local.lua
local colors = require("colors")
return { smart = { glow_color = colors.accent } }
```
