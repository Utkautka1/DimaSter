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

-- какие строки показывать и в каком порядке
local rows = {
  { "ОС", "os" },
  { "Ядро", "kernel" },
  { "Аптайм", "uptime" },
  { "Пакеты", "packages" },
  { "Shell", "shell" },
  { "Окружение", "desktop" },
  { "Терминал", "terminal" },
  { "Процессор", "cpu" },
  { "Видеокарта", "gpu" },
  { "Память", "memory" },
  { "Диск /", "disk" },
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

local lines = { color("bold", color("magenta", info.user .. "@" .. info.host)), "" }

for _, row in ipairs(rows) do
  local title, value = row[1], info[row[2]]
  if value then
    lines[#lines + 1] = color("bold", color("blue", tr(title))) .. ": " .. value
  end
end

local palette = ""
for code = 40, 47 do palette = palette .. "\27[" .. code .. "m   " end
lines[#lines + 1] = ""
lines[#lines + 1] = palette .. "\27[0m"

return {
  lines = lines,
  logo = logo,
  -- image = "~/Pictures/logo.png", -- картинка вместо текстового логотипа
  logo_width = 22,
}
