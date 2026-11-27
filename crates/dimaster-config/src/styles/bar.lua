-- Строка статуса стиля из витрины. Перед этим текстом подставляются
-- STYLE (имя стиля) и THEME (цвета выбранного скина).

local looks = {
  glass = { chip = { radius = 12 }, bar = { margin = { x = 18, y = 10 }, padding = { x = 8, y = 5 }, background = THEME.bg, radius = 18, opacity = 0.7 } },
  neon = { chip = { left = "\u{e0b2}", right = "\u{e0b0}" }, bar = { margin = { x = 0, y = 0 } } },
  minimal = { flat = true, bar = { margin = { x = 40, y = 8 } } },
  retro = { chip = { left = "[", right = "]" }, flat = true, bar = { margin = { x = 8, y = 4 } } },
  mac = { chip = { radius = 7 }, bar = { position = "top", margin = { x = 14, y = 6 } } },
  sunset = { chip = { left = "\u{e0b6}", right = "\u{e0b4}" }, bar = { margin = { x = 16, y = 10 } } },
  city = { chip = { radius = 3 }, bar = { margin = { x = 0, y = 0 }, padding = { x = 12, y = 6 }, background = THEME.bg, opacity = 0.85 } },
  petals = { chip = { radius = 14 }, bar = { position = "top", margin = { x = 30, y = 10 } } },
}

local look = looks[STYLE] or looks.glass

local function chip(text, accent)
  if not text or text == "" then return nil end
  if look.flat then
    local style = look.chip or {}
    return { text = (style.left or "") .. text .. (style.right or "") .. "  ", color = accent }
  end
  local item = { text = " " .. text .. " ", color = THEME.bg, background = accent, radius = look.chip.radius }
  item.left_cap, item.right_cap = look.chip.left, look.chip.right
  return item
end

local function list(...)
  local out = {}
  for i = 1, select("#", ...) do out[#out + 1] = (select(i, ...)) end
  return out
end

dimaster.statusbar(function(info)
  local bar = {}
  for key, value in pairs(look.bar) do bar[key] = value end
  bar.left = list(chip(info.mode, THEME.ansi[5]), chip(info.host, THEME.ansi[7]))
  bar.center = list(chip(info.git_branch, THEME.ansi[3]))
  bar.right = list(chip(info.cpu and (info.cpu .. "%"), THEME.ansi[4]), chip(os.date("%H:%M"), THEME.ansi[6]))
  return bar
end)
