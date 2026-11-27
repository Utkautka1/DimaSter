# Интеграция DimaSter для fish.
if status is-interactive; and set -q DIMASTER_STARTUP_FETCH
    function __dimaster_startup_fetch --on-event fish_prompt
        functions -e __dimaster_startup_fetch
        set -l bin "$DIMASTER_BIN"
        set -e DIMASTER_STARTUP_FETCH DIMASTER_BIN
        if test -n "$bin" -a -x "$bin"
            "$bin" fetch
        else if command -q dimaster
            dimaster fetch
        end
    end
end
function __dimaster_preexec --on-event fish_preexec
    set -l line (string replace -ra '[[:cntrl:]]' ' ' -- $argv[1] | string join ' ')
    printf '\e]633;E;%s\a\e]133;C\a' "$line"
end
function __dimaster_postexec --on-event fish_postexec
    printf '\e]133;D;%s\a' $status
end
function __dimaster_prompt --on-event fish_prompt
    printf '\e]133;A\a'
end
# заменяется только штатный обработчик fish, не ваш и не дистрибутивный
if set -q DIMASTER_NOT_FOUND; and string match -q '*/share/fish/*' -- (functions --details fish_command_not_found)
    function fish_command_not_found
        printf '%s: %s\n' $argv[1] "$DIMASTER_NOT_FOUND" >&2
    end
end
if status is-interactive; and set -q DIMASTER_COLOR_LS
    if not functions -q ls
        if command -q eza
            function ls --wraps=eza
                command eza --icons=always --color=auto $argv
            end
            if not functions -q ll
                function ll --wraps=eza
                    command eza -al --icons=always --group-directories-first --color=auto $argv
                end
            end
        else
            function ls --wraps=ls
                command ls --color=auto $argv
            end
        end
    end
    set -e DIMASTER_COLOR_LS
end
