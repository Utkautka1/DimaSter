-- Строка статуса DimaSter. Данные собирает Rust, а что и как показывать,
-- решает эта функция. Она вызывается при каждом обновлении данных.
--
-- info: mode, host, user, cwd, git_branch, git_changes, node, python, go,
--       process, cpu, memory, docker_running, docker_stopped. Нет данных — nil.
-- Возврат: { left = {...}, center = {...}, right = {...} }, где элемент —
-- строка или { text = "...", color = "#rrggbb", background = "#rrggbb",
-- rounded = true }. Все тексты и значки меняются здесь.
--
-- Пример «таблетки»:
--   { text = " u ", color = "#11111b", background = "#cba6f7", rounded = true }

local blue, green, yellow, gray = "#7aa2f7", "#9ece6a", "#e0af68", "#565f89"

dimaster.statusbar(function(info)
  local left = { { text = " " .. info.mode .. " ", color = blue }, " " .. (info.host or "") }

  local center = {}
  if info.git_branch then
    center[#center + 1] = { text = " " .. info.git_branch, color = green }
    if info.git_changes then
      center[#center + 1] = { text = " ±" .. info.git_changes, color = yellow }
    end
  end
  for _, tool in ipairs({ "node", "python", "go" }) do
    if info[tool] then center[#center + 1] = { text = "  " .. tool .. " " .. info[tool], color = gray } end
  end

  local right = {}
  if info.process then
    right[#right + 1] = info.process .. " " .. info.cpu .. "% " .. info.memory .. "  "
  end
  if info.docker_running then
    right[#right + 1] = { text = "🐳 " .. info.docker_running .. "▲ " .. info.docker_stopped .. "▼  ", color = gray }
  end
  right[#right + 1] = os.date("%H:%M") .. " "

  return { left = left, center = center, right = right }
end)
