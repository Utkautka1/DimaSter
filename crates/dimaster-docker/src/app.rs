//! Состояние экрана: вкладка, сервер, строки и то, куда идут клавиши.

use std::time::Duration;

use dimaster_config::Docker;
use dimaster_i18n::{t, tr};

use crate::actions::{self, Context, Task};
use crate::host::Host;
use crate::kind::{Item, KINDS, Kind};
use crate::logs::Tail;
use crate::worker::{Event, Stats, Target, Worker};

pub(crate) enum Mode {
    List,
    /// Набирается фильтр после `/`.
    Filter,
    /// Ждём `y` перед удалением; второе поле — что удаляется.
    Confirm(Task, String),
    /// Вывод `inspect` или своей команды.
    Text {
        lines: Vec<String>,
        scroll: usize,
    },
    /// Живые логи; `back` — на сколько строк отмотано от конца.
    Logs {
        tail: Tail,
        back: usize,
    },
}

fn question(action: &str, name: String) -> String {
    match action {
        "prune" => tr("Удалить всё неиспользуемое?"),
        "down" => t!("Остановить и удалить проект {name}?", name = name),
        _ => t!("Удалить {name}?", name = name),
    }
}

pub struct App {
    pub(crate) config: Docker,
    hosts: Vec<Host>,
    host: usize,
    pub(crate) kind: Kind,
    pub(crate) items: Vec<Item>,
    pub(crate) stats: Stats,
    /// Номер выбранной строки среди прошедших фильтр.
    pub(crate) selected: usize,
    pub(crate) top: usize,
    pub(crate) filter: String,
    pub(crate) mode: Mode,
    pub(crate) message: String,
    pub(crate) loading: bool,
    // строк списка на экране; задаётся отрисовкой
    pub(crate) page: usize,
    // программа, ждущая, когда ей отдадут терминал
    pub(crate) attach: Option<Vec<String>>,
    pub(crate) done: bool,
    worker: Worker,
}

impl App {
    /// `servers` — имена из `~/.ssh/config`; `docker.lua` может задать свои.
    pub fn new(config: Docker, servers: &[String]) -> Self {
        let listed = config.settings.hosts.as_deref().unwrap_or(servers);
        let remote = listed.iter().filter_map(|name| Host::ssh(name));
        let hosts = std::iter::once(Host::Local).chain(remote).collect();
        crate::timing::set(config.settings.timing);
        let period = Duration::from_secs(config.settings.refresh);
        let worker = Worker::start(period, config.settings.stats);
        let app = Self::initial(config, hosts, worker);
        app.refresh();
        app
    }

    // все поля в начальном состоянии
    fn initial(config: Docker, hosts: Vec<Host>, worker: Worker) -> Self {
        Self {
            config,
            hosts,
            host: 0,
            kind: Kind::default(),
            items: Vec::new(),
            stats: Stats::new(),
            selected: 0,
            top: 0,
            filter: String::new(),
            mode: Mode::List,
            message: String::new(),
            loading: true,
            page: 20,
            attach: None,
            done: false,
            worker,
        }
    }

    pub(crate) fn host(&self) -> &Host {
        &self.hosts[self.host]
    }

    fn target(&self) -> Target {
        Target {
            host: self.host().clone(),
            kind: self.kind,
        }
    }

    pub(crate) fn refresh(&self) {
        self.worker.watch(&self.target());
    }

    // другая вкладка или сервер: прежние строки не имеют смысла
    fn switched(&mut self) {
        self.items.clear();
        self.stats.clear();
        self.filter.clear();
        (self.selected, self.top, self.loading) = (0, 0, true);
        self.message.clear();
        self.refresh();
    }

    pub(crate) fn turn_tab(&mut self, step: usize) {
        let at = KINDS.iter().position(|&k| k == self.kind).unwrap_or(0);
        self.kind = KINDS[(at + step) % KINDS.len()];
        self.switched();
    }

    pub(crate) fn turn_host(&mut self, step: usize) {
        self.host = (self.host + step) % self.hosts.len();
        self.switched();
    }

    pub(crate) fn host_count(&self) -> usize {
        self.hosts.len()
    }

