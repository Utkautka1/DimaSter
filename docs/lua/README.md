# Lua в DimaSter

Всё, что настраивается и расширяется, пишется на Lua 5.4 (встроен в терминал). Каталог: `$XDG_CONFIG_HOME/dimaster`, обычно `~/.config/dimaster`.

**Новичку:** [С чего начать](start.md) · [Готовые рецепты](recipes.md) · [Команды и управление](cli.md)

| Файл в `~/.config/dimaster` | Справка |
|---|---|
| `init.lua`, `lib/layers.lua`, `lib/merge.lua` | [Слои и контекст](core/layers.md) |
| `local.lua`, `profiles/`, `hosts/` | [Профили и личные правки](core/profiles.md) |
| `settings/appearance.lua` | [Внешний вид](settings/appearance.md) |
| `settings/font.lua` | [Шрифт](settings/font.md) |
| `settings/window.lua` | [Окно](settings/window.md) |
| `settings/effects.lua` | [Анимации](settings/effects.md) |
| `settings/behavior.lua` | [Поведение, умная оболочка, окружение](settings/behavior.md) |
| `snippets` и списки SSH, Docker | [Списки](settings/lists.md) |
| `settings/keys.lua` | [Клавиши и действия](settings/keys.md) |
| `ui.lua` | [Вид интерфейса](interface/ui.md) |
| `draw.lua` | [Свободное рисование](interface/draw.md) |
| `statusbar.lua` | [Строка статуса и виджеты](interface/statusbar.md) |
| ключи `programs`, `bell` | [Программы и звонок](settings/programs.md) |
| ключ `cursor` | [Курсор](interface/cursor.md) |
| `Ctrl+Shift+K` | [Витрина скинов](themes/skins.md) |
| `themes/*.lua` | [Темы](themes/themes.md) |
| `lang/*.lua` | [Языки](lang/lang.md) |
| `plugins/*.lua` | [Плагины](plugins/plugins.md) |
| `fetch.lua` | [fetch](fetch/fetch.md) |
| `view.lua` | [Просмотр таблиц](view.md) |

## Где что лежит

| Путь | Назначение | Перезаписывает мастер |
|---|---|---|
| `init.lua` | точка входа, возвращает таблицу настроек | да |
| `lib/layers.lua` | какие файлы и в каком порядке читаются | да |
| `lib/merge.lua` | как объединяются таблицы | да |
| `settings/appearance.lua` | тема, отступы, курсор | да |
| `settings/font.lua` | шрифт | да |
| `settings/window.lua` | окно | да |
| `settings/effects.lua` | анимации | да |
| `settings/behavior.lua` | история, shell, мышь, буфер, сессии | да |
| `settings/keys.lua` | сочетания клавиш | да |
| `themes/<имя>.lua` | цветовые темы | только `imported.lua` |
| `profiles/...` | настройки по обстоятельствам | нет |
| `hosts/<имя машины>.lua` | настройки одной машины | нет |
| `ui.lua` | вид интерфейса поверх терминала | нет |
| `lang/<код>.lua` | свои переводы | нет |
| `local.lua` | ваши правки, самый сильный слой | нет |
| `plugins/*.lua` | плагины | нет |
| `fetch.lua` | оформление `dimaster fetch` | нет |
| `statusbar.lua` | строка статуса | нет |

Перед перезаписью изменённого файла мастер (`dimaster --setup`) кладёт копию в `backups/<время>/`.

Изменения настроек, тем и плагинов применяются на лету. `fetch.lua` читается при каждом запуске `dimaster fetch`.

Свои правки пишите в `local.lua`: так повторный запуск мастера их не затронет.

## Рецепты

```lua
-- profiles/desktop/gnome.lua: в GNOME нужна своя рамка
return { window = { decorations = "own" } }
```

```lua
-- hosts/laptop.lua: на ноутбуке крупнее и без эффектов
return { font = { size = 13 }, effects = { cursor_animation = false, smooth_scroll = false } }
```

```lua
-- local.lua: клавиши tmux одним нажатием
return {
  keys = {
    { key = "t", mods = "alt", action = "send:\2c" },     -- Ctrl+B, c
    { key = "arrowright", mods = "alt", action = "send:\2n" },
  },
}
```

## Ошибки и отладка

- Ошибка в настройках: действуют прежние, в заголовке окна «ошибка в init.lua», подробности в stderr.
- Подробный лог: `RUST_LOG=info dimaster`.
- Проверить, что окно видит: `dimaster ctl info`, `dimaster ctl reload`.
- Переменные окружения: `DIMASTER_PROFILE` (профиль), `DIMASTER_FONT` (шрифт, если не задан в настройках), `DIMASTER_TERM` (значение `TERM`).

## Чего пока нет по сравнению с Ghostty и kitty

| Область | Нет |
|---|---|
| Окна | вкладки и сплиты (не планировались по требованиям) |
| Текст | эмодзи-кластеры (режим 2027) |
| Клавиатура | глобальные сочетания и рассылка во все окна (`global:`, `all:`) |
| Картинки | z-index |
| Настройки | правила замены шрифта по диапазонам |
| Прочее | настройка кнопок мыши, режим 132 колонок; выпадающее окно делается средствами композитора |
