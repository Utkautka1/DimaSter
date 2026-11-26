//! Подсказки под командами, история команд и метки.

use dimaster_grid::{PromptMark, Size, SmartOptions, Terminal};

fn terminal() -> Terminal {
    let mut term = Terminal::new(Size { rows: 20, cols: 60 });
    term.set_smart_options(SmartOptions {
        ok: "OK".into(),
        empty: "ничего".into(),
        errors: true,
        search_tools: vec!["grep".into()],
    });
    term
}

// то, что присылают скрипты интеграции вокруг одной команды
fn command(term: &mut Terminal, text: &str, output: &str, exit: i32) {
    let stream = format!(
        "\x1b]133;A\x07$ {text}\r\n\x1b]633;E;{text}\x07\x1b]133;C\x07{output}\x1b]133;D;{exit}\x07"
    );
    term.advance(stream.as_bytes());
}

#[test]
fn hints_for_silent_failed_and_search_commands() {
    let mut term = terminal();
    command(&mut term, "true", "", 0);
    command(&mut term, "grep x f", "", 1);
    command(&mut term, "boom", "", 139);
    command(&mut term, "echo hi", "hi\r\n", 0);
    let text = term.text();
    assert!(text.contains("$ true\nOK"), "{text}");
    assert!(text.contains("$ grep x f\nничего"), "{text}");
    assert!(text.contains("✗ 139: SIGSEGV"), "{text}");
    assert!(text.ends_with("$ echo hi\nhi"), "{text}");
}

#[test]
fn history_keeps_output_and_marks_failures() {
    let mut term = terminal();
    command(&mut term, "make", "a\r\nb\r\n", 2);
    let finished = term.commands().next().unwrap();
    assert_eq!((finished.command.as_str(), finished.exit), ("make", 2));
    assert_eq!(finished.output, ["a", "b"]);
    assert_eq!(term.completed_commands(), 1);
    assert_eq!(term.line(0).unwrap().mark, Some(PromptMark::Failed));
    assert_eq!(term.prompt_line(5, false), Some(0));
}

#[test]
fn empty_enter_and_full_screen_programs_stay_quiet() {
    let mut term = terminal();
    term.advance(b"\x1b]133;D;0\x07");
    assert_eq!(term.completed_commands(), 0);
    command(&mut term, "vim", "\x1b[?1049h\x1b[?1049l", 0);
    assert!(!term.text().contains("OK"));
}

// программа уже объяснила ошибку сама — вторая строка не нужна
#[test]
fn failure_with_output_gets_no_hint() {
    let mut term = dimaster_grid::Terminal::new(dimaster_grid::Size { rows: 6, cols: 40 });
    term.advance(b"\x1b]133;A\x07$ lol\r\n\x1b]633;E;lol\x07\x1b]133;C\x07");
    term.advance(b"zsh: command not found: lol\r\n\x1b]133;D;127\x07");
    let screen: String = term.screen().rows().iter().map(|r| r.text()).collect();
    assert!(!screen.contains('✗'), "{screen}");
}
