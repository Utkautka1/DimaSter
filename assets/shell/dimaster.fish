# Интеграция DimaSter для fish.
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
