//! Списки SSH, сниппетов и Docker поверх общего пикера.

use dimaster_i18n::tr;

use crate::modes::Mode;
use crate::modes::launch::Command;
use crate::modes::picker::{Kind, Picker};
use crate::sources;
use crate::view::View;

impl View {
    // в полноэкранной программе список напечатал бы команду не туда
    fn open_list(&mut self, kind: Kind, items: Vec<String>, values: Vec<String>) {
        if self.term.is_alternate() {
            return;
        }
        self.mode = Mode::Picker(Picker {
            kind,
            items,
            values,
            query: String::new(),
            selected: 0,
        });
        self.window.request_redraw();
    }

    // пустой список объясняет, почему он пуст
    fn items_or(items: Vec<String>, empty: &str) -> Vec<String> {
        match items.is_empty() {
            true => vec![tr(empty)],
            false => items,
        }
    }

    pub(crate) fn open_ssh(&mut self) {
        let hosts = sources::ssh_hosts();
        let items = Self::items_or(hosts.clone(), "в ~/.ssh/config нет серверов");
        self.open_list(Kind::Ssh, items, hosts);
    }

    pub(crate) fn open_snippets(&mut self) {
        let snippets = &self.config.snippets;
        let items = snippets
            .iter()
            .map(|s| format!("{}  —  {}", s.name, s.command));
        let items = Self::items_or(
            items.collect(),
            "нет сниппетов: добавьте snippets в local.lua",
        );
        let values = snippets.iter().map(|s| s.command.clone()).collect();
        self.open_list(Kind::Snippets, items, values);
    }

    pub(crate) fn open_docker(&mut self) {
        let found = sources::containers();
        let empty = match found {
            Some(_) => "контейнеров нет",
            None => "Docker не отвечает",
        };
        let found = found.unwrap_or_default();
        let items = Self::items_or(found.iter().map(|c| c.line.clone()).collect(), empty);
        let values = found
            .iter()
            .map(|c| format!("{}\t{}", c.name, u8::from(c.up)))
            .collect();
        self.open_list(Kind::Docker, items, values);
    }

    /// Выполняет только у пустого приглашения; иначе команда лишь вставляется.
    pub(crate) fn launch(&mut self, command: Command) {
        let prompt = self.term.input_start().is_some();
        let busy_line = crate::syntax::input_text_at_end(&self.term).is_some();
        let mut text = command.text;
        // перевод строки из сниппета выполнил бы команду сам
        text.retain(|c| !c.is_control());
        if command.run && prompt && !busy_line {
            text.push('\r');
        }
        self.type_bytes(text.into_bytes());
    }
}
