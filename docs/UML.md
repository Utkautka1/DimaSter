# DimaSter: диаграммы

## Зависимости крейтов

```mermaid
graph TD
    app[dimaster] --> grid[dimaster-grid]
    app --> pty[dimaster-pty]
    grid --> vt[dimaster-vt]
```

## Классы

```mermaid
classDiagram
    class Parser { +advance(perform, bytes) }
    class Perform {
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
    class Modes
    class Pty { +spawn(command, size) +resize(size) +wait() }

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

## Поток данных

```mermaid
sequenceDiagram
    participant S as shell
    participant P as Pty
    participant T as Terminal
    S->>P: вывод
    P->>T: advance(bytes)
    T-->>P: take_reply()
    P-->>S: ответ на запрос
```
