use meshpilot_core::{CpuObservation, Finding, RamObservation};
use std::{env, process::ExitCode};

const USAGE: &str = "MeshPilot · диагностика без изменений\n\nmeshpilot <status|doctor> [--json] [--dry-run]\n\n  status   Состояние системы и доступность наблюдений\n  doctor   Пассивная проверка ОС, утилит и файлов WebKitGTK на CachyOS\n\nКоды: 0 — отчёт готов, 2 — ошибка аргументов, 3 — ограниченная диагностика.";

fn cpu_line(cpu: Option<&CpuObservation>) -> String {
    match cpu {
        Some(c) => format!(
            "CPU: {:.1} % за {} мс · {} · {}",
            c.usage_percent, c.interval_ms, c.source, c.observed_at_utc
        ),
        None => "CPU: нет данных".into(),
    }
}

fn ram_line(ram: Option<&RamObservation>) -> String {
    match ram {
        Some(r) => format!(
            "RAM: занято {} из {} байт · {} · {}",
            r.used_bytes, r.total_bytes, r.source, r.observed_at_utc
        ),
        None => "RAM: нет данных".into(),
    }
}

fn main() -> ExitCode {
    let mut command = None;
    let mut json = false;
    for arg in env::args_os().skip(1) {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            Some("--version" | "-V") => {
                println!("meshpilot {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            Some("--json") => json = true,
            Some("--dry-run") => {} // Обе команды всегда только для чтения.
            Some(c @ ("status" | "doctor")) if command.is_none() => command = Some(c.to_owned()),
            _ => {
                eprintln!("Неизвестный аргумент или лишняя команда.\n{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    match command.as_deref() {
        Some("status") => {
            let r = meshpilot_linux::status();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&r).expect("serializable report")
                );
            } else {
                println!(
                    "MeshPilot · только чтение\nСистема: {}\n{}\n{}\nИнтернет и DNS: не проверено; профиль не выбран, изменений для отката нет в этом этапе.\nОбновления программ: не проверено.\nИсправления безопасности: не проверено.\nПоследняя успешная проверка: —\nОбновить всё: недоступно; установка не запускается.",
                    match (r.cachyos, r.os_id.as_deref()) {
                        (Some(true), Some("arch")) => "CachyOS (ID=arch)",
                        (None, Some("arch")) => "arch (CachyOS не подтверждена)",
                        (_, id) => id.unwrap_or("не удалось определить"),
                    },
                    cpu_line(r.cpu.as_ref()),
                    ram_line(r.ram.as_ref())
                );
            }
            ExitCode::SUCCESS
        }
        Some("doctor") => {
            let r = meshpilot_linux::doctor();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&r).expect("serializable report")
                );
            } else {
                println!("MeshPilot · диагностика без запуска менеджеров пакетов");
                for c in &r.checks {
                    let label = match c.finding {
                        Finding::Detected => "обнаружено",
                        Finding::Missing => "не найдено",
                        Finding::Unknown => "не удалось проверить",
                        Finding::Unsupported => "не поддерживается",
                    };
                    println!("{}: {label} ({})", c.id, c.detail);
                }
            }
            if r.ready() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(3)
            }
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_text_keeps_interval_source_time_and_measured_zero() {
        let cpu = CpuObservation {
            usage_percent: 0.0,
            interval_ms: 637,
            busy_ticks: 0,
            total_ticks: 81,
            source: "/proc/stat",
            observed_at_utc: "2026-11-20T10:00:00.123Z".into(),
        };
        assert_eq!(
            cpu_line(Some(&cpu)),
            "CPU: 0.0 % за 637 мс · /proc/stat · 2026-11-20T10:00:00.123Z"
        );
        assert_eq!(cpu_line(None), "CPU: нет данных");
    }

    #[test]
    fn text_distinguishes_measured_zero_from_unavailable() {
        let r = RamObservation {
            used_bytes: 0,
            total_bytes: 1024,
            source: "/proc/meminfo",
            observed_at_utc: "2026-11-20T10:00:00.123Z".into(),
        };
        assert_eq!(
            ram_line(Some(&r)),
            "RAM: занято 0 из 1024 байт · /proc/meminfo · 2026-11-20T10:00:00.123Z"
        );
        assert_eq!(ram_line(None), "RAM: нет данных");
    }
}
