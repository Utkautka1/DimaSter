//! Общая загрузка машины из `/proc`: её показывает монитор, пока в терминале
//! ничего не запущено.

use dimaster_i18n::tr;

/// Такты процессора с загрузки: всего и без простоя.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ticks {
    total: u64,
    busy: u64,
}

// первая строка `/proc/stat`: user nice system idle iowait irq softirq steal
fn parse_stat(stat: &str) -> Option<Ticks> {
    let line = stat.lines().next()?.strip_prefix("cpu ")?;
    // guest уже входит в user, поэтому берутся только первые восемь
    let numbers = line.split_whitespace().take(8).map(str::parse);
    let values: Vec<u64> = numbers.collect::<Result<_, _>>().ok()?;
    let total: u64 = values.iter().sum();
    let idle = values.get(3)? + values.get(4)?;
    Some(Ticks {
        total,
        busy: total - idle,
    })
}

// занято и всего, в КиБ; свободным считается и то, что ядро отдаст по запросу
fn parse_memory(meminfo: &str) -> Option<(u64, u64)> {
    let field = |name: &str| {
        let line = meminfo.lines().find_map(|l| l.strip_prefix(name))?;
        line.split_whitespace().next()?.parse::<u64>().ok()
    };
    let total = field("MemTotal:")?;
    Some((total.saturating_sub(field("MemAvailable:")?), total))
}

// доля занятых тактов между двумя замерами, в процентах
fn load(before: Ticks, now: Ticks) -> Option<u64> {
    let total = now.total.checked_sub(before.total).filter(|&t| t > 0)?;
    Some(now.busy.saturating_sub(before.busy) * 100 / total)
}

fn memory_text((used, total): (u64, u64)) -> String {
    let gigabytes = |kib: u64| kib as f64 / (1024.0 * 1024.0);
    format!(
        "{:.1}/{:.1} {}",
        gigabytes(used),
        gigabytes(total),
        tr("ГБ")
    )
}

/// Добавляет `system_cpu` и `system_memory`; CPU — со второго замера.
pub(crate) fn sample(previous: &mut Option<Ticks>, facts: &mut Vec<(String, String)>) {
    let read = |path| std::fs::read_to_string(path).ok();
    let now = read("/proc/stat").and_then(|text| parse_stat(&text));
    if let Some(cpu) = previous.zip(now).and_then(|(b, n)| load(b, n)) {
        facts.push(("system_cpu".into(), cpu.to_string()));
    }
    *previous = now;
    if let Some(memory) = read("/proc/meminfo").and_then(|text| parse_memory(&text)) {
        facts.push(("system_memory".into(), memory_text(memory)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_is_counted_between_samples() {
        let before = parse_stat("cpu  10 0 10 70 10 0 0 0 5 0\ncpu0 1").expect("stat");
        let after = parse_stat("cpu  40 0 30 110 20 0 0 0 5 0").expect("stat");
        assert_eq!(load(before, after), Some(50));
        assert_eq!(load(after, after), None);
        let memory = "MemTotal: 8000 kB\nMemFree: 10 kB\nMemAvailable: 6000 kB\n";
        assert_eq!(parse_memory(memory), Some((2000, 8000)));
    }
}
