# Защитные пределы: `limits`

Задаются в любом файле настроек и плагинами. Увеличивая предел, вы снимаете защиту: программа в терминале, картинка или плагин смогут занять всю память или подвесить окно. Значения вне технических границ приводятся к ближайшей границе.

```lua
return {
  limits = {
    clipboard_bytes = 4194304,
  },
}
```

| Ключ | По умолчанию | Что будет при изменении |
|---|---|---|
| `escape_string_bytes` | 16777216 | больше — программа может занять всю память одной escape-строкой |
| `clipboard_bytes` | 1048576 | больше — программа может положить в буфер обмена огромный текст |
| `shell_text_bytes` | 4096 | больше — длинные заголовки и пути от программ занимают память |
| `hyperlinks` | 4096 | больше — растёт память под ссылки |
| `hyperlink_bytes` | 2048 | больше — программа может подсунуть огромный адрес |
| `hyperlink_sweep_pause` | 256 | меньше — частая очистка ссылок тормозит вывод |
| `image_memory_bytes` | 268435456 | больше — картинки могут занять всю память |
| `image_decode_bytes` | 268435456 | больше — декодирование картинок может занять всю память |
| `image_rows` | 500 | больше — медленная прокрутка с картинками |
| `images_per_row` | 64 | больше — медленная отрисовка строки |
| `kitty_payload_bytes` | 67108864 | больше — программа может прислать или прочитать огромный файл |
| `kitty_image_side` | 8192 | больше — огромная картинка займёт всю память |
| `keyboard_stack` | 16 | больше — программы копят режимы клавиатуры |
| `png_side` | 8192 | больше — огромный PNG займёт всю память |
| `sixel_side` | 4096 | больше — Sixel может занять всю память |
| `sixel_paint_budget` | 8000000 | больше — Sixel может надолго подвесить окно |
| `gif_side` | 4096 | больше — огромный GIF займёт всю память |
| `gif_bytes` | 268435456 | больше — анимация может занять всю память |
| `decoder_timeout` | 5 | секунды; больше — зависший ffmpeg дольше держит окно |
| `decoder_output_bytes` | 134217728 | больше — вывод ffmpeg может занять всю память |
| `remote_request_bytes` | 1048576 | больше — клиент dimaster ctl может занять память |
| `remote_timeout` | 3 | секунды; больше — зависший клиент дольше держит соединение |
| `reply_backlog_bytes` | 1048576 | больше — ответы программе, которая их не читает, копятся в памяти |
| `animation_frame_ms` | 16 | меньше — выше нагрузка на процессор и видеокарту |
| `sync_update_ms` | 150 | больше — зависшая программа дольше замораживает экран |
| `start_settle_ms` | 60 | меньше — shell может получить неверный размер окна |
| `bell_quiet_ms` | 300 | меньше — частый звонок плодит процессы |
| `bell_frame_px` | 4 | толщина рамки вспышки |
| `path_cache_secs` | 5 | меньше — подсветка команд чаще читает диск |
| `sensor_timeout` | 5 | секунды; больше — зависший датчик дольше не обновляется |
| `plugin_history_lines` | 1000 | больше — плагины получают больше вывода и тормозят |
| `plugin_overrides` | 64 | больше — настройки от плагинов копятся в памяти |
| `plugin_depth` | 4 | больше — плагины, вызывающие друг друга, могут подвесить окно |
| `ssh_include_depth` | 3 | больше — зацикленные Include в ssh_config читаются дольше |
| `plugin_effects` | 256 | больше — плагин может завалить окно действиями |
| `plugin_budget_ms` | 100 | больше — медленный плагин подвешивает окно |
| `plugin_shapes` | 2000 | больше — рисунок плагина тормозит отрисовку |
| `plugin_points` | 64 | больше — сложные фигуры тормозят отрисовку |
| `bar_rows` | 5 | больше — строка статуса закрывает терминал |
| `bar_segments` | 32 | больше — строка статуса тормозит отрисовку |
| `bar_pixels` | 200 | больше — отступы бара могут занять всё окно |
| `import_depth` | 4 | больше — зацикленный импорт читается дольше |
| `fetch_picture_cells` | 200 | больше — картинка fetch заполняет весь экран |
| `photo_samples` | 20000 | больше — дольше подбираются цвета по картинке |
| `shape_cache` | 8192 | меньше — медленнее отрисовка; больше — растёт память |
| `curve_steps` | 8 | меньше — угловатые буквы; больше — медленнее |
| `docker_log_lines` | 10000 | больше — журналы занимают память |
| `docker_log_line_bytes` | 16384 | больше — одна строка журнала занимает память |
| `docker_log_queue` | 1024 | меньше — журналы отстают |
| `docker_output_bytes` | 16777216 | больше — вывод docker может занять всю память |
| `view_command_bytes` | 65536 | больше — клиент view --send может занять память |
| `view_sample_rows` | 1000 | больше — дольше открываются таблицы |
| `table_query_secs` | 60 | меньше — долгие запросы view обрываются; больше — зависший держит программу |
| `table_result_bytes` | 1073741824 | больше — результат запроса может занять всю память |
| `regex_bytes` | 1048576 | больше — сложный поиск может занять всю память |
| `sixel_pixels` | 4000000 | больше — Sixel может занять всю память |
| `decoder_poll_ms` | 5 | меньше — выше нагрузка на процессор |
| `view_connect_secs` | 2 | секунды ожидания окна view --remote |
| `view_reply_secs` | 60 | секунды ожидания ответа view --send |
| `tui_poll_ms` | 100 | меньше — выше нагрузка; больше — экраны view и docker отстают |
| `tunnel_poll_ms` | 50 | меньше — выше нагрузка на процессор |
| `animation_step_ms` | 50 | больше — после паузы курсор прыгает |
| `plugin_label_chars` | 500 | больше — длинные подписи плагинов тормозят отрисовку |
| `min_wake_ms` | 16 | меньше — выше нагрузка на процессор |
| `redraw_wait_ms` | 40 | больше — анимации картинок идут рывками |
| `scrollback_max` | 1000000 | больше — история может занять всю память |
| `session_lines_max` | 100000 | больше — сохранённая сессия занимает диск |
| `panel_radius_max` | 200 | наибольший радиус углов панелей |
| `panel_padding_max` | 100 | наибольший внутренний отступ панелей и заголовка |
| `palette_columns_max` | 500 | наибольшая ширина палитры в ячейках |
| `list_rows_max` | 100 | наибольшее число строк в списках |
| `strip_width_max` | 400 | наибольшая ширина полосы прокрутки и миникарты |
| `notice_min_ms` | 200 | наименьшее время плашки |
| `notice_max_ms` | 60000 | наибольшее время плашки |
| `notice_margin_max` | 2000 | наибольший отступ плашки |
| `window_cells_max` | 1000 | наибольший начальный размер окна в ячейках |
| `fetch_logo_width_max` | 200 | наибольшая ширина логотипа fetch |
| `scrollbar_thumb_px` | 12 | наименьшая высота ползунка |
| `scrollbar_mark_px` | 2 | высота метки на полосе прокрутки |
| `minimap_row_px` | 2 | высота одной полоски миникарты |
| `minimap_view_px` | 4 | наименьшая высота рамки миникарты и поля слева |
| `progress_px` | 3 | толщина полоски выполнения |
| `glow_px` | 4 | средняя толщина пульсирующей рамки |
| `glow_wave_px` | 3 | размах пульсации рамки |
| `history_columns_min` | 30 | наименьшая ширина панели истории в ячейках |
| `monitor_gap_px` | 20 | зазор между монитором и миникартой |
| `monitor_margin_px` | 4 | отступ монитора от края |
| `skins_columns` | 46 | ширина витрины скинов в ячейках |
| `skins_rows` | 14 | строк в витрине скинов |
| `presentation_side_divisor` | 10 | поля презентации: ширина окна делится на это число |
| `presentation_top_px` | 24 | наименьшее поле презентации сверху |
| `photo_shade_min` | 20 | наименьшее затемнение фото, проценты |
| `photo_shade_max` | 60 | наибольшее затемнение фото, проценты |
| `scene_width` | 960 | ширина рисованного фона в пикселях |
| `scene_height` | 600 | высота рисованного фона в пикселях |
| `underline_wave_room` | 3 | место под волной: толщин линии от низа ячейки |
| `underline_double_gap` | 2 | шаг двойного подчёркивания в толщинах линии |
| `underline_dot_step` | 2 | шаг точек подчёркивания в толщинах линии |
| `underline_dash_period` | 50 | шаг штрихов, проценты ширины ячейки |
| `underline_dash_length` | 33 | длина штриха, проценты ширины ячейки |
| `strike_position` | 67 | высота зачёркивания, проценты от базовой линии |
| `hollow_cursor_lines` | 2 | толщина контура курсора в толщинах линии |
| `smooth_scroll_rows` | 3 | наибольшая плавная прокрутка в строках |
| `light_background_percent` | 50 | яркость фона, ниже которой текст для контраста светлеет |
| `skins_swatch_top` | 6 | отступ образца цвета: строка делится на это число |
| `skins_swatch_cut` | 3 | насколько образец ниже строки: строка делится на это число |
| `skins_swatch_cells` | 7 | ширина подложки образцов в ячейках |
| `skins_back_percent` | 40 | левая доля строки стиля, щелчок по которой листает назад |
| `opacity_min_percent` | 0 | наименьшая разрешённая прозрачность, проценты |
| `opacity_max_percent` | 100 | наибольшая разрешённая прозрачность, проценты; выше 100 на экране не отличается |
| `history_width_min` | 20 | наименьшая ширина панели истории, проценты окна |
| `history_width_max` | 100 | наибольшая ширина панели истории, проценты окна |

