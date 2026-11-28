//! Все известные флаги команды. Источники складываются:
//! список из настроек, дополнения `fish` и `man`; `--help` — по желанию.
//!
//! `--help` запускает саму программу, поэтому по умолчанию выключен.

use std::process::Command;

use dimaster_config::Native;
use dimaster_docker::capture;

// самая подробная команда первой: `git push`, затем `git`
fn names(typed: &[&str]) -> Vec<Vec<String>> {
    let plain = |w: &&str| {
        let safe = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.');
        !w.starts_with('-') && w.chars().all(safe)
    };
    let words: Vec<String> = typed
        .iter()
        .copied()
        .take_while(plain)
        .take(3)
        .map(str::to_owned)
        .collect();
    (1..=words.len())
        .rev()
        .map(|n| words[..n].to_vec())
        .collect()
}

fn output(program: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args).env("MANWIDTH", "100");
    let wait = dimaster_limits::SENSOR_TIMEOUT.secs();
    capture(command, wait).ok().filter(|text| !text.is_empty())
}

fn variants(line: &str) -> Vec<String> {
    match line.contains("[no-]") {
        true => vec![line.replace("[no-]", ""), line.replace("[no-]", "no-")],
        false => vec![line.to_owned()],
    }
}

// строки вида `-a, --all` из справки
fn parse(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim_start)
        .filter(|l| l.starts_with('-'))
        // `--[no-]verify` — это два флага
        .flat_map(variants)
        .flat_map(|l| {
            let words = l.split([' ', '\t', ',', '=', '[', '<', '|']);
            words.map(str::to_owned).collect::<Vec<_>>()
        })
        .filter(|w| {
            w.starts_with('-') && !w.starts_with("---") && !w.trim_start_matches('-').is_empty()
        })
        .filter(|w| w.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .collect()
}

// строка уходит аргументом, а не в текст сценария; подстановки не передаются
fn from_fish(typed: &[&str], start: &str) -> Vec<String> {
    let line = format!("{} {start}", typed.join(" "));
    if line.contains(['$', '(', ')', '`', ';', '|', '&', '<', '>', '{', '}']) {
        return Vec::new();
    }
    let script = "complete -C\"$argv[1]\"";
    let text = output("fish", &["--private", "-c", script, &line]);
    parse(&text.unwrap_or_default())
}

fn from_manual(typed: &[&str]) -> Vec<String> {
    let mut pages = names(typed).into_iter().map(|n| n.join("-"));
    let text = pages.find_map(|page| output("man", &["-P", "cat", &page]));
    parse(&text.unwrap_or_default())
}

fn from_help(typed: &[&str]) -> Vec<String> {
    let Some(words) = names(typed).into_iter().next() else {
        return Vec::new();
    };
    let mut args: Vec<&str> = words[1..].iter().map(String::as_str).collect();
    args.push("--help");
    parse(&output(&words[0], &args).unwrap_or_default())
}

// свои списки: ключ — команда с подкомандами
fn from_settings(native: &Native, typed: &[&str]) -> Vec<String> {
    let known = names(typed).into_iter().map(|n| n.join(" "));
    let mut found = known.filter_map(|name| native.flags.iter().find(|f| f.0 == name));
    found.next().map(|f| f.1.clone()).unwrap_or_default()
}

// всё, что известно без запуска самой программы
fn known(native: &Native, typed: &[&str], start: &str) -> Vec<String> {
    let mut flags = from_settings(native, typed);
    if native.fish_flags {
        flags.extend(from_fish(typed, start));
    }
    if native.man_flags {
        flags.extend(from_manual(typed));
    }
    flags
}

// опечатка (`--onelain`): ближайшие по написанию, не дальше двух правок
fn similar(flags: &[String], typed: &str) -> Vec<String> {
    let scored = flags.iter().filter(|_| typed.len() > 3);
    let mut near: Vec<(usize, &String)> = scored
        .filter_map(|f| Some((crate::syntax::edit_distance(typed, f, 2)?, f)))
        .collect();
    near.sort();
    let best = near.first().map(|n| n.0);
    near.into_iter()
        .filter(|n| Some(n.0) == best)
        .map(|n| n.1.clone())
        .collect()
}

fn matching(mut flags: Vec<String>, start: &str) -> Vec<String> {
    flags.sort();
    flags.dedup();
    let exact: Vec<String> = flags
        .iter()
        .filter(|f| f.starts_with(start))
        .cloned()
        .collect();
    match exact.is_empty() {
        true => similar(&flags, start),
        false => exact,
    }
}

/// Флаги команды `typed`, начинающиеся с `start`; при опечатке — похожие.
pub(crate) fn find(native: &Native, typed: &[&str], start: &str) -> Vec<String> {
    // fish спрашивается с `--`, чтобы при опечатке было из чего выбирать
    let dashes = if start.starts_with("--") { "--" } else { "-" };
    let found = matching(known(native, typed, dashes), start);
    // саму программу спрашиваем, только когда иначе ничего не нашлось
    if found.is_empty() && native.help_flags {
        return matching(from_help(typed), start);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_flags_from_help_text() {
        let text = "  -a, --all\n         text -x\n  --color[=WHEN]\n - item\n  --[no-]verify";
        let mut found = parse(text);
        found.sort();
        found.dedup();
        assert_eq!(found, ["--all", "--color", "--no-verify", "--verify", "-a"]);
        assert_eq!(parse("--force\tForce an update"), ["--force"]);
        // рисунки из man (`---X---A`) флагами не являются
        assert!(parse("---X---A").is_empty());
    }

    #[test]
    fn subcommand_comes_before_the_program() {
        let found = names(&["git", "push", "-q", "origin"]);
        assert_eq!(found, [vec!["git", "push"], vec!["git"]]);
        assert!(names(&["--help"]).is_empty());
    }

    #[test]
    fn typo_offers_the_closest_flag() {
        let flags = ["--oneline", "--online-help", "--all"].map(str::to_owned);
        assert_eq!(matching(flags.to_vec(), "--onelain"), ["--oneline"]);
        assert!(matching(flags.to_vec(), "--zzzzzz").is_empty());
    }

    #[test]
    fn own_flags_are_found_by_command() {
        let native = Native {
            fish_flags: false,
            man_flags: false,
            flags: vec![("my deploy".into(), vec!["--prod".into(), "--dry".into()])],
            ..Native::default()
        };
        assert_eq!(find(&native, &["my", "deploy"], "--p"), ["--prod"]);
        assert!(find(&native, &["my"], "--").is_empty());
    }
}
