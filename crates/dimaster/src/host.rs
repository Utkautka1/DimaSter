//! Плагины и внешние команды внутри открытого окна.

use dimaster_config::{Action, Effect, Plugins, Snapshot, config_dir};

use crate::desktop;
use crate::remote::Request;
use crate::restore::open_window;
use crate::view::View;

// плагин, вызывающий действие из обработчика этого действия, не зациклит окно
const MAX_DEPTH: u8 = 4;

impl View {
    /// (Пере)загружает плагины и сообщает им о старте.
    pub(crate) fn load_plugins(&mut self) {
        let Some(directory) = config_dir() else {
            return;
        };
        match Plugins::load(&directory) {
            Ok((plugins, errors)) => {
                errors
                    .iter()
                    .for_each(|e| log::warn!("Плагин не загружен: {e}"));
                self.plugins = Some(plugins);
                self.emit("startup", "");
                self.render_status();
            }
            Err(e) => log::warn!("Плагины недоступны: {e}"),
        }
    }

    pub(crate) fn plugin_snapshot(&self) -> Snapshot {
        let selection = self.selecting.selection.map(|s| self.term.selected_text(s));
        Snapshot {
            title: self.term.title().to_owned(),
            cwd: self
                .shell_directory()
                .map(|d| d.display().to_string())
                .unwrap_or_default(),
            selection: selection.unwrap_or_default(),
            screen: self.term.text(),
        }
    }

    pub(crate) fn emit(&mut self, event: &str, value: &str) {
        let Some(plugins) = &self.plugins else { return };
        let effects = plugins.emit(event, value, self.plugin_snapshot());
        self.apply_effects(effects);
    }

    pub(crate) fn run_plugin(&mut self, name: &str) {
        let Some(plugins) = &self.plugins else { return };
        let effects = plugins.run(name, self.plugin_snapshot());
        self.apply_effects(effects);
    }

    fn apply_effects(&mut self, effects: Vec<Effect>) {
        if self.plugin_depth >= MAX_DEPTH {
            return log::warn!("Плагины вызывают друг друга по кругу");
        }
        self.plugin_depth += 1;
        for effect in effects {
            match effect {
                Effect::Send(text) => self.type_bytes(text.into_bytes()),
                Effect::Notify { title, body } => desktop::notify(&title, &body),
                Effect::Action(name) => match Action::parse(&name) {
                    Some(action) => self.perform(action),
                    None => log::warn!("Плагин: неизвестное действие {name}"),
                },
            }
        }
        self.plugin_depth -= 1;
    }

    // команды, которые только читают состояние
    fn query(&self, words: &[&str]) -> Option<String> {
        match words {
            ["get-text"] => Some(self.term.text()),
            ["info"] => Some(self.info()),
            _ => None,
        }
    }

    fn command(&mut self, args: &[String]) -> Result<String, String> {
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        if let Some(answer) = self.query(&words) {
            return Ok(answer);
        }
        match words.as_slice() {
            ["send-text", text] => self.type_bytes(text.as_bytes().to_vec()),
            ["action", name] => {
                let action = Action::parse(name).ok_or_else(|| format!("нет действия {name}"))?;
                self.perform(action);
            }
            ["set-title", title] => self.window.set_title(title),
            ["reload"] => self.reload(),
            ["new-window"] => open_window(self.shell_directory(), None),
            _ => return Err(format!("неизвестная команда: {}", words.join(" "))),
        }
        Ok(String::new())
    }

    fn info(&self) -> String {
        let cwd = self
            .shell_directory()
            .map(|d| d.display().to_string())
            .unwrap_or_default();
        let screen = self.term.screen();
        let (rows, cols) = (screen.rows().len(), screen.rows()[0].cells.len());
        format!(
            "pid={}\ntitle={}\ncwd={cwd}\nsize={rows}x{cols}",
            std::process::id(),
            self.term.title()
        )
    }

    pub(crate) fn remote(&mut self, request: Request) {
        let result = self.command(&request.args);
        // клиент мог уже отключиться
        let _ = request.reply.send(result);
        self.window.request_redraw();
    }
}