```lua
return {
  limits = {
    escape_string_bytes = 16777216,
    clipboard_bytes = 1048576,
    shell_text_bytes = 4096,
    hyperlinks = 4096,
    hyperlink_bytes = 2048,
    hyperlink_sweep_pause = 256,
    image_memory_bytes = 268435456,
    image_decode_bytes = 268435456,
    image_rows = 500,
    images_per_row = 64,
    kitty_payload_bytes = 67108864,
    kitty_image_side = 8192,
    keyboard_stack = 16,
    png_side = 8192,
    sixel_side = 4096,
    sixel_paint_budget = 8000000,
    gif_side = 4096,
    gif_bytes = 268435456,
    decoder_timeout = 5,
    decoder_output_bytes = 134217728,
    remote_request_bytes = 1048576,
    remote_timeout = 3,
    reply_backlog_bytes = 1048576,
    animation_frame_ms = 16,
    sync_update_ms = 150,
    start_settle_ms = 60,
    bell_quiet_ms = 300,
    bell_frame_px = 4,
    path_cache_secs = 5,
    sensor_timeout = 5,
    plugin_history_lines = 1000,
    plugin_overrides = 64,
    plugin_depth = 4,
    ssh_include_depth = 3,
    plugin_effects = 256,
    plugin_budget_ms = 100,
    plugin_shapes = 2000,
    plugin_points = 64,
    bar_rows = 5,
    bar_segments = 32,
    bar_pixels = 200,
    import_depth = 4,
    fetch_picture_cells = 200,
    photo_samples = 20000,
    shape_cache = 8192,
    curve_steps = 8,
    docker_log_lines = 10000,
    docker_log_line_bytes = 16384,
    docker_log_queue = 1024,
    docker_output_bytes = 16777216,
    view_command_bytes = 65536,
    view_sample_rows = 1000,
    table_query_secs = 60,
    table_result_bytes = 1073741824,
    regex_bytes = 1048576,
    sixel_pixels = 4000000,
    decoder_poll_ms = 5,
    view_connect_secs = 2,
    view_reply_secs = 60,
    tui_poll_ms = 100,
    tunnel_poll_ms = 50,
    animation_step_ms = 50,
    plugin_label_chars = 500,
    min_wake_ms = 16,
    redraw_wait_ms = 40,
    scrollback_max = 1000000,
    session_lines_max = 100000,
    panel_radius_max = 200,
    panel_padding_max = 100,
    palette_columns_max = 500,
    list_rows_max = 100,
    strip_width_max = 400,
    notice_min_ms = 200,
    notice_max_ms = 60000,
    notice_margin_max = 2000,
    window_cells_max = 1000,
    fetch_logo_width_max = 200,
    scrollbar_thumb_px = 12,
    scrollbar_mark_px = 2,
    minimap_row_px = 2,
    minimap_view_px = 4,
    progress_px = 3,
    glow_px = 4,
    glow_wave_px = 3,
    history_columns_min = 30,
    monitor_gap_px = 20,
    monitor_margin_px = 4,
    skins_columns = 46,
    skins_rows = 14,
    presentation_side_divisor = 10,
    presentation_top_px = 24,
    photo_shade_min = 20,
    photo_shade_max = 60,
    scene_width = 960,
    scene_height = 600,
    underline_wave_room = 3,
    underline_double_gap = 2,
    underline_dot_step = 2,
    underline_dash_period = 50,
    underline_dash_length = 33,
    strike_position = 67,
    hollow_cursor_lines = 2,
    smooth_scroll_rows = 3,
    light_background_percent = 50,
    skins_swatch_top = 6,
    skins_swatch_cut = 3,
    skins_swatch_cells = 7,
    skins_back_percent = 40,
    opacity_min_percent = 0,
    opacity_max_percent = 100,
    history_width_min = 20,
    history_width_max = 100,
  },
}
```
