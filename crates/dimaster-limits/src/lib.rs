//! Защитные пределы и внутренние значения, изменяемые из Lua (`limits`).
//!
//! Один набор на процесс: пределы читаются в разборе вывода, декодерах и
//! фоновых потоках, куда конфиг окна не передаётся. Атомики — чтобы чтение
//! на горячем пути стоило как обычная загрузка из памяти.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Один предел: имя в Lua, значение по умолчанию, границы и последствия.
pub struct Limit {
    pub name: &'static str,
    pub default: u64,
    // уже или шире этих границ код упал бы, а не просто стал небезопасным
    pub min: u64,
    pub max: u64,
    /// Что защищает и что будет при изменении.
    pub risk: &'static str,
    value: AtomicU64,
}

impl Limit {
    const fn new(name: &'static str, [default, min, max]: [u64; 3], risk: &'static str) -> Self {
        let value = AtomicU64::new(default);
        Self {
            name,
            default,
            min,
            max,
            risk,
            value,
        }
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn set(&self, value: u64) {
        let value = value.clamp(self.min, self.max);
        self.value.store(value, Ordering::Relaxed);
    }

    pub fn size(&self) -> usize {
        usize::try_from(self.get()).unwrap_or(usize::MAX)
    }

    pub fn u32(&self) -> u32 {
        u32::try_from(self.get()).unwrap_or(u32::MAX)
    }

    pub fn u16(&self) -> u16 {
        u16::try_from(self.get()).unwrap_or(u16::MAX)
    }

    pub fn u8(&self) -> u8 {
        u8::try_from(self.get()).unwrap_or(u8::MAX)
    }

    pub fn millis(&self) -> Duration {
        Duration::from_millis(self.get())
    }

    pub fn secs(&self) -> Duration {
        Duration::from_secs(self.get())
    }
}

/// Задаёт перечисленные пределы, остальные возвращает к стандартным.
pub fn apply(chosen: &[(String, u64)]) {
    for limit in ALL {
        let found = chosen.iter().find(|c| c.0 == limit.name);
        limit.set(found.map_or(limit.default, |c| c.1));
    }
}

pub static ESCAPE_STRING_BYTES: Limit = Limit::new(
    "escape_string_bytes",
    [16777216, 64, 1099511627776],
    "больше — программа может занять всю память одной escape-строкой",
);

pub static CLIPBOARD_BYTES: Limit = Limit::new(
    "clipboard_bytes",
    [1048576, 0, 1099511627776],
    "больше — программа может положить в буфер обмена огромный текст",
);

pub static SHELL_TEXT_BYTES: Limit = Limit::new(
    "shell_text_bytes",
    [4096, 16, 1099511627776],
    "больше — длинные заголовки и пути от программ занимают память",
);

pub static HYPERLINKS: Limit = Limit::new(
    "hyperlinks",
    [4096, 1, 65535],
    "больше — растёт память под ссылки",
);

pub static HYPERLINK_BYTES: Limit = Limit::new(
    "hyperlink_bytes",
    [2048, 16, 1048576],
    "больше — программа может подсунуть огромный адрес",
);

pub static HYPERLINK_SWEEP_PAUSE: Limit = Limit::new(
    "hyperlink_sweep_pause",
    [256, 4, 1099511627776],
    "меньше — частая очистка ссылок тормозит вывод",
);

pub static IMAGE_MEMORY_BYTES: Limit = Limit::new(
    "image_memory_bytes",
    [268435456, 1048576, 1099511627776],
    "больше — картинки могут занять всю память",
);

pub static IMAGE_DECODE_BYTES: Limit = Limit::new(
    "image_decode_bytes",
    [268435456, 1048576, 1099511627776],
    "больше — декодирование картинок может занять всю память",
);

pub static IMAGE_ROWS: Limit = Limit::new(
    "image_rows",
    [500, 1, 100000],
    "больше — медленная прокрутка с картинками",
);

pub static IMAGES_PER_ROW: Limit = Limit::new(
    "images_per_row",
    [64, 1, 100000],
    "больше — медленная отрисовка строки",
);

pub static KITTY_PAYLOAD_BYTES: Limit = Limit::new(
    "kitty_payload_bytes",
    [67108864, 4096, 1099511627776],
    "больше — программа может прислать или прочитать огромный файл",
);

pub static KITTY_IMAGE_SIDE: Limit = Limit::new(
    "kitty_image_side",
    [8192, 1, 65535],
    "больше — огромная картинка займёт всю память",
);

pub static KEYBOARD_STACK: Limit = Limit::new(
    "keyboard_stack",
    [16, 1, 100000],
    "больше — программы копят режимы клавиатуры",
);

pub static PNG_SIDE: Limit = Limit::new(
    "png_side",
    [8192, 1, 65535],
    "больше — огромный PNG займёт всю память",
);

pub static SIXEL_SIDE: Limit = Limit::new(
    "sixel_side",
    [4096, 6, 65535],
    "больше — Sixel может занять всю память",
);

pub static SIXEL_PAINT_BUDGET: Limit = Limit::new(
    "sixel_paint_budget",
    [8000000, 1000, 1099511627776],
    "больше — Sixel может надолго подвесить окно",
);

pub static GIF_SIDE: Limit = Limit::new(
    "gif_side",
    [4096, 1, 65535],
    "больше — огромный GIF займёт всю память",
);

pub static GIF_BYTES: Limit = Limit::new(
    "gif_bytes",
    [268435456, 1048576, 1099511627776],
    "больше — анимация может занять всю память",
);

pub static DECODER_TIMEOUT: Limit = Limit::new(
    "decoder_timeout",
    [5, 1, 3600],
    "секунды; больше — зависший ffmpeg дольше держит окно",
);

pub static DECODER_OUTPUT_BYTES: Limit = Limit::new(
    "decoder_output_bytes",
    [134217728, 1048576, 1099511627776],
    "больше — вывод ffmpeg может занять всю память",
);

pub static REMOTE_REQUEST_BYTES: Limit = Limit::new(
    "remote_request_bytes",
    [1048576, 64, 1099511627776],
    "больше — клиент dimaster ctl может занять память",
);

pub static REMOTE_TIMEOUT: Limit = Limit::new(
    "remote_timeout",
    [3, 1, 3600],
    "секунды; больше — зависший клиент дольше держит соединение",
);

pub static REPLY_BACKLOG_BYTES: Limit = Limit::new(
    "reply_backlog_bytes",
    [1048576, 1024, 1099511627776],
    "больше — ответы программе, которая их не читает, копятся в памяти",
);

pub static ANIMATION_FRAME_MS: Limit = Limit::new(
    "animation_frame_ms",
    [16, 1, 1000],
    "меньше — выше нагрузка на процессор и видеокарту",
);

pub static SYNC_UPDATE_MS: Limit = Limit::new(
    "sync_update_ms",
    [150, 0, 60000],
    "больше — зависшая программа дольше замораживает экран",
);

pub static START_SETTLE_MS: Limit = Limit::new(
    "start_settle_ms",
    [60, 0, 10000],
    "меньше — shell может получить неверный размер окна",
);

pub static BELL_QUIET_MS: Limit = Limit::new(
    "bell_quiet_ms",
    [300, 0, 60000],
    "меньше — частый звонок плодит процессы",
);

pub static BELL_FRAME_PX: Limit = Limit::new("bell_frame_px", [4, 0, 200], "толщина рамки вспышки");

pub static PATH_CACHE_SECS: Limit = Limit::new(
    "path_cache_secs",
    [5, 0, 3600],
    "меньше — подсветка команд чаще читает диск",
);

pub static SENSOR_TIMEOUT: Limit = Limit::new(
    "sensor_timeout",
    [5, 1, 3600],
    "секунды; больше — зависший датчик дольше не обновляется",
);

pub static PLUGIN_HISTORY_LINES: Limit = Limit::new(
    "plugin_history_lines",
    [1000, 0, 1099511627776],
    "больше — плагины получают больше вывода и тормозят",
);

pub static PLUGIN_OVERRIDES: Limit = Limit::new(
    "plugin_overrides",
    [64, 1, 100000],
    "больше — настройки от плагинов копятся в памяти",
);

pub static PLUGIN_DEPTH: Limit = Limit::new(
    "plugin_depth",
    [4, 1, 64],
    "больше — плагины, вызывающие друг друга, могут подвесить окно",
);

pub static SSH_INCLUDE_DEPTH: Limit = Limit::new(
    "ssh_include_depth",
    [3, 0, 64],
    "больше — зацикленные Include в ssh_config читаются дольше",
);

pub static PLUGIN_EFFECTS: Limit = Limit::new(
    "plugin_effects",
    [256, 1, 1099511627776],
    "больше — плагин может завалить окно действиями",
);

pub static PLUGIN_BUDGET_MS: Limit = Limit::new(
    "plugin_budget_ms",
    [100, 1, 600000],
    "больше — медленный плагин подвешивает окно",
);

pub static PLUGIN_SHAPES: Limit = Limit::new(
    "plugin_shapes",
    [2000, 0, 1000000],
    "больше — рисунок плагина тормозит отрисовку",
);

pub static PLUGIN_POINTS: Limit = Limit::new(
    "plugin_points",
    [64, 3, 100000],
    "больше — сложные фигуры тормозят отрисовку",
);

pub static BAR_ROWS: Limit = Limit::new(
    "bar_rows",
    [5, 1, 100],
    "больше — строка статуса закрывает терминал",
);

pub static BAR_SEGMENTS: Limit = Limit::new(
    "bar_segments",
    [32, 1, 10000],
    "больше — строка статуса тормозит отрисовку",
);

pub static BAR_PIXELS: Limit = Limit::new(
    "bar_pixels",
    [200, 0, 10000],
    "больше — отступы бара могут занять всё окно",
);

pub static IMPORT_DEPTH: Limit = Limit::new(
    "import_depth",
    [4, 0, 64],
    "больше — зацикленный импорт читается дольше",
);

pub static FETCH_PICTURE_CELLS: Limit = Limit::new(
    "fetch_picture_cells",
    [200, 1, 2000],
    "больше — картинка fetch заполняет весь экран",
);

pub static PHOTO_SAMPLES: Limit = Limit::new(
    "photo_samples",
    [20000, 100, 1099511627776],
    "больше — дольше подбираются цвета по картинке",
);

pub static SHAPE_CACHE: Limit = Limit::new(
    "shape_cache",
    [8192, 16, 1099511627776],
    "меньше — медленнее отрисовка; больше — растёт память",
);

pub static CURVE_STEPS: Limit = Limit::new(
    "curve_steps",
    [8, 1, 64],
    "меньше — угловатые буквы; больше — медленнее",
);

pub static DOCKER_LOG_LINES: Limit = Limit::new(
    "docker_log_lines",
    [10000, 10, 1099511627776],
    "больше — журналы занимают память",
);

pub static DOCKER_LOG_LINE_BYTES: Limit = Limit::new(
    "docker_log_line_bytes",
    [16384, 128, 1099511627776],
    "больше — одна строка журнала занимает память",
);

pub static DOCKER_LOG_QUEUE: Limit = Limit::new(
    "docker_log_queue",
    [1024, 1, 1099511627776],
    "меньше — журналы отстают",
);

pub static DOCKER_OUTPUT_BYTES: Limit = Limit::new(
    "docker_output_bytes",
    [16777216, 1024, 1099511627776],
    "больше — вывод docker может занять всю память",
);

pub static VIEW_COMMAND_BYTES: Limit = Limit::new(
    "view_command_bytes",
    [65536, 64, 1099511627776],
    "больше — клиент view --send может занять память",
);

pub static VIEW_SAMPLE_ROWS: Limit = Limit::new(
    "view_sample_rows",
    [1000, 1, 1099511627776],
    "больше — дольше открываются таблицы",
);

pub static TABLE_QUERY_SECS: Limit = Limit::new(
    "table_query_secs",
    [60, 1, 1099511627776],
    "меньше — долгие запросы view обрываются; больше — зависший держит программу",
);

pub static TABLE_RESULT_BYTES: Limit = Limit::new(
    "table_result_bytes",
    [1073741824, 1024, 1099511627776],
    "больше — результат запроса может занять всю память",
);

pub static REGEX_BYTES: Limit = Limit::new(
    "regex_bytes",
    [1048576, 1024, 1099511627776],
    "больше — сложный поиск может занять всю память",
);

pub static SIXEL_PIXELS: Limit = Limit::new(
    "sixel_pixels",
    [4000000, 64, 1099511627776],
    "больше — Sixel может занять всю память",
);

pub static DECODER_POLL_MS: Limit = Limit::new(
    "decoder_poll_ms",
    [5, 1, 1000],
    "меньше — выше нагрузка на процессор",
);

pub static VIEW_CONNECT_SECS: Limit = Limit::new(
    "view_connect_secs",
    [2, 1, 3600],
    "секунды ожидания окна view --remote",
);

pub static VIEW_REPLY_SECS: Limit = Limit::new(
    "view_reply_secs",
    [60, 1, 3600],
    "секунды ожидания ответа view --send",
);

pub static TUI_POLL_MS: Limit = Limit::new(
    "tui_poll_ms",
    [100, 5, 5000],
    "меньше — выше нагрузка; больше — экраны view и docker отстают",
);

pub static TUNNEL_POLL_MS: Limit = Limit::new(
    "tunnel_poll_ms",
    [50, 1, 5000],
    "меньше — выше нагрузка на процессор",
);

pub static ANIMATION_STEP_MS: Limit = Limit::new(
    "animation_step_ms",
    [50, 1, 1000],
    "больше — после паузы курсор прыгает",
);

pub static PLUGIN_LABEL_CHARS: Limit = Limit::new(
    "plugin_label_chars",
    [500, 1, 1099511627776],
    "больше — длинные подписи плагинов тормозят отрисовку",
);

pub static MIN_WAKE_MS: Limit = Limit::new(
    "min_wake_ms",
    [16, 1, 1000],
    "меньше — выше нагрузка на процессор",
);

pub static REDRAW_WAIT_MS: Limit = Limit::new(
    "redraw_wait_ms",
    [40, 1, 5000],
    "больше — анимации картинок идут рывками",
);

pub static SCROLLBACK_MAX: Limit = Limit::new(
    "scrollback_max",
    [1000000, 0, 1099511627776],
    "больше — история может занять всю память",
);

pub static SESSION_LINES_MAX: Limit = Limit::new(
    "session_lines_max",
    [100000, 0, 1099511627776],
    "больше — сохранённая сессия занимает диск",
);

pub static PANEL_RADIUS_MAX: Limit = Limit::new(
    "panel_radius_max",
    [200, 0, 100000],
    "наибольший радиус углов панелей",
);

pub static PANEL_PADDING_MAX: Limit = Limit::new(
    "panel_padding_max",
    [100, 0, 100000],
    "наибольший внутренний отступ панелей и заголовка",
);

pub static PALETTE_COLUMNS_MAX: Limit = Limit::new(
    "palette_columns_max",
    [500, 10, 100000],
    "наибольшая ширина палитры в ячейках",
);

pub static LIST_ROWS_MAX: Limit = Limit::new(
    "list_rows_max",
    [100, 1, 100000],
    "наибольшее число строк в списках",
);

pub static STRIP_WIDTH_MAX: Limit = Limit::new(
    "strip_width_max",
    [400, 1, 100000],
    "наибольшая ширина полосы прокрутки и миникарты",
);

pub static NOTICE_MIN_MS: Limit =
    Limit::new("notice_min_ms", [200, 1, 60000], "наименьшее время плашки");

pub static NOTICE_MAX_MS: Limit = Limit::new(
    "notice_max_ms",
    [60000, 60000, 1099511627776],
    "наибольшее время плашки",
);

pub static NOTICE_MARGIN_MAX: Limit = Limit::new(
    "notice_margin_max",
    [2000, 0, 100000],
    "наибольший отступ плашки",
);

pub static WINDOW_CELLS_MAX: Limit = Limit::new(
    "window_cells_max",
    [1000, 10, 100000],
    "наибольший начальный размер окна в ячейках",
);

pub static FETCH_LOGO_WIDTH_MAX: Limit = Limit::new(
    "fetch_logo_width_max",
    [200, 0, 100000],
    "наибольшая ширина логотипа fetch",
);

pub static SCROLLBAR_THUMB_PX: Limit = Limit::new(
    "scrollbar_thumb_px",
    [12, 1, 100000],
    "наименьшая высота ползунка",
);

pub static SCROLLBAR_MARK_PX: Limit = Limit::new(
    "scrollbar_mark_px",
    [2, 1, 100000],
    "высота метки на полосе прокрутки",
);

pub static MINIMAP_ROW_PX: Limit = Limit::new(
    "minimap_row_px",
    [2, 1, 100],
    "высота одной полоски миникарты",
);

pub static MINIMAP_VIEW_PX: Limit = Limit::new(
    "minimap_view_px",
    [4, 1, 100000],
    "наименьшая высота рамки миникарты и поля слева",
);

pub static PROGRESS_PX: Limit =
    Limit::new("progress_px", [3, 0, 100000], "толщина полоски выполнения");

pub static GLOW_PX: Limit = Limit::new(
    "glow_px",
    [4, 1, 100000],
    "средняя толщина пульсирующей рамки",
);

pub static GLOW_WAVE_PX: Limit =
    Limit::new("glow_wave_px", [3, 0, 100000], "размах пульсации рамки");

pub static HISTORY_COLUMNS_MIN: Limit = Limit::new(
    "history_columns_min",
    [30, 1, 100000],
    "наименьшая ширина панели истории в ячейках",
);

pub static MONITOR_GAP_PX: Limit = Limit::new(
    "monitor_gap_px",
    [20, 0, 100000],
    "зазор между монитором и миникартой",
);

pub static MONITOR_MARGIN_PX: Limit = Limit::new(
    "monitor_margin_px",
    [4, 0, 100000],
    "отступ монитора от края",
);

pub static SKINS_COLUMNS: Limit = Limit::new(
    "skins_columns",
    [46, 16, 1000],
    "ширина витрины скинов в ячейках",
);

pub static SKINS_ROWS: Limit = Limit::new("skins_rows", [14, 1, 1000], "строк в витрине скинов");

pub static PRESENTATION_SIDE_DIVISOR: Limit = Limit::new(
    "presentation_side_divisor",
    [10, 2, 100000],
    "поля презентации: ширина окна делится на это число",
);

pub static PRESENTATION_TOP_PX: Limit = Limit::new(
    "presentation_top_px",
    [24, 0, 100000],
    "наименьшее поле презентации сверху",
);

pub static PHOTO_SHADE_MIN: Limit = Limit::new(
    "photo_shade_min",
    [20, 0, 100],
    "наименьшее затемнение фото, проценты",
);

pub static PHOTO_SHADE_MAX: Limit = Limit::new(
    "photo_shade_max",
    [60, 0, 100],
    "наибольшее затемнение фото, проценты",
);

pub static SCENE_WIDTH: Limit = Limit::new(
    "scene_width",
    [960, 16, 4096],
    "ширина рисованного фона в пикселях",
);

pub static SCENE_HEIGHT: Limit = Limit::new(
    "scene_height",
    [600, 16, 4096],
    "высота рисованного фона в пикселях",
);

// проценты как доля; границы переставляются, если заданы наоборот
fn share(low: &Limit, high: &Limit) -> (f32, f32) {
    let (low, high) = (low.get() as f32 / 100.0, high.get() as f32 / 100.0);
    (low.min(high), high.max(low))
}

/// Разрешённые прозрачности.
pub fn opacity_range() -> (f32, f32) {
    share(&OPACITY_MIN_PERCENT, &OPACITY_MAX_PERCENT)
}

/// Разрешённая ширина панели истории в долях окна.
pub fn history_width_range() -> (f32, f32) {
    share(&HISTORY_WIDTH_MIN, &HISTORY_WIDTH_MAX)
}

pub static UNDERLINE_WAVE_ROOM: Limit = Limit::new(
    "underline_wave_room",
    [3, 0, 100],
    "место под волной: толщин линии от низа ячейки",
);

pub static UNDERLINE_DOUBLE_GAP: Limit = Limit::new(
    "underline_double_gap",
    [2, 1, 100],
    "шаг двойного подчёркивания в толщинах линии",
);

pub static UNDERLINE_DOT_STEP: Limit = Limit::new(
    "underline_dot_step",
    [2, 1, 100],
    "шаг точек подчёркивания в толщинах линии",
);

pub static UNDERLINE_DASH_PERIOD: Limit = Limit::new(
    "underline_dash_period",
    [50, 1, 1000],
    "шаг штрихов, проценты ширины ячейки",
);

pub static UNDERLINE_DASH_LENGTH: Limit = Limit::new(
    "underline_dash_length",
    [33, 1, 1000],
    "длина штриха, проценты ширины ячейки",
);

pub static STRIKE_POSITION: Limit = Limit::new(
    "strike_position",
    [67, 0, 100],
    "высота зачёркивания, проценты от базовой линии",
);

pub static HOLLOW_CURSOR_LINES: Limit = Limit::new(
    "hollow_cursor_lines",
    [2, 1, 100],
    "толщина контура курсора в толщинах линии",
);

pub static SMOOTH_SCROLL_ROWS: Limit = Limit::new(
    "smooth_scroll_rows",
    [3, 0, 1000],
    "наибольшая плавная прокрутка в строках",
);

pub static LIGHT_BACKGROUND_PERCENT: Limit = Limit::new(
    "light_background_percent",
    [50, 0, 100],
    "яркость фона, ниже которой текст для контраста светлеет",
);

pub static SKINS_SWATCH_TOP: Limit = Limit::new(
    "skins_swatch_top",
    [6, 1, 100],
    "отступ образца цвета: строка делится на это число",
);

pub static SKINS_SWATCH_CUT: Limit = Limit::new(
    "skins_swatch_cut",
    [3, 2, 100],
    "насколько образец ниже строки: строка делится на это число",
);

pub static SKINS_SWATCH_CELLS: Limit = Limit::new(
    "skins_swatch_cells",
    [7, 0, 100],
    "ширина подложки образцов в ячейках",
);

pub static SKINS_BACK_PERCENT: Limit = Limit::new(
    "skins_back_percent",
    [40, 0, 100],
    "левая доля строки стиля, щелчок по которой листает назад",
);

pub static OPACITY_MIN_PERCENT: Limit = Limit::new(
    "opacity_min_percent",
    [0, 0, 100],
    "наименьшая разрешённая прозрачность, проценты",
);

pub static OPACITY_MAX_PERCENT: Limit = Limit::new(
    "opacity_max_percent",
    [100, 0, 1000],
    "наибольшая разрешённая прозрачность, проценты; выше 100 на экране не отличается",
);

pub static HISTORY_WIDTH_MIN: Limit = Limit::new(
    "history_width_min",
    [20, 0, 100],
    "наименьшая ширина панели истории, проценты окна",
);

pub static HISTORY_WIDTH_MAX: Limit = Limit::new(
    "history_width_max",
    [100, 1, 1000],
    "наибольшая ширина панели истории, проценты окна",
);

/// Все пределы в порядке записи в конфиг.
#[rustfmt::skip]
pub static ALL: &[&Limit] = &[
    &ESCAPE_STRING_BYTES,
    &CLIPBOARD_BYTES,
    &SHELL_TEXT_BYTES,
    &HYPERLINKS,
    &HYPERLINK_BYTES,
    &HYPERLINK_SWEEP_PAUSE,
    &IMAGE_MEMORY_BYTES,
    &IMAGE_DECODE_BYTES,
    &IMAGE_ROWS,
    &IMAGES_PER_ROW,
    &KITTY_PAYLOAD_BYTES,
    &KITTY_IMAGE_SIDE,
    &KEYBOARD_STACK,
    &PNG_SIDE,
    &SIXEL_SIDE,
    &SIXEL_PAINT_BUDGET,
    &GIF_SIDE,
    &GIF_BYTES,
    &DECODER_TIMEOUT,
    &DECODER_OUTPUT_BYTES,
    &REMOTE_REQUEST_BYTES,
    &REMOTE_TIMEOUT,
    &REPLY_BACKLOG_BYTES,
    &ANIMATION_FRAME_MS,
    &SYNC_UPDATE_MS,
    &START_SETTLE_MS,
    &BELL_QUIET_MS,
    &BELL_FRAME_PX,
    &PATH_CACHE_SECS,
    &SENSOR_TIMEOUT,
    &PLUGIN_HISTORY_LINES,
    &PLUGIN_OVERRIDES,
    &PLUGIN_DEPTH,
    &SSH_INCLUDE_DEPTH,
    &PLUGIN_EFFECTS,
    &PLUGIN_BUDGET_MS,
    &PLUGIN_SHAPES,
    &PLUGIN_POINTS,
    &BAR_ROWS,
    &BAR_SEGMENTS,
    &BAR_PIXELS,
    &IMPORT_DEPTH,
    &FETCH_PICTURE_CELLS,
    &PHOTO_SAMPLES,
    &SHAPE_CACHE,
    &CURVE_STEPS,
    &DOCKER_LOG_LINES,
    &DOCKER_LOG_LINE_BYTES,
    &DOCKER_LOG_QUEUE,
    &DOCKER_OUTPUT_BYTES,
    &VIEW_COMMAND_BYTES,
    &VIEW_SAMPLE_ROWS,
    &TABLE_QUERY_SECS,
    &TABLE_RESULT_BYTES,
    &REGEX_BYTES,
    &SIXEL_PIXELS,
    &DECODER_POLL_MS,
    &VIEW_CONNECT_SECS,
    &VIEW_REPLY_SECS,
    &TUI_POLL_MS,
    &TUNNEL_POLL_MS,
    &ANIMATION_STEP_MS,
    &PLUGIN_LABEL_CHARS,
    &MIN_WAKE_MS,
    &REDRAW_WAIT_MS,
    &SCROLLBACK_MAX,
    &SESSION_LINES_MAX,
    &PANEL_RADIUS_MAX,
    &PANEL_PADDING_MAX,
    &PALETTE_COLUMNS_MAX,
    &LIST_ROWS_MAX,
    &STRIP_WIDTH_MAX,
    &NOTICE_MIN_MS,
    &NOTICE_MAX_MS,
    &NOTICE_MARGIN_MAX,
    &WINDOW_CELLS_MAX,
    &FETCH_LOGO_WIDTH_MAX,
    &SCROLLBAR_THUMB_PX,
    &SCROLLBAR_MARK_PX,
    &MINIMAP_ROW_PX,
    &MINIMAP_VIEW_PX,
    &PROGRESS_PX,
    &GLOW_PX,
    &GLOW_WAVE_PX,
    &HISTORY_COLUMNS_MIN,
    &MONITOR_GAP_PX,
    &MONITOR_MARGIN_PX,
    &SKINS_COLUMNS,
    &SKINS_ROWS,
    &PRESENTATION_SIDE_DIVISOR,
    &PRESENTATION_TOP_PX,
    &PHOTO_SHADE_MIN,
    &PHOTO_SHADE_MAX,
    &SCENE_WIDTH,
    &SCENE_HEIGHT,
    &UNDERLINE_WAVE_ROOM,
    &UNDERLINE_DOUBLE_GAP,
    &UNDERLINE_DOT_STEP,
    &UNDERLINE_DASH_PERIOD,
    &UNDERLINE_DASH_LENGTH,
    &STRIKE_POSITION,
    &HOLLOW_CURSOR_LINES,
    &SMOOTH_SCROLL_ROWS,
    &LIGHT_BACKGROUND_PERCENT,
    &SKINS_SWATCH_TOP,
    &SKINS_SWATCH_CUT,
    &SKINS_SWATCH_CELLS,
    &SKINS_BACK_PERCENT,
    &OPACITY_MIN_PERCENT,
    &OPACITY_MAX_PERCENT,
    &HISTORY_WIDTH_MIN,
    &HISTORY_WIDTH_MAX,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_clamped_and_reset() {
        apply(&[("plugin_depth".into(), 1000)]);
        assert_eq!(PLUGIN_DEPTH.get(), 64);
        apply(&[]);
        assert_eq!(PLUGIN_DEPTH.get(), PLUGIN_DEPTH.default);
        assert!(ALL.iter().all(|l| (l.min..=l.max).contains(&l.default)));
    }
}
