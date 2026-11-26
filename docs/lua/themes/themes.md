# Темы

[← к оглавлению](../README.md)

Тема — файл `themes/<имя>.lua`, возвращающий цвета:

```lua
return {
  foreground = "#cdd6f4",
  background = "#1e1e2e",
  cursor = "#f5e0dc",
  selection = "#585b70",
  selection_text = "#cdd6f4", -- необязательно
  ansi = {
    "#45475a", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de",
    "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8",
  },
}
```

Встроенные: `catppuccin-mocha`, `gruvbox-dark`, `nord`. Кроме того, по имени подхватываются темы Ghostty — из `~/.config/ghostty/themes` и `/usr/share/ghostty/themes`: `theme = "Catppuccin Mocha"`. Ваш файл с тем же именем важнее встроенной темы. Цвета: `#rrggbb` или `#rgb`. Отдельные цвета поверх темы задаются в `colors` теми же ключами.

Программы могут временно менять цвета (OSC 4, 10, 11, 12); сброс возвращает тему.
