-- Что и как показывает `dimaster fetch`. Данные о системе собирает Rust,
-- а оформление целиком здесь: меняйте порядок, названия, цвета и логотип.
--
-- info: user, host, os, kernel, uptime, packages, shell, desktop, session,
--       terminal, cpu, gpu, memory, disk. Чего узнать не удалось — nil.
-- tr(текст): перевод на язык терминала.
-- color(имя, текст): black red green yellow blue magenta cyan white, bold.
-- dimaster: hostname, desktop, session, ssh, profile, config_dir.
--
-- Результат: { lines = {...}, logo = {...}, image = "путь", logo_width = 22 }

local groups = {
  {
    { "󰣇 OS", "os", "blue" },
    { "󰒋 Kernel", "kernel", "magenta" },
    { "󱑒 Uptime", "uptime", "green" },
    { "󰏖 Packages", "packages", "yellow" },
    { "󱆃 Shell", "shell", "cyan" },
  },
  {
    { "󰻠 CPU", "cpu", "red" },
    { "󰢮 GPU", "gpu", "yellow" },
    { "󰍛 Memory", "memory", "magenta" },
    { "󰋊 Disk", "disk", "green" },
  },
}

local logo = {
  " ██████╗  ███████╗",
  " ██╔══██╗ ██╔════╝",
  " ██║  ██║ ███████╗",
  " ██║  ██║ ╚════██║",
  " ██████╔╝ ███████║",
  " ╚═════╝  ╚══════╝",
  "     DimaSter",
}

for index, line in ipairs(logo) do logo[index] = color("cyan", line) end

local title = color("bold", color("magenta", info.user))
  .. color("bold", "@")
  .. color("bold", color("blue", info.host))
local lines = { title, "" }

for g, rows in ipairs(groups) do
  for _, row in ipairs(rows) do
    local value = info[row[2]]
    if value then
      lines[#lines + 1] = color("bold", color(row[3], row[1])) .. " ➜  " .. value
    end
  end
  if g < #groups then lines[#lines + 1] = "" end
end

local dots = { 90, 31, 32, 33, 34, 35, 36, 37 }
local palette = ""
for _, code in ipairs(dots) do palette = palette .. "\27[" .. code .. "m● " end
lines[#lines + 1] = ""
lines[#lines + 1] = palette .. "\27[0m"

return {
  lines = lines,
  logo = logo,
  -- image = "~/Pictures/logo.png", -- картинка вместо текстового логотипа
  logo_width = 22,
}
