-- Часть API плагинов, написанная на самом Lua: реестр обработчиков и команд.
-- Ошибка одного плагина записывается в лог и не мешает остальным.

local handlers, commands, order = {}, {}, {}

function dimaster.on(event, callback)
  assert(type(callback) == "function", "dimaster.on: нужна функция")
  handlers[event] = handlers[event] or {}
  table.insert(handlers[event], callback)
end

function dimaster.command(name, title, callback)
  assert(type(callback) == "function", "dimaster.command: нужна функция")
  if not commands[name] then order[#order + 1] = name end
  commands[name] = { title = title, run = callback }
end

local function guard(what, callback, ...)
  local ok, problem = pcall(callback, ...)
  if not ok then dimaster.log(what .. ": " .. tostring(problem)) end
end

local registry = {}

function registry.emit(event, ...)
  for _, callback in ipairs(handlers[event] or {}) do
    guard("событие " .. event, callback, ...)
  end
end

function registry.run(name)
  local command = commands[name]
  if command then guard("команда " .. name, command.run) end
end

function registry.list()
  local list = {}
  for _, name in ipairs(order) do
    list[#list + 1] = { name, commands[name].title }
  end
  return list
end

return registry
