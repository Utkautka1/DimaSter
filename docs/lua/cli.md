# Команды и управление из консоли

[← к оглавлению](README.md)

## Запуск

| Команда | Что делает |
|---|---|
| `dimaster` | окно с вашим shell |
| `dimaster -e htop` | окно с программой; интеграция shell подключается, только если это сам shell без аргументов |
| `dimaster --class имя …` | свой app_id окна; должен идти первым |
| `dimaster --setup` | мастер настройки; изменённые файлы копируются в `backups/` |
| `dimaster --headless команда` | выполнить без окна и напечатать получившийся экран |
| `dimaster fetch [--logo файл]` | сведения о системе |
| `dimaster icat файл…` | показать картинку или видео (внутри DimaSter) |
| `dimaster ssh-terminfo хост` | установить описание терминала на сервере |

## `dimaster ctl`: управление открытыми окнами

Работает, пока включён `remote_control`. Внутри окна DimaSter команда обращается к нему же; снаружи при нескольких окнах задайте `DIMASTER_SOCKET`.

| Команда | Что делает |
|---|---|
| `dimaster ctl list` | открытые окна и их сокеты |
| `dimaster ctl info` | pid, заголовок, каталог, размер |
| `dimaster ctl get-text` | видимый текст экрана |
| `dimaster ctl send-text 'ls'` | ввести текст в программу |
| `dimaster ctl action command_palette` | выполнить действие терминала |
| `dimaster ctl set-title 'сборка'` | заголовок окна |
| `dimaster ctl reload` | перечитать настройки |
| `dimaster ctl new-window` | новое окно в том же каталоге |

Список действий для `action` — на странице [Клавиши и действия](settings/keys.md).

```bash
dimaster ctl action toggle_monitor
```

## Переменные окружения

| Переменная | Назначение |
|---|---|
| `DIMASTER_PROFILE` | какой профиль из `profiles/` подключить |
| `DIMASTER_FONT` | шрифт, если он не задан в настройках |
| `DIMASTER_TERM` | значение `TERM` |
| `DIMASTER_SOCKET` | сокет окна для `ctl` |
| `RUST_LOG=info` | подробный журнал |
| `TERM_PROGRAM=dimaster` | выставляет сам терминал; удобно для проверки в `.zshrc` |
