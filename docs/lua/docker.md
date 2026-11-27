# Контейнеры

```sh
dimaster docker
```

В окне терминала — `Ctrl+Alt+D` или «Docker: управление» в палитре команд (действие `containers`): экран откроется в новом окне. Прежний быстрый список по `Ctrl+Shift+D` остался.

Вкладки: контейнеры, compose-проекты, образы, тома, сети. Контейнеры одного проекта стоят рядом. Первый сервер — локальный (с учётом `docker context`), остальные берутся из `~/.ssh/config` и открываются через `docker -H ssh://имя`.

Для сервера нужен вход по ключу: пароль внутри экрана не спрашивается (кроме shell внутри контейнера). Новый сервер сначала добавь в `known_hosts` обычным `ssh имя`.

## Клавиши

| Клавиша | Действие | Имя |
|---|---|---|
| `j` `k` `g` `G`, `PageUp` `PageDown` | перемещение | `down` `up` `top` `bottom` `page_up` `page_down` |
| `Tab`, `Shift+Tab` | вкладки | `next_tab` `previous_tab` |
| `h`, `H` | сервер | `next_host` `previous_host` |
| `/` | фильтр | `filter` |
| `s` | старт или стоп | `toggle` |
| `r` | перезапуск | `restart` |
| `p` | пауза или продолжить | `pause` |
| `l` | живые логи | `logs` |
| `Enter` | shell внутри контейнера | `shell` |
| `i` | `inspect` | `inspect` |
| `d` | удалить (для проекта — `compose down`) | `remove` |
| `x` | `compose down` | `down_project` |
| `u` | `compose up` (только локально) | `up` |
| `D` | удалить всё неиспользуемое на вкладке | `prune` |
| `R` | обновить сейчас | `refresh` |
| `q` | выход или назад | `quit` |

Перед `d`, `x`, `D` спрашивается подтверждение: выполняется только по `y`.

## docker.lua

Файл `~/.config/dimaster/docker.lua`:

```lua
return {
  enabled = true,
  refresh = 3,          -- секунды
  confirm = true,       -- спрашивать перед удалением
  stats = true,         -- CPU и память
  log_tail = 500,       -- сколько прошлых строк логов показать
  shell = { "sh", "-c", "command -v bash >/dev/null && exec bash || exec sh" },
  hosts = { "web" },    -- вместо всех серверов из ~/.ssh/config
  keys = { x = "quit", d = false },
  commands = {
    -- docker top <id>, вывод показывается на экране
    { key = "t", tabs = { "containers" }, args = { "top", "{id}" } },
  },
  columns = { hidden = { "ports" } },
  tables = {
    containers = {
      columns = {
        order = { "name", "state", "cpu" },
        cpu = { align = "right", color = "yellow" },
        name = { title = "контейнер", width = 30 },
      },
    },
  },
}
```

Колонки, `colors`, `max_width`, `gap`, функции `format` и `status` устроены как в [view.lua](view.md). Имена таблиц: `containers`, `compose`, `images`, `volumes`, `networks`. Функция `status` получает `host`, `tab`, `rows`, `message`.

Свои команды — это только аргументы `docker`: shell на твоей машине не запускается. `{id}` и `{name}` заменяются значениями выбранной строки.
