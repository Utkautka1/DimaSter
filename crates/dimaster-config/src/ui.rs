//! Вид всего, что рисуется поверх терминала: таблица `ui` из конфига.
//!
//! `None` у цвета означает «вывести из темы»: так вид не ломается при смене
//! темы, пока пользователь не задал цвет явно.

use crate::Action;

/// Подложка всплывающих панелей: палитры, истории, поиска, монитора.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelStyle {
    pub background: Option<u32>,
    pub color: Option<u32>,
    pub selected_background: Option<u32>,
    pub selected_color: Option<u32>,
    // радиус углов и поля вокруг текста в пикселях
    pub radius: u32,
    pub padding: u32,
    pub opacity: f32,
}

impl Default for PanelStyle {
    fn default() -> Self {
        Self {
            background: None,
            color: None,
            selected_background: None,
            selected_color: None,
            radius: 0,
            padding: 0,
            opacity: 1.0,
        }
    }
}

/// Горизонтальное положение.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Вертикальное положение.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowButton {
    Minimize,
    Maximize,
    Close,
}

/// Кнопка своей полосы заголовка.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleButton {
    pub kind: WindowButton,
    pub text: String,
    pub color: Option<u32>,
}

/// Своя полоса заголовка.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleStyle {
    // пиксели сверх высоты строки
    pub padding: u32,
    // `{title}` заменяется на заголовок от программы
    pub format: String,
    pub align: Align,
    pub background: Option<u32>,
    pub color: Option<u32>,
    pub buttons: Vec<TitleButton>,
    pub buttons_left: bool,
}

fn button(kind: WindowButton, text: &str) -> TitleButton {
    TitleButton {
        kind,
        text: text.to_owned(),
        color: None,
    }
}

impl Default for TitleStyle {
    fn default() -> Self {
        Self {
            padding: 8,
            format: "{title}".to_owned(),
            align: Align::Left,
            background: None,
            color: None,
            buttons: vec![
                button(WindowButton::Minimize, " — "),
                button(WindowButton::Maximize, " □ "),
                button(WindowButton::Close, " ✕ "),
            ],
            buttons_left: false,
        }
    }
}

/// Палитра команд.
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteStyle {
    // ширина в ячейках
    pub width: usize,
    pub rows: usize,
    pub position: Anchor,
    pub prompt: String,
    /// Свой список команд вместо встроенного; пустой — встроенный.
    pub items: Vec<(String, Action)>,
}

impl Default for PaletteStyle {
    fn default() -> Self {
        Self {
            width: 50,
            rows: 11,
            position: Anchor::Top,
            prompt: " > ".to_owned(),
            items: Vec::new(),
        }
    }
}

/// Панель истории команд.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistoryStyle {
    // доля ширины окна
    pub width: f32,
    pub rows: usize,
    pub left: bool,
}

impl Default for HistoryStyle {
    fn default() -> Self {
        Self {
            width: 0.6,
            rows: 10,
            left: false,
        }
    }
}

/// Цвета меток на ссылках и подсветок в тексте.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Marks {
    pub hint_color: u32,
    pub hint_background: u32,
    pub found: u32,
    pub current: u32,
    pub link: u32,
}

impl Default for Marks {
    fn default() -> Self {
        Self {
            hint_color: 0x11131a,
            hint_background: 0xe0af68,
            found: 0x6b5a1e,
            current: 0xb0620f,
            link: 0x2f4a7a,
        }
    }
}

/// Полоса прокрутки и миникарта.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strip {
    // ширина в пикселях
    pub width: u32,
    // ползунок полосы или текст миникарты
    pub color: u32,
    pub command: u32,
    pub error: u32,
    pub found: u32,
    pub left: bool,
}

impl Strip {
    fn sized(width: u32) -> Self {
        Self {
            width,
            color: 0x9aa5ce,
            command: 0x7aa2f7,
            error: 0xf7768e,
            found: 0xe0af68,
            left: false,
        }
    }
}

/// Монитор ресурсов в углу.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorStyle {
    pub left: bool,
    pub bottom: bool,
    // `{process}`, `{cpu}`, `{memory}`
    pub format: String,
    pub waiting: String,
}

impl Default for MonitorStyle {
    fn default() -> Self {
        Self {
            left: false,
            bottom: false,
            format: " {process}  CPU {cpu}%  {memory} ".to_owned(),
            waiting: " ожидание ".to_owned(),
        }
    }
}

/// Тексты режимов и панелей.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texts {
    pub vi: String,
    pub hint: String,
    pub minimap: String,
    // `{query}`, `{position}`, `{count}`, `{kind}`
    pub search: String,
    pub search_plain: String,
    pub search_regex: String,
    pub history: String,
    pub history_output: String,
    // `{code}`, `{reason}`
    pub history_failed: String,
}

impl Default for Texts {
    fn default() -> Self {
        Self {
            vi: " VI: hjkl w b 0 $ g G · v V · y копировать · / поиск · q выход ".into(),
            hint: " Метка — открыть, ЗАГЛАВНАЯ — копировать, Esc ".into(),
            minimap: " Карта: j/k прокрутка · g/G края · Enter здесь · Esc назад ".into(),
            search: " Поиск: {query}_  {position}/{count}  [{kind}, Ctrl+R] ".into(),
            search_plain: "текст".into(),
            search_regex: "regex".into(),
            history: " j/k выбор · Enter вставить · y вывод · q".into(),
            history_output: "вывод".into(),
            history_failed: "код {code}: {reason}".into(),
        }
    }
}

/// Плашка «скопировано».
#[derive(Debug, Clone, PartialEq)]
pub struct NoticeStyle {
    pub copy: bool,
    /// Показывать и при выделении мышью, когда текст идёт только в primary.
    pub primary: bool,
    // `{count}`, `{lines}`, `{preview}`, `{target}`; `\n` — новая строка
    pub text: String,
    // миллисекунды на экране
    pub duration: u32,
    pub side: Align,
    pub position: Anchor,
    // отступ от краёв окна в пикселях
    pub margin: (u32, u32),
    /// Свои цвета и форма; `None` — как у остальных панелей.
    pub look: Option<PanelStyle>,
}

impl Default for NoticeStyle {
    fn default() -> Self {
        Self {
            copy: true,
            primary: false,
            text: " ✓ Скопировано: {count} ".to_owned(),
            duration: 1200,
            side: Align::Right,
            position: Anchor::Bottom,
            margin: (10, 4),
            look: None,
        }
    }
}

/// Вся таблица `ui`.
#[derive(Debug, Clone, PartialEq)]
pub struct Ui {
    pub panel: PanelStyle,
    pub title: TitleStyle,
    pub palette: PaletteStyle,
    pub history: HistoryStyle,
    pub marks: Marks,
    pub scrollbar: Strip,
    pub minimap: Strip,
    pub monitor: MonitorStyle,
    pub notice: NoticeStyle,
    /// Где строка поиска и подсказки режимов: `true` — сверху.
    pub prompt_top: bool,
    pub texts: Texts,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            panel: PanelStyle::default(),
            title: TitleStyle::default(),
            palette: PaletteStyle::default(),
            history: HistoryStyle::default(),
            marks: Marks::default(),
            scrollbar: Strip::sized(5),
            minimap: Strip::sized(90),
            monitor: MonitorStyle::default(),
            notice: NoticeStyle::default(),
            prompt_top: false,
            texts: Texts::default(),
        }
    }
}
