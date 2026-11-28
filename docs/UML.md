# DimaSter: диаграммы

## Зависимости крейтов

```mermaid
graph TD
    app[dimaster] --> grid[dimaster-grid]
    app --> render[dimaster-render]
    app --> font[dimaster-font]
    render --> grid
    render --> font
    font --> image[dimaster-image]
    app --> config[dimaster-config]
    grid --> image
    app --> image
    image -.-> ffmpeg
    app --> i18n[dimaster-i18n]
    config --> i18n
    grid --> i18n
    font --> i18n
    render --> i18n
    app --> pty[dimaster-pty]
    app --> view[dimaster-view]
    view --> data[dimaster-data]
    view --> config
    view --> i18n
    view --> image
    app --> data
    grid --> vt[dimaster-vt]
```

## Классы

```mermaid
classDiagram
    class Parser { +advance(perform, bytes) usize }
    class Perform {
        +paused() bool
        <<trait>>
        print(ch)
        execute(byte)
        esc(seq)
        csi(seq)
        osc(data)
        dcs(seq, data)
        apc(data)
    }
    class Sequence { params; intermediates; action }
    class Params { +groups() +get_or(index, default) }
    class Terminal {
        +advance_part(bytes) SlowImage
        +finish_image(decoded)
        +advance(bytes)
        +resize(size)
        +take_reply()
        +text()
    }
    class State { screen; other; cursor; modes; tabs; title; cwd }
    class Screen { +rows() +history() }
    class Row { cells; wrapped; mark }
    class Cell { ch; extra; style }
    class Style { fg; bg; flags }
    class Cursor { row; col; style }
    class Modes { mouse; synchronized; cursor_shape }
    class KeyboardStack { +current(alternate) }
    class Selection { kind; anchor; head }
    class Span { +contains(line, col) }
    class Selecting { selection; clicks }
    class Clipboard { +set(target, text) +get(target) }
    class Viewport { scroll; selection }
    View *-- Selecting
    View *-- Clipboard
    Selecting o-- Selection
    Terminal ..> Span : span(selection)
    Renderer ..> Viewport
    Viewport o-- Span
    class DefaultColors { foreground; background }
    class Pointer { cell; held }
    State *-- KeyboardStack
    State *-- DefaultColors
    View *-- Pointer
    class Pty { +spawn(command, size) +resize(size) +wait() }
    class App { +resumed() +window_event() +about_to_wait() }
    class View { +start_when_settled() +read_output() +draw() +resize() +key() }
    class Session { +spawn(launch, size, output) +send(bytes) +resize(size) }
    class Held { +acquire() +release() +share() }
    class Signals { +raise() +close() +take() }
    View *-- Held
    Held o-- Terminal
    Session ..> Signals
    class Renderer { +new() +resize() +draw(term, fonts, scroll) }
    class Builder { +build(term, scroll) }
    class Atlas { +get(fonts, key) +clear() }
    class Pipeline { +upload(queue, upload) }
    class FontSystem { +new(size) +metrics() +set_size(size) +set_features(list) +shape(text, bold, italic) +glyph(id, cells) }
    class ColrPainter { +outline_glyph() +paint() +push_clip() +push_layer() }
    class ShapedGlyph { id; cluster; x; y }
    class Run { text; cells; color }
    class Image { width; height; pixels }
    class Animation { width; height; frames; +frame_at(elapsed) +frame_at_limit(elapsed, loops) }
    class Store { +picture(animation) +keep(id, picture) +put_virtual() +get_virtual() +trim() }
    class SlowImage { +decode() DecodedImage }
    State o-- SlowImage : deferred
    class Picture { serial; animation; started; generation; playback }
    class Placement { col; cols; rows; picture; crop }
    class VirtualPlacement { picture; cols; rows }
    class PackedRow { wrapped; mark; images; text; runs; extras }
    class Textures { +prepare(images) +get(image) }
    State *-- Store
    Store *-- VirtualPlacement
    Screen *-- PackedRow
    Row *-- Placement
    Placement --> Picture
    VirtualPlacement --> Picture
    Picture *-- Animation
    Renderer *-- Textures
    FontSystem ..> ColrPainter : COLR v0/v1
    class Config { font; theme; padding; cursor; scrollback; shell; keys }
    class Theme { foreground; background; cursor; selection; ansi }
    class Source { +apply(config_home, config) }
    class Appearance { colors; padding; rtl }
    Config *-- Theme
    Config *-- Binding
    Source ..> Config
    View *-- Config
    class AssistStore { commands; dirs; +record_command() +bump_dir() +best_suggestion() }
    class Picker { kind; items; query; +matches() }
    View *-- AssistStore
    class Mode { Normal; Search; Vi; Palette; Hint; History; Picker; Minimap; Paste; CloseConfirm }
    class PasteConfirm { text; reason }
    class Animator { +tick(target, effects) +next_frame() }
    class WindowConfig { decorations; opacity; blur; background_image }
    class Effects { cursor_blink; cursor_animation; cursor_trail; smooth_scroll }
    class Label { x; y; text; color }
    View *-- Mode
    View *-- Animator
    Config *-- WindowConfig
    Config *-- Effects
    View ..> Label
    class Plugins { +load(directory) +emit(event) +run(command) +commands() }
    class Server { path }
    class Request { args; reply }
    class Saved { cwd; lines }
    class Launch { args; term_name; directory; socket }
    View *-- Plugins
    class Commands { options; input_start; running; history }
    class CommandCache { bins; +highlight_input(term, palette) }
    View *-- CommandCache
    class Links { uris; free; +uri(number) +sweep(used) }
    State *-- Links
    class Finished { command; exit; duration; output }
    class SmartOptions { ok; empty; errors; search_tools }
    class Assist { glow_since; typed }
    class Rect { x; y; width; height; color }
    class Hints { targets; typed }
    class History { entries; selected }
    State *-- Commands
    Commands *-- Finished
    Commands *-- SmartOptions
    class Help { enabled; duration; welcome; typo; denied; skip; managers }
    View ..> Help : help_with(Finished)
    class Tuning { timings; limits; hint_letters; pairs; suggest_keys; sizes }
    class DockerTiming { query; action; ssh_persist; tunnel }
    View ..> Tuning
    class Native { history_search; complete; abbreviations; aliases; flag_sources; flags }
    class Walk { prefix; index; shown; found }
    View ..> Native
    Assist *-- Walk
    class Limit { name; default; min; max; risk; +get() +set(value) }
    View ..> Limit : apply(Config.limits)
    View *-- Assist
    class Status { target; facts; line }
    class Sampler { versions; previous; ticks; +sample(target) }
    class Ticks { total; busy }
    Sampler *-- Ticks : system, /proc
    class StatusLine { left; center; right }
    class Segment { text; color; background; rounded }
    class Minimap { origin }
    View *-- Status
    class StatusBar { style; rows }
    class Ui { panel; title; palette; history; marks; scrollbar; minimap; monitor; texts }
    class CursorLook { thickness; width; height; radius; opacity; speed; linear; trail }
    Config *-- Ui
    Config *-- CursorLook
    class Drawn { rects; labels }
    class Style { id; title; literal; scene; shade }
    class Look { style; plain; picture }
    class Choice { style; photo; +layers(skin) }
    class Photo { name; base; +loaded() }
    class Loaded { theme; picture; shade }
    View *-- Look
    Look ..> Choice
    Choice --> Style
    Choice --> Photo : own background
    Photo *-- Loaded
    Style ..> Scene : background
    class Skin { name; theme }
    class Skins { names; query; selected; saved; +press() +click(row) }
    View ..> Skins : gallery
    Skins ..> Skin
    class Picker { kind; items; values; query; +press() }
    class Snippet { name; command }
    Config *-- Snippet
    View ..> Picker : SSH, Docker, snippets
    class Hooks { keys; timers; overrides }
    View *-- Hooks
    Plugins ..> Hooks : bind, every, set
    class Notice { shown; +expires(now) }
    View *-- Notice
    class Layout { lines; logo; image; random; picture; logo_width }
    class PictureStyle { width; height; fit; position; gap; border }
    Layout *-- PictureStyle
    class Shape { <<enum>> Rect; Line; Polygon; Text }
    Plugins ..> Shape : draw.lua
    class Binding { chords; actions; unconsumed; performable; table }
    class KeyState { pending; tables; +find(bindings, pressed) }
    View *-- KeyState
    class Policy { title; colors; cursor_shape; pointer_shape; progress }
    class Programs
    class Bell { flash; attention; notify; command }
    Config *-- Programs
    Config *-- Bell
    Programs ..> Policy
    View ..> Drawn
    class BarStyle { top; margin; padding; background; radius; opacity }
    StatusBar *-- BarStyle
    Status *-- StatusBar
    Status ..> Sampler : отдельный поток
    Status *-- StatusLine
    StatusLine *-- Segment
    Plugins ..> StatusLine
    View ..> Rect
    class Layout { lines; logo; image; logo_width }
    class ColorOverrides { palette; foreground; background; cursor }
    State *-- ColorOverrides
    App *-- Server
    Server ..> Request
    Request ..> View
    View ..> Saved
    Session ..> Launch
    Terminal ..> Match : search / link_at
    Renderer *-- Appearance
    FontSystem ..> ShapedGlyph
    FontSystem ..> Image : decode_png
    Builder --> Run
    class Outline { +rasterize() Bitmap }

    App *-- View
    View *-- Terminal
    View *-- Session
    View *-- Renderer
    View *-- FontSystem
    Session *-- Pty
    Renderer *-- Atlas
    Renderer *-- Pipeline
    Renderer ..> Builder
    Builder --> Atlas
    Builder --> FontSystem
    FontSystem ..> Outline

    Terminal *-- Parser
    Terminal *-- State
    Parser ..> Perform
    Perform ..> Sequence
    Sequence --> Params
    State ..|> Perform
    State *-- "2" Screen
    State *-- Cursor
    State *-- Modes
    Screen *-- Row
    Row *-- Cell
    Cell *-- Style
```

