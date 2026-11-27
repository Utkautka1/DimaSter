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
| `command` | текст команды, код завершения, секунды | команда завершилась |
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

## Свои клавиши: `dimaster.bind`

```lua
dimaster.bind("ctrl+shift+e", function()
  dimaster.notify("Каталог", dimaster.cwd())
end)

-- в git-репозитории Ctrl+G показывает статус, иначе клавиша уходит программе
dimaster.bind("ctrl+g", function()
  local repo = io.open(dimaster.cwd() .. "/.git/HEAD")
  if not repo then return false end
  repo:close()
  dimaster.send("git status\r")
end)
```

Нажатие пишется так же, как в `keys`: `"ctrl+shift+e"`, `"f5"`, `"key_g"`, `"catch_all"` (любая клавиша). Цепочки `a>b` здесь не поддерживаются. Плагин спрашивается **раньше** обычных привязок. Вернув `false`, обработчик отдаёт клавишу дальше; любое другое значение забирает её. В режимах (поиск, палитра, vi) плагины клавиш не получают. Не больше 128 привязок.

`catch_all` вызывает функцию на каждое нажатие, включая набор паролей, и замедляет ввод.

## Таймеры: `dimaster.every`

```lua
-- раз в минуту проверять место на диске
dimaster.every(60, function()
  local pipe = io.popen("df --output=pcent / | tail -1")
  local used = tonumber(pipe:read("a"):match("%d+"))
  pipe:close()
  if used and used > 90 then dimaster.notify("Диск", "занято " .. used .. "%") end
end)
```

Период в секундах, не меньше 1; до 16 таймеров. Пропущенные во время сна компьютера срабатывания не догоняются.

## История: `dimaster.history()`

Последние 1000 строк вместе с экраном. Доступна в командах, `bind`, `every` и событии `command`; в других обработчиках, `statusbar` и `draw` — пустая строка.

```lua
dimaster.command("errors", "Сколько ошибок на экране", function()
  local _, count = dimaster.history():gsub("error", "")
  dimaster.notify("Найдено", tostring(count))
end)
```

## Настройки на лету: `dimaster.set` и `dimaster.reset`

`dimaster.set` принимает такую же таблицу, как `local.lua`, и накладывает её поверх файлов. Файлы не изменяются.

```lua
local big = false
dimaster.bind("ctrl+shift+f11", function()
  big = not big
  if big then
    dimaster.set({ font = { size = 18 }, window = { opacity = 1.0 } })
  else
    dimaster.reset()
  end
end)
```

```lua
-- красный фон, пока открыт каталог production
dimaster.on("cwd", function(path)
  if path:find("production") then
    dimaster.set({ colors = { background = "#3b1219" } })
  else
    dimaster.reset()
  end
end)
```

- Словари объединяются по ключам, списки заменяются, `keys` добавляются.
- `reset` убирает всё, что установили плагины.
- Таблица с ошибкой не принимается, причина пишется в stderr.
- Изменение любого файла настроек сбрасывает установленное: плагины загружаются заново.
- Значения — строки, числа, флаги и таблицы; функции передать нельзя.

## Код завершения команды

```lua
dimaster.on("command", function(line, exit, seconds)
  if exit ~= 0 and seconds > 5 then
    dimaster.notify("Ошибка " .. exit, line)
  end
end)
```

Нужна интеграция shell.

## Чего плагины по-прежнему не могут

Изменить вывод программы до показа, получить щелчки мыши по нарисованному и читать строки старше последней тысячи. Долгая внешняя команда в `io.popen` или `os.execute` задержит окно: ограничение в 100 мс прерывает только код на Lua.
