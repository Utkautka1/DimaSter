# DimaSter

Терминал для Linux на Rust. Готовы этапы 1 (ядро), 2 (окно, GPU, ввод) и 3 (совместимость).

## Запуск

Окно с вашим shell:

```bash
cargo run --release -p dimaster
```

Другая программа в окне:

```bash
cargo run --release -p dimaster -- -e htop
```

Без окна (печатает получившийся экран):

```bash
cargo run --release -p dimaster -- --headless ls -la
```

| Сочетание | Действие |
|---|---|
| Ctrl+Shift+V | вставить |
| Shift+PageUp / PageDown, колесо | прокрутка истории |
| Shift + мышь | мышь для терминала, даже если программа её захватила |

Своя запись terminfo включается переменной `DIMASTER_TERM=xterm-dimaster`.

Шрифт можно временно сменить переменной `DIMASTER_FONT`.

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
| `dimaster-font` | поиск шрифтов, свой растеризатор |
| `dimaster-render` | отрисовка на wgpu |
| `dimaster` | приложение |

Диаграммы: [docs/UML.md](docs/UML.md). Требования: [docs/NOTES.md](docs/NOTES.md).
