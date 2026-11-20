use meshpilot_core::Finding;
use std::{env, process::ExitCode};

const USAGE: &str = "MeshPilot · диагностика без изменений\n\nmeshpilot <status|doctor> [--json] [--dry-run]\n\n  status   Состояние системы и доступность наблюдений\n  doctor   Пассивная проверка CachyOS и системных утилит\n\nКоды: 0 — отчёт готов, 2 — ошибка аргументов, 3 — ограниченная диагностика.";

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
                    "MeshPilot · только чтение\nСистема: {}\nИнтернет и DNS: не проверено; профиль не выбран, изменений для отката нет в этом этапе.\nОбновления программ: не проверено.\nИсправления безопасности: не проверено.\nПоследняя успешная проверка: —\nОбновить всё: недоступно; установка не запускается.",
                    r.os_id.as_deref().unwrap_or("не удалось определить")
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