    /// Строки, прошедшие фильтр.
    pub(crate) fn visible(&self) -> Vec<&Item> {
        let wanted = self.filter.to_lowercase();
        let matches = |item: &&Item| {
            let mut cells = item.cells.iter();
            wanted.is_empty() || cells.any(|c| c.to_lowercase().contains(&wanted))
        };
        self.items.iter().filter(matches).collect()
    }

    pub(crate) fn current(&self) -> Option<Item> {
        self.visible().get(self.selected).map(|&i| i.clone())
    }

    // выбор следует за строкой, даже если список перестроился
    fn listed(&mut self, items: Vec<Item>) {
        let chosen = self.current().map(|i| i.id);
        self.items = items;
        let found = self
            .visible()
            .iter()
            .position(|i| Some(&i.id) == chosen.as_ref());
        let last = self.visible().len().saturating_sub(1);
        self.selected = found.unwrap_or(self.selected).min(last);
    }

    fn receive(&mut self, event: Event) {
        match event {
            // ответ мог прийти уже после смены вкладки
            Event::Listed(target, result) if target == self.target() => {
                self.loading = false;
                match result {
                    Ok(items) => self.listed(items),
                    Err(error) => (self.message, self.items) = (error, Vec::new()),
                }
            }
            Event::Stats(host, stats) if host == *self.host() => self.stats = stats,
            Event::Listed(..) | Event::Stats(..) => {}
            Event::Done(result) => {
                self.message = result.map_or_else(|e| e, |()| tr("Готово"));
                self.refresh();
            }
            Event::Shown(Ok(text)) => {
                let lines = text.lines().map(str::to_owned).collect();
                self.mode = Mode::Text { lines, scroll: 0 };
            }
            Event::Shown(Err(error)) => self.message = error,
        }
    }

    /// Забирает новости из фоновых потоков; `true` — нужно перерисовать.
    pub(crate) fn poll(&mut self) -> bool {
        let events: Vec<Event> = self.worker.incoming.try_iter().collect();
        let fresh = !events.is_empty();
        events.into_iter().for_each(|e| self.receive(e));
        let logs = match &mut self.mode {
            Mode::Logs { tail, .. } => tail.poll(),
            _ => false,
        };
        fresh | logs
    }

    pub(crate) fn perform(&mut self, task: Task) {
        match task {
            Task::Run(args) => {
                self.message = tr("Выполняется…");
                self.worker.run(self.host(), &args, false);
            }
            Task::Show(args) => self.worker.run(self.host(), &args, true),
            Task::Follow(args) => match Tail::start(self.host().command(&args)) {
                Ok(tail) => self.mode = Mode::Logs { tail, back: 0 },
                Err(e) => self.message = e.to_string(),
            },
            Task::Attach(args) => self.attach = Some(args),
        }
    }

    /// Действие над выбранной строкой; удаление сначала переспрашивает.
    pub(crate) fn act(&mut self, action: &str) {
        // очистка не требует выбранной строки
        let item = self.current().unwrap_or_default();
        if item.id.is_empty() && action != "prune" {
            return;
        }
        let settings = &self.config.settings;
        let context = Context {
            host: self.host(),
            tail: settings.log_tail,
            shell: &settings.shell,
        };
        let Some(task) = actions::task(self.kind, action, &item, &context) else {
            return;
        };
        if settings.confirm && actions::destructive(action) {
            self.mode = Mode::Confirm(task, question(action, item.name));
        } else {
            self.perform(task);
        }
    }

    /// Своя команда из `docker.lua`; `false` — клавиша не её.
    pub(crate) fn custom(&mut self, key: &str) -> bool {
        let tab = self.kind.name();
        let found = self
            .config
            .settings
            .commands
            .iter()
            .find(|c| c.key == key && (c.tabs.is_empty() || c.tabs.iter().any(|t| t == tab)));
        let (Some(command), Some(item)) = (found, self.current()) else {
            return false;
        };
        let fill = |a: &String| a.replace("{id}", &item.id).replace("{name}", &item.name);
        let args = command.args.iter().map(fill).collect();
        self.perform(Task::Show(args));
        true
    }
}
