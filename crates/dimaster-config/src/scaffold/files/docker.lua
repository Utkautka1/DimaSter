-- Контейнеры: dimaster docker (в терминале Ctrl+Alt+D)
-- Все ключи описаны в документации, раздел «Контейнеры».
return {
  enabled = true,
  refresh = 3,
  confirm = true,
  stats = true,
  log_tail = 500,
  query_timeout = 10,
  action_timeout = 120,
  ssh_persist = 60,
  tunnel_timeout = 10,
  tunnel_retry = 60,
  colors = { header = "blue", status = "magenta" },
  -- hosts = { "web", "db" }, -- ~/.ssh/config
  -- keys = { x = "quit", d = false },
  -- columns = { hidden = { "ports" } },
  -- commands = { { key = "t", tabs = { "containers" }, args = { "top", "{id}" } } },
}
