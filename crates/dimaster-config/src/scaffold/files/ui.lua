-- Вид всего, что рисуется поверх терминала. Здесь перечислены все ключи со
-- значениями по умолчанию: уберите «--» перед нужной строкой и поменяйте.
-- Цвета — "#rrggbb"; не заданный цвет берётся из темы.
-- Строка статуса настраивается отдельно, в statusbar.lua.

return {
  ui = {
    -- prompt_position = "bottom", -- строка поиска и подсказки: top | bottom

    panel = { -- палитра, история, поиск, монитор, адрес ссылки
      -- background = "#292b31",
      -- color = "#d8dee9",
      -- selected_background = "#d8dee9",
      -- selected_color = "#11131a",
      -- radius = 0,
      -- padding = 0,
      -- opacity = 1.0,
    },

    title = { -- своя полоса заголовка
      -- padding = 8,
      -- format = "{title}",
      -- align = "left", -- left | center | right
      -- background = "#11131a",
      -- color = "#d8dee9",
      -- buttons_side = "right",
      -- buttons = { { action = "minimize", text = " — " }, { action = "maximize", text = " □ " }, { action = "close", text = " ✕ " } },
    },

    palette = {
      -- width = 50,
      -- rows = 11,
      -- position = "top", -- top | center | bottom
      -- prompt = " > ",
      -- items = { { title = "Копировать", action = "copy" } }, -- свой список команд
    },

    history = {
      -- width = 0.6,
      -- rows = 10,
      -- side = "right",
    },

    marks = { -- метки на ссылках и подсветки
      -- hint_color = "#11131a",
      -- hint_background = "#e0af68",
      -- found = "#6b5a1e",
      -- current = "#b0620f",
      -- link = "#2f4a7a",
    },

    scrollbar = {
      -- width = 5,
      -- color = "#9aa5ce",
      -- command = "#7aa2f7",
      -- error = "#f7768e",
      -- found = "#e0af68",
      -- side = "right",
    },

    minimap = {
      -- width = 90,
      -- color = "#9aa5ce",
      -- command = "#7aa2f7",
      -- error = "#f7768e",
      -- side = "right",
    },

    monitor = {
      -- side = "right",
      -- position = "top",
      -- format = " {process}  CPU {cpu}%  {memory} ",
      -- waiting = " ожидание ",
    },

    notice = { -- плашка после копирования
      -- copy = true,
      -- text = " ✓ Скопировано: {count} ",
      -- duration = 1200,
      -- side = "right",
      -- position = "bottom",
      -- margin = { x = 10, y = 4 },
      -- свой вид (иначе как у panel): background, color, radius, padding, opacity
    },

    texts = {
      -- vi = " VI: hjkl w b 0 $ g G · v V · y копировать · / поиск · q выход ",
      -- hint = " Метка — открыть, ЗАГЛАВНАЯ — копировать, Esc ",
      -- minimap = " Карта: j/k прокрутка · g/G края · Enter здесь · Esc назад ",
      -- search = " Поиск: {query}_  {position}/{count}  [{kind}, Ctrl+R] ",
      -- search_plain = "текст",
      -- search_regex = "regex",
      -- history = " j/k выбор · Enter вставить · y вывод · q",
      -- history_output = "вывод",
      -- history_failed = "код {code}: {reason}",
    },
  },
}
