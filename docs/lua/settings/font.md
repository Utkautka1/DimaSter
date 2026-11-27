# Шрифт

[← к оглавлению](../README.md)

| Ключ | Тип и значения | По умолчанию |
|---|---|---|
| `family` | имя семейства | из fontconfig |
| `bold`, `italic`, `bold_italic` | отдельные семейства для начертаний | то же, что `family` |
| `size` | пункты, 4–200 | `11` |
| `line_height` | множитель, 0.8–2.0 | `1.0` |
| `ligatures` | `true` / `false` | `true` |
| `features` | список OpenType-фич (`"-calt"`, `"ss01"`) | `{}` |
| `dirs` | список каталогов со своими шрифтами | `{}` |

## Примеры

```lua
return {
  font = {
    family = "JetBrainsMono Nerd Font",
    size = 12,
    line_height = 1.15,
    ligatures = true,
    features = { "ss01", "-calt" },
  },
}
```

```lua
-- курсив другим шрифтом
return { font = { family = "Fira Code", italic = "Victor Mono" } }
```

## Как найти имя семейства

```bash
fc-list : family | sort -u
```

Значки в `fetch`, строке статуса и приглашении требуют Nerd Font. Размер на лету меняют `Ctrl+=`, `Ctrl+-`, `Ctrl+0`; в настройки это не записывается.