## Просмотр таблиц

```mermaid
classDiagram
    class Database { +load(path) +query(sql) +table(name) +set_limits(limits) +open_table(name) +open(sql) +fetch(rows, indexes) +scan(rows, visit) +close(rows) }
    class Rows { columns; len; source }
    Database ..> Rows
    class Limits { time; bytes }
    class Watchdog { +start(connection, time) }
    Database *-- Limits
    Database ..> Watchdog : query
    class Table { name; columns; rows }
    class App { sheets; active; mode; +press(key) }
    class Sheet { sql; data; sample; columns; found; +search() +apply() +page() }
    class Viewer { settings; +columns() +cell() +status() }
    class Plot { Histogram; Scatter }
    class Server { requests }
    App --> Plot
    Server ..> App : dimaster view --send
    App *-- Database
    App *-- Sheet
    App --> Viewer : view.lua
    Sheet --> Rows
    Database ..> Table
```

## Поток данных

```mermaid
sequenceDiagram
    participant W as окно (winit)
    participant V as View
    participant S as Session
    participant T as Terminal
    participant R as Renderer
    W->>V: клавиша
    V->>S: байты в shell
    S-->>V: Wake и вывод
    V->>T: advance(bytes)
    T-->>S: take_reply()
    V->>R: draw(term)
```

