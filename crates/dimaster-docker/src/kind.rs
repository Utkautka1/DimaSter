//! Вкладки: что запросить у Docker и как разобрать ответ в строки таблицы.

use serde_json::Value;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    #[default]
    Containers,
    Compose,
    Images,
    Volumes,
    Networks,
}

pub(crate) const KINDS: [Kind; 5] = [
    Kind::Containers,
    Kind::Compose,
    Kind::Images,
    Kind::Volumes,
    Kind::Networks,
];

/// Строка вкладки.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Item {
    /// Что передавать командам `docker`.
    pub(crate) id: String,
    pub(crate) name: String,
    /// Значения в порядке [`Kind::columns`].
    pub(crate) cells: Vec<String>,
    // файлы compose-проекта
    pub(crate) files: Vec<String>,
}

// колонка таблицы и поле JSON, из которого она берётся
type Column = (&'static str, &'static str);

// пустое поле заполняется не из этого ответа
const CONTAINERS: &[Column] = &[
    ("name", "Names"),
    ("state", "State"),
    ("status", "Status"),
    ("cpu", ""),
    ("memory", ""),
    ("image", "Image"),
    ("ports", "Ports"),
    ("project", ""),
];
const COMPOSE: &[Column] = &[
    ("name", "Name"),
    ("status", "Status"),
    ("files", "ConfigFiles"),
];
const IMAGES: &[Column] = &[
    ("repository", "Repository"),
    ("tag", "Tag"),
    ("id", "ID"),
    ("size", "Size"),
    ("created", "CreatedSince"),
];
const VOLUMES: &[Column] = &[("name", "Name"), ("driver", "Driver")];
const NETWORKS: &[Column] = &[
    ("name", "Name"),
    ("driver", "Driver"),
    ("scope", "Scope"),
    ("id", "ID"),
];

impl Kind {
    /// Имя вкладки; под ним же правила колонок в `docker.lua`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Containers => "containers",
            Self::Compose => "compose",
            Self::Images => "images",
            Self::Volumes => "volumes",
            Self::Networks => "networks",
        }
    }

    fn fields(self) -> &'static [Column] {
        match self {
            Self::Containers => CONTAINERS,
            Self::Compose => COMPOSE,
            Self::Images => IMAGES,
            Self::Volumes => VOLUMES,
            Self::Networks => NETWORKS,
        }
    }

    pub(crate) fn columns(self) -> Vec<String> {
        self.fields().iter().map(|f| f.0.to_owned()).collect()
    }

    /// Аргументы `docker`, возвращающие список.
    pub(crate) fn list_args(self) -> Vec<String> {
        let args: &[&str] = match self {
            Self::Containers => &["ps", "--all", "--format", "{{json .}}"],
            Self::Compose => &["compose", "ls", "--all", "--format", "json"],
            Self::Images => &["image", "ls", "--format", "{{json .}}"],
            Self::Volumes => &["volume", "ls", "--format", "{{json .}}"],
            Self::Networks => &["network", "ls", "--format", "{{json .}}"],
        };
        args.iter().map(|a| (*a).to_owned()).collect()
    }

    /// Разбирает массив JSON или по объекту в строке.
    pub(crate) fn parse(self, output: &str) -> Vec<Item> {
        let records: Vec<Value> = match serde_json::from_str(output) {
            Ok(Value::Array(all)) => all,
            _ => output
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect(),
        };
        let mut items: Vec<Item> = records.iter().map(|r| self.item(r)).collect();
        // контейнеры одного проекта стоят рядом
        let project = self.columns().iter().position(|c| c == "project");
        if let Some(at) = project {
            items.sort_by(|a, b| (&a.cells[at], &a.name).cmp(&(&b.cells[at], &b.name)));
        }
        items
    }

    fn item(self, record: &Value) -> Item {
        let text = |key: &str| record[key].as_str().unwrap_or_default().to_owned();
        let cell = |&(column, key): &Column| match column {
            "project" => label(&text("Labels"), "com.docker.compose.project"),
            _ => text(key),
        };
        let cells = self.fields().iter().map(cell).collect();
        let (id, name) = self.identity(&text);
        let files = text("ConfigFiles");
        let files = files
            .split(',')
            .filter(|f| !f.is_empty())
            .map(str::to_owned);
        Item {
            id,
            name,
            cells,
            files: files.collect(),
        }
    }

    fn identity(self, text: &impl Fn(&str) -> String) -> (String, String) {
        match self {
            Self::Containers => (text("ID"), text("Names")),
            Self::Networks => (text("ID"), text("Name")),
            Self::Compose | Self::Volumes => (text("Name"), text("Name")),
            // по метке удаляется одна метка, по номеру — вся картинка
            Self::Images => {
                let (repository, tag) = (text("Repository"), text("Tag"));
                let name = format!("{repository}:{tag}");
                match repository == "<none>" || tag == "<none>" {
                    true => (text("ID"), name),
                    false => (name.clone(), name),
                }
            }
        }
    }
}

// значение метки из строки `k=v,k=v`
fn label(labels: &str, key: &str) -> String {
    let found = labels
        .split(',')
        .find_map(|pair| pair.strip_prefix(key)?.strip_prefix('='));
    found.unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containers_are_grouped_by_project() {
        let output = concat!(
            r#"{"ID":"2","Names":"zed","State":"running","Labels":""}"#,
            "\n",
            r#"{"ID":"1","Names":"api","State":"exited","Labels":"a=b,com.docker.compose.project=shop"}"#,
        );
        let items = Kind::Containers.parse(output);
        let names: Vec<_> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["zed", "api"]);
        assert_eq!(items[1].cells.last().map(String::as_str), Some("shop"));
    }

    #[test]
    fn compose_array_and_untagged_image() {
        let found = Kind::Compose.parse(r#"[{"Name":"shop","ConfigFiles":"/a.yml,/b.yml"}]"#);
        assert_eq!(found[0].files, ["/a.yml", "/b.yml"]);
        let image = Kind::Images.parse(r#"{"Repository":"<none>","Tag":"<none>","ID":"abc"}"#);
        assert_eq!(image[0].id, "abc");
    }
}
