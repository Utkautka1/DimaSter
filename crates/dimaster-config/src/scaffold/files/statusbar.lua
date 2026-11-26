-- Строка статуса DimaSter. Данные собирает Rust, а что и как показывать,
-- решает эта функция. Она вызывается при каждом обновлении данных.
--
-- info: mode, host, user, cwd, git_branch, git_changes, node, python, go,
--       process, cpu, memory, docker_running, docker_stopped. Нет данных — nil.
-- Возврат: { left = {...}, center = {...}, right = {...} }, где элемент —
-- строка или { text = "...", color = "#rrggbb", background = "#rrggbb",
-- rounded = true }. Все тексты, значки и цвета меняются здесь.

-- палитра: тёмный текст на светлых «таблетках» и наоборот
local ink = "#11111b"
local mauve, lavender, pink, plum = "#cba6f7", "#b4befe", "#f5c2e7", "#3e385c"
local light = "#e8e5ff"

local function pill(text, background, color)
  return { text = " " .. text .. " ", color = color or ink, background = background, rounded = true }
end

-- «таблетки» с зазором в один пробел
local function row(items)
  local result = {}
  for index, item in ipairs(items) do
    if index > 1 then result[#result + 1] = " " end
    result[#result + 1] = item
  end
  return result
end

dimaster.statusbar(function(info)
  local left = { pill(info.mode, mauve), pill(info.host or "?", plum, light) }

  local center = {}
  if info.git_branch then
    local changes = info.git_changes and (" ±" .. info.git_changes) or ""
    center[#center + 1] = pill(info.git_branch .. changes, lavender)
  end
  for _, tool in ipairs({ "node", "python", "go" }) do
    if info[tool] then center[#center + 1] = pill(tool .. " " .. info[tool], pink) end
  end

  local right = {}
  if info.process then
    right[#right + 1] = pill(info.process .. " " .. info.cpu .. "% " .. info.memory, plum, light)
  end
  -- контейнеры показываются, только когда они есть
  local running, stopped = tonumber(info.docker_running), tonumber(info.docker_stopped)
  if running and running + stopped > 0 then
    right[#right + 1] = pill("🐳 " .. running .. " ▲ " .. stopped .. " ▼", plum, light)
  end
  right[#right + 1] = pill("󱑒 " .. os.date("%H:%M"), ink, mauve)

  return { left = row(left), center = row(center), right = row(right) }
end)
