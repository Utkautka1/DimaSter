# Плагины

[← к оглавлению](../README.md)

Каждый файл `plugins/*.lua` загружается по алфавиту, все — в одно общее состояние Lua.

### Регистрация

| Функция | Что делает |
|---|---|
| `dimaster.on(событие, function(value) end)` | подписка |
| `dimaster.command(имя, название, function() end)` | команда: видна в палитре и вызывается как `plugin:<имя>` |

### События

| Событие | `value` | Когда |
|---|---|---|
| `startup` | пусто | после загрузки плагинов |
| `title` | новый заголовок | программа сменила заголовок |
| `cwd` | каталог | shell сообщил новый каталог (OSC 7) |
| `command` | текст команды | команда завершилась |
| `focus` | `"true"` / `"false"` | окно получило или потеряло фокус |
| `bell` | пусто | программа прислала BEL |
| `notification` | текст | программа прислала уведомление |

### Действия и чтение

| Функция | Что делает |
|---|---|
| `dimaster.send(текст)` | ввести текст в программу |
| `dimaster.action(имя)` | выполнить действие из таблицы выше |
| `dimaster.notify(заголовок, текст)` | уведомление рабочего стола |
| `dimaster.log(текст)` | запись в лог |
| `dimaster.title()` | заголовок |
| `dimaster.cwd()` | текущий каталог |
| `dimaster.selection()` | выделенный текст |
| `dimaster.screen_text()` | видимый экран |

Функции чтения возвращают снимок на момент вызова обработчика. Действия выполняются после того, как обработчик вернулся. Один вызов ограничен 100 мс.

```lua
-- уведомить, когда открыт каталог с секретами
dimaster.on("cwd", function(path)
  if path:find("secrets") then dimaster.notify("DimaSter", "Осторожно: " .. path) end
end)

-- команда в палитре: повторить последнюю команду
dimaster.command("again", "Повторить команду", function()
  dimaster.send("\27[A\r")
end)
```

Плагин выполняется с вашими правами и полной стандартной библиотекой Lua (`io`, `os`). Ставьте только те, чей код прочли.

## Первый плагин

Создайте `~/.config/dimaster/plugins/hello.lua`:

```lua
dimaster.command("hello", "Поздороваться", function()
  dimaster.notify("DimaSter", "Каталог: " .. dimaster.cwd())
end)
```

Сохраните, откройте палитру (`Ctrl+Shift+P`) и выберите «Поздороваться». Чтобы повесить на клавишу:

```lua
-- local.lua
return { keys = { { key = "ctrl+shift+e", action = "plugin:hello" } } }
```

## Идеи

```lua
-- команда с текстом из выделения
dimaster.command("google", "Искать выделенное", function()
  local text = dimaster.selection()
  if text ~= "" then
    dimaster.send("xdg-open 'https://duckduckgo.com/?q=" .. text:gsub("'", ""):gsub(" ", "+") .. "'\r")
  end
end)
```

```lua
-- напомнить о долгой сборке
dimaster.on("command", function(line)
  if line:match("^cargo build") then dimaster.notify("Сборка", "Готово: " .. line) end
end)
```

## Ограничения и отладка

- Все плагины, `statusbar.lua` и `draw.lua` живут в одном состоянии Lua: объявляйте переменные через `local`, чтобы не мешать друг другу.
- За один вызов можно поставить в очередь не больше 256 действий.
- Обработчик дольше 100 мс прерывается.
- Ошибка одного плагина не останавливает остальные; смотрите её в stderr. `dimaster.log("…")` пишет туда же.
- `dimaster.tr("…")` переводит строку на язык терминала.
