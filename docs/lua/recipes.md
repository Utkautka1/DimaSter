# Готовые рецепты

[← к оглавлению](README.md)

Каждый пример можно вставить в `local.lua`.

## Ночная и дневная тема

```lua
local hour = tonumber(os.date("%H"))
return { theme = (hour >= 20 or hour < 8) and "catppuccin-mocha" or "gruvbox-dark" }
```

Условие проверяется при загрузке и перезагрузке настроек, а не по таймеру.

## Спокойный режим: без анимаций и прозрачности

```lua
return {
  window = { opacity = 1.0, blur = false },
  effects = { cursor_animation = false, smooth_scroll = false, cursor_trail = false },
  cursor = { shape = "block", pulse = false },
}
```

## На сервере по SSH крупнее и без украшений

```lua
return { font = { size = dimaster.ssh and 13 or 11 }, widgets = { statusbar = not dimaster.ssh } }
```

## Привычные клавиши tmux одним нажатием

```lua
return {
  keys = {
    { key = "alt+t", action = "text:\\x02c" },
    { key = "alt+arrowright", action = "text:\\x02n" },
    { key = "alt+arrowleft", action = "text:\\x02p" },
  },
}
```

## «Лидер» для своих команд

```lua
return {
  keys = {
    { key = "ctrl+a>g", action = "text:git status\\n" },
    { key = "ctrl+a>l", action = "text:git log --oneline -15\\n" },
    { key = "ctrl+a>ctrl+a", action = "text:\\x01" },
  },
}
```

## Режим прокрутки без модификаторов

```lua
return {
  keys = {
    { key = "ctrl+shift+r", action = "activate_key_table:scroll" },
    { table = "scroll", key = "k", action = "scroll_page_up" },
    { table = "scroll", key = "j", action = "scroll_page_down" },
    { table = "scroll", key = "escape", action = "deactivate_key_table" },
    { table = "scroll", key = "catch_all", action = "ignore" },
  },
}
```

## Запретить программам менять вид терминала

```lua
return {
  programs = { title = false, colors = false, cursor_shape = false, pointer_shape = false },
  clipboard_write = false,
  bell = { flash = false, attention = false },
}
```
