# DimaSter

Терминал для Linux на Rust. Сейчас готов этап 1: ядро без окна.

## Запуск

Headless-режим выполняет команду в PTY и печатает получившийся экран:

```bash
cargo run --release -p dimaster -- ls -la
```

## Проверки

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets
```

```bash
cargo bench -p dimaster-grid
```

## Структура

| Крейт | Зачем |
|---|---|
| `dimaster-vt` | разбор escape-последовательностей |
| `dimaster-grid` | экран, история, режимы |
| `dimaster-pty` | запуск shell |
| `dimaster` | приложение |

Диаграммы: [docs/UML.md](docs/UML.md). Требования: [docs/NOTES.md](docs/NOTES.md).
