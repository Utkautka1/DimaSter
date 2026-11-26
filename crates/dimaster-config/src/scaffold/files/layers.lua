-- Какие файлы и в каком порядке участвуют в настройках.
--
-- Отсутствующий файл просто пропускается, ошибка внутри файла — показывается.
-- Глобальная таблица dimaster даёт контекст: hostname, desktop, session,
-- ssh, profile, config_dir.

local layers = {}

-- базовые настройки, по одному файлу на тему
local SETTINGS = { "appearance", "font", "window", "effects", "behavior", "keys" }

local function names(context)
  local list = {}
  for _, name in ipairs(SETTINGS) do list[#list + 1] = "settings." .. name end
  -- вид интерфейса: профили, хост и local.lua могут его перекрыть
  list[#list + 1] = "ui"
  -- окружение: композитор, дисплейный сервер, удалённая сессия
  if context.desktop ~= "" then list[#list + 1] = "profiles.desktop." .. context.desktop end
  if context.session ~= "" then list[#list + 1] = "profiles.session." .. context.session end
  if context.ssh then list[#list + 1] = "profiles.ssh" end
  -- явный выбор: DIMASTER_PROFILE=presentation dimaster
  if context.profile ~= "" then list[#list + 1] = "profiles." .. context.profile end
  -- конкретная машина и личные правки — самые сильные
  if context.hostname ~= "" then list[#list + 1] = "hosts." .. context.hostname end
  list[#list + 1] = "local"
  return list
end

local function optional(name)
  if not package.searchpath(name, package.path) then return nil end
  local layer = require(name)
  assert(type(layer) == "table", name .. ".lua должен возвращать таблицу")
  return layer
end

function layers.load(context)
  local result = {}
  for _, name in ipairs(names(context)) do
    local layer = optional(name)
    if layer then result[#result + 1] = layer end
  end
  return result
end

return layers
