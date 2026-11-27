//! Встроенный нечёткий пикер файлов и частых каталогов (аналог fzf + zoxide).

use std::path::{Path, PathBuf};

use dimaster_config::Tuning;

use super::Press;
use super::launch::{self, Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Files,
    Dirs,
    Ssh,
    Snippets,
    Docker,
    /// Заполнение поля сниппета: набранное и есть значение.
    Fill,
}

#[derive(Debug)]
pub(crate) struct Picker {
    pub(crate) kind: Kind,
    pub(crate) items: Vec<String>,
    /// Что стоит за каждой строкой списков SSH, сниппетов и Docker;
    /// для `Fill` — один шаблон.
    pub(crate) values: Vec<String>,
    pub(crate) query: String,
    pub(crate) selected: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stay,
    Close,
    Insert(String),
    ChangeDir(String),
    Launch(Command),
    /// Спросить значения полей шаблона.
    Fill(String),
}

/// Оценка нечёткого совпадения подпоследовательности символов (больше — лучше).
pub(crate) fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let q_low = query.to_lowercase();
    let q: Vec<char> = q_low.chars().collect();
    let mut qi = 0usize;
    let (mut score, mut streak, mut prev_sep) = (0i32, 0i32, true);
    for ch in candidate.chars() {
        let low = ch.to_lowercase().next().unwrap_or(ch);
        if qi < q.len() && low == q[qi] {
            streak += 1;
            score += 4 + streak * 8 + if prev_sep { 10 } else { 0 };
            qi += 1;
        } else {
            streak = 0;
        }
        prev_sep = matches!(ch, '/' | '_' | '-' | ' ' | '.');
    }
    let exact = i32::from(candidate.to_lowercase().contains(&q_low)) * 40;
    (qi == q.len()).then_some(score + exact - candidate.len() as i32 / 8)
}

pub(crate) fn shell_escape(text: &str) -> String {
    if text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '~'))
    {
        return text.to_owned();
    }
    format!("'{}'", text.replace('\'', "'\\''"))
}

pub(crate) fn collect_files(root: &Path, limits: &Tuning) -> Vec<String> {
    let mut out = Vec::new();
    walk(root, Path::new(""), 0, &mut out, limits);
    out.sort();
    out
}

fn walk(base: &Path, rel: &Path, depth: usize, out: &mut Vec<String>, limits: &Tuning) {
    if depth > limits.picker_depth || out.len() >= limits.picker_files {
        return;
    }
    let Ok(entries) = std::fs::read_dir(base.join(rel)) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= limits.picker_files {
            break;
        }
        visit_entry(base, rel, depth, entry, out, limits);
    }
}

fn visit_entry(
    base: &Path,
    rel: &Path,
    depth: usize,
    entry: std::fs::DirEntry,
    out: &mut Vec<String>,
    limits: &Tuning,
) {
    let name = entry.file_name();
    let Some(name_str) = name.to_str() else {
        return;
    };
    let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
    if is_dir && limits.picker_skip.iter().any(|s| s == name_str) {
        return;
    }
    let next = rel.join(&name);
    let display = next.to_string_lossy().into_owned();
    out.push(if is_dir {
        format!("{display}/")
    } else {
        display
    });
    if is_dir {
        walk(base, &next, depth + 1, out, limits);
    }
}

pub(crate) fn merge_dirs(root: &Path, ranked: &[(u32, String)]) -> Vec<String> {
    let mut out: Vec<String> = ranked.iter().map(|(_, p)| p.clone()).collect();
    if let Ok(entries) = std::fs::read_dir(root) {
        let mut local: Vec<String> = entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|e| e.file_name().to_str().map(|n| root.join(n)))
            .map(|p: PathBuf| p.to_string_lossy().into_owned())
            .filter(|p| !out.contains(p))
            .collect();
        local.sort();
        out.extend(local);
    }
    out
}

impl Picker {
    pub(crate) fn matches(&self) -> Vec<&str> {
        let mut scored: Vec<(i32, usize, &str)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| Some((fuzzy_score(&self.query, item)?, idx, item.as_str())))
            .collect();
        if !self.query.is_empty() {
            scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        }
        scored.into_iter().map(|(_, _, item)| item).collect()
    }

    pub(crate) fn press(&mut self, press: Press) -> Outcome {
        let count = self.matches().len();
        if let Some(outcome) = self.special(press) {
            return outcome;
        }
        match press {
            Press::Escape | Press::Ctrl('c') => Outcome::Close,
            Press::Up | Press::Ctrl('k') => {
                self.selected = (self.selected + count.max(1) - 1) % count.max(1);
                Outcome::Stay
            }
            Press::Down | Press::Ctrl('j') => {
                self.selected = (self.selected + 1) % count.max(1);
                Outcome::Stay
            }
            Press::Enter { shift } => self.accept(shift),
            Press::Backspace => {
                self.query.pop();
                self.retype()
            }
            Press::Char(ch) => {
                self.query.push(ch);
                self.retype()
            }
            _ => Outcome::Stay,
        }
    }

    // новый запрос начинает выбор сверху
    fn retype(&mut self) -> Outcome {
        self.selected = 0;
        Outcome::Stay
    }

    // значение выбранной строки; у поясняющей строки пустого списка его нет
    fn chosen_value(&self) -> Option<&str> {
        let chosen = self.matches().get(self.selected).copied()?;
        let index = self.items.iter().position(|i| i == chosen)?;
        self.values.get(index).map(String::as_str)
    }

    // клавиши, смысл которых зависит от вида списка
    fn special(&self, press: Press) -> Option<Outcome> {
        let enter = match press {
            Press::Enter { shift } => Some(shift),
            _ => None,
        };
        match self.kind {
            Kind::Ssh => {
                let shift = enter?;
                let host = self.chosen_value().map(|h| launch::ssh(h, shift));
                Some(host.map_or(Outcome::Close, Outcome::Launch))
            }
            Kind::Snippets => enter.map(|_| self.snippet()),
            Kind::Docker => {
                let (name, up) = self.chosen_value()?.split_once('\t')?;
                launch::docker(name, up == "1", press).map(Outcome::Launch)
            }
            Kind::Fill => enter.map(|_| self.filled()),
            Kind::Files | Kind::Dirs => None,
        }
    }

    fn snippet(&self) -> Outcome {
        self.chosen_value()
            .map_or(Outcome::Close, |template| after_fill(template.to_owned()))
    }

    fn filled(&self) -> Outcome {
        let template = self.values.first().map(String::as_str).unwrap_or_default();
        after_fill(launch::fill_field(template, &self.query))
    }

    fn accept(&self, shift: bool) -> Outcome {
        let Some(chosen) = self.matches().get(self.selected).copied() else {
            return Outcome::Close;
        };
        let escaped = shell_escape(chosen);
        match (self.kind, shift) {
            (Kind::Dirs, false) => Outcome::ChangeDir(escaped),
            _ => Outcome::Insert(escaped),
        }
    }
}

// пока в шаблоне есть поля — спрашиваем дальше, потом вставляем
fn after_fill(template: String) -> Outcome {
    match launch::next_field(&template) {
        Some(_) => Outcome::Fill(template),
        None => Outcome::Launch(Command {
            text: template,
            run: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_ranks_contiguous_matches_higher() {
        let exact = fuzzy_score("syn", "src/syntax.rs").unwrap();
        let spread = fuzzy_score("syn", "src/a_y_n_file.rs").unwrap();
        assert!(exact > spread);
        assert!(fuzzy_score("zzz", "src/syntax.rs").is_none());
        assert_eq!(shell_escape("a b.txt"), "'a b.txt'");
    }
}
