-- Применяется, когда DimaSter запущен внутри SSH-сессии (проброс X11/Wayland).
-- Эффекты по сети дороги, поэтому выключаются.
return {
  effects = { cursor_animation = false, cursor_trail = false, smooth_scroll = false },
}