## Слои настроек

```mermaid
graph LR
    init[init.lua] --> layers[lib/layers.lua] --> merge[lib/merge.lua]
    layers --> s[settings/*] --> p[profiles/*] --> h[hosts/*] --> l[local.lua]
```

## Управление из CLI

```mermaid
sequenceDiagram
    participant C as dimaster ctl
    participant S as поток сокета
    participant V as View
    C->>S: аргументы через NUL
    S->>V: Wake::Remote(Request)
    V-->>S: результат
    S-->>C: ok / err и текст
```

## Интеграция с shell

```mermaid
sequenceDiagram
    participant S as zsh / bash / fish
    participant T as Terminal
    participant V as View
    S->>T: OSC 133;A (приглашение)
    S->>T: OSC 633;E;команда, 133;C
    S->>T: вывод
    S->>T: OSC 133;D;код
    T->>T: подсказка, снимок вывода
    V->>T: completed_commands()
    V->>V: свечение, уведомление, событие плагинам
```

## Потоки

```mermaid
sequenceDiagram
    participant R as поток разбора
    participant T as Terminal (FairMutex)
    participant W as поток окна
    R->>T: lock, advance(порция), unlock
    R-->>W: Wake (один на пачку)
    W->>T: lock на время событий
    W->>W: собрать кадр
    W->>T: unlock
    W->>W: показать кадр (ждёт монитор)
```

## Контейнеры (dimaster-docker)

```mermaid
classDiagram
    class App { items; stats; kind; mode; +press(key) +poll() +act(action) }
    class Worker { +start(period, stats) +watch(target) +run(host, args, show) }
    class Host { <<enum>> Local; Ssh; +command(args) }
    class Kind { <<enum>> +list_args() +parse(output) +columns() }
    class Task { <<enum>> Run; Show; Follow; Attach }
    class Tail { lines; +start(command) +poll() }
    class Tunnel { <<enum>> Open; Failed }
    class Mux { +path() }
    Host ..> Tunnel : ensure, socket
    Host ..> Mux : без туннеля
    class Docker { settings; view }
    App *-- Worker
    App *-- Docker
    Docker *-- DockerSettings
    Docker *-- Viewer
    App --> Host
    App --> Kind
    App ..> Task : actions
    App o-- Tail : логи
    Worker ..> Host
    Worker ..> Kind
```
