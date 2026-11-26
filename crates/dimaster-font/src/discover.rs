//! Выбор основного шрифта и поиск запасных для отсутствующих символов.

use std::process::Command;

use fontdb::{Database, Family, ID, Query, Style, Weight};

/// Имя семейства: из окружения, затем из fontconfig, затем любое моноширинное.
pub(crate) fn default_family(db: &Database) -> Option<String> {
    std::env::var("DIMASTER_FONT")
        .ok()
        .or_else(fontconfig_monospace)
        .or_else(|| {
            let face = db.faces().find(|f| f.monospaced)?;
            Some(face.families.first()?.0.clone())
        })
}

// fc-match уважает пользовательские настройки fontconfig; без него обходимся сканом
fn fontconfig_monospace() -> Option<String> {
    let output = Command::new("fc-match")
        .args(["-f", "%{family[0]}", "monospace"])
        .output()
        .ok()?;
    let name = String::from_utf8(output.stdout).ok()?;
    (!name.is_empty()).then_some(name)
}

pub(crate) fn find_face(db: &Database, family: &str, bold: bool, italic: bool) -> Option<ID> {
    db.query(&Query {
        families: &[Family::Name(family), Family::Monospace],
        weight: if bold { Weight::BOLD } else { Weight::NORMAL },
        style: if italic { Style::Italic } else { Style::Normal },
        ..Query::default()
    })
}

/// Первый шрифт с нужным символом; моноширинные проверяются раньше.
pub(crate) fn fallback_face(db: &Database, ch: char) -> Option<ID> {
    let has = |id: ID| {
        db.with_face_data(id, |data, index| {
            ttf_parser::Face::parse(data, index).is_ok_and(|f| f.glyph_index(ch).is_some())
        })
        .unwrap_or(false)
    };
    let mono = db.faces().filter(|f| f.monospaced);
    let rest = db.faces().filter(|f| !f.monospaced);
    mono.chain(rest).map(|f| f.id).find(|&id| has(id))
}
