# Окно

[← к оглавлению](../README.md)

| Ключ | Тип и значения | По умолчанию |
|---|---|---|
| `decorations` | `"own"`, `"system"`, `"none"` | `"none"` |
| `opacity` | 0.0–1.0 | `0.65` |
| `blur` | просить композитор размыть фон | `true` |
| `background_image` | путь, понимает `~/` | нет |
| `background_dir` | каталог: случайный фон из него | нет |
| `background_opacity` | заметность картинки, 0.0–1.0 | `0.3` |
| `columns`, `rows` | начальный размер в ячейках (нужны оба) | нет |

`blur` зависит от композитора; в Hyprland размытие задаётся его правилами. Тайловые композиторы игнорируют `columns` и `rows`.

## Свой класс окна и выпадающий терминал

`dimaster --class имя` задаёт app_id (WM_CLASS) вместо `dimaster`; размер такого окна решает композитор. Пример для Hyprland с конфигом на Lua:

```lua
hl.window_rule({ name = "dimaster-dropdown", match = { class = "^dimaster-dropdown$" },
  workspace = "special:dimaster", float = true, size = "monitor_w*0.7 monitor_h*0.6", center = true })

hl.bind("SUPER + SHIFT + Return", function()
  if hl.get_window("class:dimaster-dropdown") then
    hl.dispatch(hl.dsp.workspace.toggle_special("dimaster"))
  else
    hl.exec_cmd("dimaster --class dimaster-dropdown")
  end
end)
```
