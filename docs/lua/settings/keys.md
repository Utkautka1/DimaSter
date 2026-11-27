# Клавиши и действия

[← к оглавлению](../README.md)

```lua
return {
  keys = {
    { key = "k", mods = "ctrl+shift", action = "scroll_top" },
    { key = "g", mods = "ctrl+shift", action = "send:git status\r" },
    { key = "d", mods = "ctrl+shift", action = "plugin:insert_date" },
    { key = "enter", mods = "ctrl", action = "none" }, -- отдать программе
  },
}
```

- `key`: символ (`"a"`, `"="`, `","`) или имя: `enter`, `space`, `tab`, `escape`, `backspace`, `insert`, `delete`, `home`, `end`, `pageup`, `pagedown`, `arrowup`, `arrowdown`, `arrowleft`, `arrowright`, `f1`…`f12`.
- `mods`: `ctrl`, `shift`, `alt`, `super` через `+`.
- С Ctrl и Super буквы распознаются по физической клавише, поэтому работают и на русской раскладке.

| Действие | Что делает | Клавиши по умолчанию |
|---|---|---|
| `copy` | копировать | Ctrl+Shift+C |
| `paste` | вставить | Ctrl+Shift+V |
| `paste_primary` | вставить primary | Shift+Insert |
| `scroll_page_up` | страница вверх | Shift+PageUp |
| `scroll_page_down` | страница вниз | Shift+PageDown |
| `scroll_top` | начало истории | Shift+Home |
| `scroll_bottom` | конец истории | Shift+End |
| `prompt_previous` | предыдущее приглашение | Ctrl+Shift+↑ |
| `prompt_next` | следующее приглашение | Ctrl+Shift+↓ |
| `font_increase` | крупнее | Ctrl+= |
| `font_decrease` | мельче | Ctrl+- |
| `font_reset` | кегль из настроек | Ctrl+0 |
| `search` | поиск | Ctrl+Shift+F |
| `vi_mode` | vi-режим | Ctrl+Shift+Space |
| `command_palette` | палитра команд | Ctrl+Shift+P |
| `new_window` | новое окно в том же каталоге | Ctrl+Shift+N |
| `toggle_fullscreen` | полный экран | Ctrl+Enter |
| `reload_config` | перечитать настройки | Ctrl+Shift+, |
| `hint_mode` | буквенные метки на ссылках и путях | Ctrl+Shift+U |
| `command_history` | история команд с выводом | Ctrl+Shift+H |
| `pick_file` | нечёткий выбор файла и вставка пути (аналог `fzf`) | Ctrl+Shift+T |
| `jump_dir` | прыжок в частую папку (аналог `zoxide`) | Ctrl+Shift+G |
| `sudo_fix` | добавить `sudo` | Ctrl+Shift+S |
| `presentation` | режим презентации | Ctrl+Shift+Z |
| `minimap` | режим миникарты: j/k, g/G, Enter, Esc; щелчок мышью | Ctrl+Shift+M |
| `toggle_monitor` | монитор ресурсов в углу | Ctrl+Shift+O |
| `toggle_statusbar` | строка статуса | Ctrl+Shift+B |
| `none` | отключить сочетание | |
| `send:<текст>` | отправить текст программе; работают `\r`, `\27` | |
| `plugin:<имя>` | команда плагина | |

`prompt_previous` и `prompt_next` работают, только если shell помечает приглашения (OSC 133). Для zsh:

```bash
autoload -Uz add-zsh-hook
dimaster_mark() { print -n "\e]133;A\a" }
add-zsh-hook precmd dimaster_mark
```

## Цепочки нажатий

```lua
keys = {
  { key = "ctrl+a>n", action = "new_window" },       -- Ctrl+A, затем N
  { key = "ctrl+a>ctrl+a", action = "text:\\x01" },   -- отдать сам Ctrl+A
}
```

Следующую клавишу терминал ждёт сколько угодно. Если нажата непривязанная, программа получает всю цепочку и эту клавишу; если при этом есть `catch_all` с `ignore`, всё отбрасывается. Привязка на начало цепочки (`ctrl+a`) отменяет все цепочки с этим началом, и наоборот.

`key` без `mods` может быть всем сочетанием: `"ctrl+shift+a"`; знак плюс пишется последним: `"ctrl++"`.

## Несколько действий

```lua
{ key = "ctrl+shift+k", action = { "scroll_bottom", "text:clear\\n" } }
```

## Все действия с параметром

| Действие | Что делает |
|---|---|
| `text:<текст>` | отправить текст; понимает `\x15`, `\n`, `\r`, `\t`, `\e`, `\\` |
| `send:<текст>` | отправить как есть |
| `csi:<хвост>`, `esc:<хвост>` | `ESC [` или `ESC` и хвост |
| `plugin:<имя>` | команда плагина |
| `activate_key_table:<имя>` | включить таблицу |
| `activate_key_table_once:<имя>` | до первой сработавшей привязки (не `catch_all`) |
| `deactivate_key_table`, `deactivate_all_key_tables` | выключить |
| `end_key_sequence` | отправить программе начало цепочки |
| `ignore` | проглотить нажатие |
| `unbind`, `none` | снять сочетание |

## Приставки

| Приставка или ключ | Что делает |
|---|---|
| `unconsumed:` / `unconsumed = true` | после действия клавиша уходит и программе |
| `performable:` / `performable = true` | забирать клавишу, только если действие возможно (например, `copy` при выделении) |

`global:` и `all:` из Ghostty не поддерживаются: конфиг с ними сообщит об ошибке.

## Таблицы привязок

```lua
keys = {
  { key = "ctrl+shift+r", action = "activate_key_table:scroll" },
  { table = "scroll", key = "k", action = "scroll_page_up" },
  { table = "scroll", key = "j", action = "scroll_page_down" },
  { table = "scroll", key = "escape", action = "deactivate_key_table" },
  { table = "scroll", key = "catch_all", action = "ignore" },
}
```

Поиск идёт от последней включённой таблицы к основной. В `statusbar.lua` имя таблицы — `info.key_table`, число нажатых клавиш цепочки — `info.sequence`.

## Особые клавиши

- `catch_all` — любая не привязанная иначе; можно с модификаторами: `"ctrl+catch_all"`.
- `KeyA` или `key_a`, `Digit1` — физическая клавиша независимо от раскладки; она важнее символа.

`clear_default_keys = true` убирает все сочетания по умолчанию.

## Ctrl+клик

С зажатым Ctrl ссылка под указателем подсвечивается; щелчок открывает:

- адрес (`https://…`, `mailto:` или просто `example.com/путь`) — в браузере;
- папку — в файловом менеджере;
- файл — **папку, где он лежит** (сам файл не запускается).

Имена распознаются относительно текущего каталога shell, включая `Рабочий\ стол`. Домены без `https://` — только с распространёнными окончаниями (`.com`, `.ru`, `.org`, `.dev`, `.io` и т. п.). Если в каталоге есть файл с таким же именем, открывается он.

