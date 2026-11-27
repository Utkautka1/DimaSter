-- Строка статуса стиля из витрины: у каждого стиля свой набор элементов и
-- своё расположение. Перед этим текстом подставляются STYLE и THEME.

local A = THEME.ansi
local home = os.getenv("HOME") or ""

local function join(...)
  local out = {}
  for i = 1, select("#", ...) do
    local item = select(i, ...)
    if item then out[#out + 1] = item end
  end
  return out
end

-- «таблетка» с подложкой; пустое значение не рисуется
local function pill(text, accent, shape)
  if not text or text == "" then return nil end
  shape = shape or {}
  return { text = " " .. text .. " ", color = THEME.bg, background = accent,
    radius = shape.radius, left_cap = shape.left, right_cap = shape.right }
end

local function word(text, accent)
  if not text or text == "" then return nil end
  return { text = text .. "  ", color = accent }
end

local function path(info)
  if not info.cwd or info.cwd == "" then return nil end
  if home ~= "" and info.cwd:sub(1, #home) == home then return "~" .. info.cwd:sub(#home + 1) end
  return info.cwd
end

local function labelled(label, value, suffix)
  if not value or value == "" then return nil end
  return label .. " " .. value .. (suffix or "")
end

local function tool(info)
  return labelled("node", info.node) or labelled("py", info.python) or labelled("go", info.go)
end

local layouts = {}

-- парящая стеклянная капсула снизу: режим и путь, ветка, часы
function layouts.glass(info)
  local round = { radius = 12 }
  return { margin = { x = 18, y = 10 }, padding = { x = 8, y = 5 }, background = THEME.bg, radius = 18, opacity = 0.7,
    left = join(pill(info.mode, A[5], round), word(" " .. (path(info) or ""), THEME.fg)),
    center = join(pill(info.git_branch, A[3], round)),
    right = join(pill(os.date("%H:%M"), A[6], round)) }
end

-- приборная панель сверху во всю ширину: всё о системе острыми стрелками
function layouts.neon(info)
  local arrow = { left = "\u{e0b2}", right = "\u{e0b0}" }
  return { position = "top", margin = { x = 0, y = 0 },
    left = join(pill(info.mode, A[6], arrow), pill(info.host, A[7], arrow), pill(path(info), A[5], arrow)),
    center = join(pill(info.git_branch, A[3], arrow), pill(tool(info), A[4], arrow)),
    right = join(pill(labelled("CPU", info.cpu, "%"), A[2], arrow), pill(info.memory, A[4], arrow),
      pill(os.date("%H:%M:%S"), A[6], arrow)) }
end

-- две строки внизу, как в старых программах: кто и где, что запущено
function layouts.retro(info)
  local function box(text) return text and ("[ " .. text .. " ]") end
  return { margin = { x = 8, y = 4 }, gap = 2, rows = {
    { left = join(word(box((info.user or "") .. "@" .. (info.host or "")), A[3])),
      right = join(word(box(os.date("%d.%m.%Y %H:%M")), A[4])) },
    { left = join(word(box(path(info)), A[5]), word(box(info.git_branch), A[6])),
      right = join(word(box(labelled("proc", info.process)), A[3]), word(box(labelled("cpu", info.cpu, "%")), A[2]),
        word(box(labelled("mem", info.memory)), A[7])) } } }
end

-- один островок по центру: дата, ветка, время
function layouts.sunset(info)
  local soft = { left = "\u{e0b6}", right = "\u{e0b4}" }
  return { margin = { x = 16, y = 14 },
    center = join(pill(os.date("%a %d %b"), A[2], soft), word(" ", THEME.fg),
      pill(info.git_branch, A[4], soft), word(" ", THEME.fg), pill(os.date("%H:%M"), A[6], soft)) }
end

-- две строки мониторинга: ресурсы и контейнеры, ниже путь и режим
function layouts.city(info)
  local box = { radius = 3 }
  return { margin = { x = 0, y = 0 }, padding = { x = 12, y = 6 }, background = THEME.bg, opacity = 0.85, gap = 4, rows = {
    { left = join(pill(info.mode, A[5], box), word(" " .. (path(info) or ""), THEME.fg)),
      right = join(word(info.git_branch and (" " .. info.git_branch), A[3]), word(tool(info), A[4])) },
    { left = join(word(labelled("▣ CPU", info.cpu, "%"), A[2]), word(labelled("▤ RAM", info.memory), A[4]),
        word(labelled("◆ docker", info.docker_running), A[7])),
      center = join(word(info.process, A[6])),
      right = join(word((info.user or "") .. "@" .. (info.host or ""), A[8]), word(os.date("%H:%M"), A[5])) } } }
end

-- только путь и время, по центру сверху
function layouts.petals(info)
  local round = { radius = 14 }
  return { position = "top", margin = { x = 30, y = 10 },
    center = join(pill(path(info), A[6], round), word("  ✿  ", A[14]), pill(os.date("%H:%M"), A[14], round)) }
end

local layout = layouts[STYLE] or layouts.glass

dimaster.statusbar(function(info) return layout(info) end)
