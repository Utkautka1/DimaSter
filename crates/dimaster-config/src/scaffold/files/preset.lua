-- Вид DimaSter из коробки: с него начинает мастер настройки.
return {
  padding = { x = 25, y = 20 },
  cursor = { shape = "bar", pulse = true, pulse_period = 1500, pulse_min = 0.15, thickness = 2, height = 0.9 },
  font = { family = "JetBrainsMono Nerd Font", size = 11 },
  window = { decorations = "none", opacity = 0.65, blur = true, background_opacity = 0.3 },
  effects = { cursor_blink = false, cursor_animation = true, cursor_trail = false, smooth_scroll = true },
  rtl = true,
  smart = { fetch_early = true },
}
