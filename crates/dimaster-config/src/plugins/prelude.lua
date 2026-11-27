-- Часть API плагинов, написанная на самом Lua: реестр обработчиков и команд.
-- Ошибка одного плагина записывается в лог и не мешает остальным.

local handlers, commands, order = {}, {}, {}

function dimaster.on(event, callback)
  assert(type(callback) == "function", dimaster.tr("dimaster.on: нужна функция"))
  handlers[event] = handlers[event] or {}
  table.insert(handlers[event], callback)
end

function dimaster.command(name, title, callback)
  assert(type(callback) == "function", dimaster.tr("dimaster.command: нужна функция"))
  if not commands[name] then order[#order + 1] = name end
  commands[name] = { title = title, run = callback }
end

local function guard(what, callback, ...)
  local ok, problem = pcall(callback, ...)
  if not ok then dimaster.log(what .. ": " .. tostring(problem)) end
end

local statusbar

function dimaster.statusbar(callback)
  assert(type(callback) == "function", dimaster.tr("dimaster.statusbar: нужна функция"))
  statusbar = callback
end

local drawing

function dimaster.draw(callback)
  assert(type(callback) == "function", dimaster.tr("dimaster.draw: нужна функция"))
  drawing = callback
end

function dimaster.circle(c)
  return { kind = "rect", x = c.x - c.r, y = c.y - c.r, width = c.r * 2, height = c.r * 2,
    radius = c.r, color = c.color, opacity = c.opacity }
end

-- ломаная по значениям от 0 до max (по умолчанию — наибольшее из них)
function dimaster.graph(g)
  local values, top = g.values or {}, g.max or 0
  if not g.max then
    for _, value in ipairs(values) do top = math.max(top, value) end
  end
  local function point(i)
    local share = math.min(math.max(values[i] / math.max(top, 1e-9), 0), 1)
    return g.x + g.width * (i - 1) / (#values - 1), g.y + g.height * (1 - share)
  end
  local lines = {}
  for i = 2, #values do
    local x1, y1 = point(i - 1)
    local x2, y2 = point(i)
    lines[#lines + 1] = { kind = "line", x1 = x1, y1 = y1, x2 = x2, y2 = y2,
      width = g.line_width or 2, color = g.color, opacity = g.opacity }
  end
  return lines
end

local bound, timers = {}, {}
local MAX_BOUND, MAX_TIMERS = 128, 16

-- нажатие вида "ctrl+shift+e"; вернув false, обработчик отдаёт клавишу дальше
function dimaster.bind(chord, callback)
  assert(type(chord) == "string" and type(callback) == "function", dimaster.tr("dimaster.bind: нужны клавиша и функция"))
  if #bound < MAX_BOUND then bound[#bound + 1] = { chord = chord, run = callback } end
end

function dimaster.every(seconds, callback)
  assert(type(seconds) == "number" and type(callback) == "function", dimaster.tr("dimaster.every: нужны секунды и функция"))
  -- чаще раза в секунду окно не будится
  if #timers < MAX_TIMERS then timers[#timers + 1] = { seconds = math.max(seconds, 1), run = callback } end
end

-- таблица настроек превращается в литерал: так она переживает перезагрузку
local function literal(value, depth)
  local kind = type(value)
  if kind == "string" then return string.format("%q", value) end
  if kind == "boolean" or (kind == "number" and value == value and math.abs(value) < math.huge) then
    return tostring(value)
  end
  assert(kind == "table" and depth < 8, dimaster.tr("dimaster.set: нужны числа, строки, флаги и таблицы"))
  local parts = {}
  for _, item in ipairs(value) do parts[#parts + 1] = literal(item, depth + 1) end
  for key, item in pairs(value) do
    if type(key) == "string" then
      parts[#parts + 1] = "[" .. string.format("%q", key) .. "]=" .. literal(item, depth + 1)
    end
  end
  return "{" .. table.concat(parts, ",") .. "}"
end

function dimaster.set(settings)
  assert(type(settings) == "table", dimaster.tr("dimaster.set: нужна таблица"))
  dimaster.set_literal(literal(settings, 0))
end

local registry = {}

function registry.keys()
  local list = {}
  for index, entry in ipairs(bound) do list[index] = entry.chord end
  return list
end

function registry.key(index)
  local entry = bound[index]
  if not entry then return false end
  local ok, result = pcall(entry.run)
  if not ok then dimaster.log(entry.chord .. ": " .. tostring(result)) end
  return ok and result ~= false
end

function registry.intervals()
  local list = {}
  for index, entry in ipairs(timers) do list[index] = entry.seconds end
  return list
end

function registry.tick(index)
  local entry = timers[index]
  if entry then guard(dimaster.tr("таймер ") .. index, entry.run) end
end

function registry.draw(info)
  if drawing then return drawing(info) end
end

function registry.status(info)
  if statusbar then return statusbar(info) end
end

function registry.emit(event, ...)
  for _, callback in ipairs(handlers[event] or {}) do
    guard(dimaster.tr("событие ") .. event, callback, ...)
  end
end

function registry.run(name)
  local command = commands[name]
  if command then guard(dimaster.tr("команда ") .. name, command.run) end
end

function registry.list()
  local list = {}
  for _, name in ipairs(order) do
    list[#list + 1] = { name, commands[name].title }
  end
  return list
end

return registry
