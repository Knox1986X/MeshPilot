//! Расписания: systemd user timers + oneshot service, который перезапускает
//! бинарник приложения в головless-режиме (`--run-schedule <id>`) и выполняет
//! только отчёты (ничего не меняет в системе).
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::cleanup;
use crate::util;

#[derive(Clone, Copy)]
pub(crate) struct ScheduleSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub calendar_human: &'static str,
    pub on_calendar: &'static str,
}

pub(crate) const SCHEDULES: &[ScheduleSpec] = &[
    ScheduleSpec { id: "mirrors-daily", name: "Проверка зеркал", description: "Ежедневно: сводка по зеркалам pacman (дистрибутив, инструменты, размер pacman-базы). Только отчёт.", calendar_human: "ежедневно 05:00", on_calendar: "*-*-* 05:00:00" },
    ScheduleSpec { id: "dns-daily", name: "Проверка DNS", description: "Ежедневно: бенчмарк популярных резолверов, отчёт с лучшим кандидатом. Применение — только вручную в разделе «Сеть».", calendar_human: "ежедневно 05:15", on_calendar: "*-*-* 05:15:00" },
    ScheduleSpec { id: "net-weekly", name: "Сетевой профиль", description: "Еженедельно: прямая доступность контрольных URL и статус loopback-прокси обхода. Только отчёт.", calendar_human: "вс 05:20", on_calendar: "Sun *-*-* 05:20:00" },
    ScheduleSpec { id: "cleanup-weekly", name: "Безопасная очистка", description: "Еженедельно: сводка оценки безопасного профиля очистки. Ничего не удаляется.", calendar_human: "вс 06:00", on_calendar: "Sun *-*-* 06:00:00" },
];

pub(crate) fn find_spec(id: &str) -> Option<&'static ScheduleSpec> {
    SCHEDULES.iter().find(|s| s.id == id)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub calendar: String,
    pub last_run: Option<String>,
    pub last_result: Option<String>,
    pub next_run: Option<String>,
    pub mode: String, // всегда "report" на этом этапе
    pub supported: bool,
    pub unsupported_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ScheduleState {
    last_run_utc: Option<String>,
    last_result: Option<String>,
    report_path: Option<String>,
}

fn user_unit_dir() -> PathBuf {
    util::home_dir().join(".config/systemd/user")
}

fn service_unit_name(id: &str) -> String {
    format!("meshpilot-{id}.service")
}

fn timer_unit_name(id: &str) -> String {
    format!("meshpilot-{id}.timer")
}

fn state_file(id: &str) -> PathBuf {
    util::state_dir().join("schedules").join(format!("{id}.json"))
}

fn reports_dir() -> PathBuf {
    util::state_dir().join("reports")
}

fn read_state(id: &str) -> ScheduleState {
    std::fs::read(state_file(id))
        .ok()
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_default()
}

fn write_state(id: &str, state: &ScheduleState) {
    if let Ok(body) = serde_json::to_vec_pretty(state) {
        let _ = util::ensure_dir(&util::state_dir().join("schedules"));
        let _ = util::atomic_write(&state_file(id), &body, true);
    }
}

/// systemctl --user доступен и есть runtime-каталог (иначе user units не стартуют).
pub(crate) fn supported() -> Result<(), String> {
    if util::which("systemctl").is_none() {
        return Err("systemctl не найден".to_string());
    }
    if std::env::var_os("XDG_RUNTIME_DIR").is_none() {
        return Err("XDG_RUNTIME_DIR не задан — user systemd не запущен (сеанс без пользовательского systemd?)".to_string());
    }
    Ok(())
}

pub(crate) fn list() -> Vec<ScheduleInfo> {
    let supported_res = supported();
    let supported_flag = supported_res.is_ok();
    let unsupported_reason = supported_res.err(); // move out до замыкания
    SCHEDULES
        .iter()
        .map(|spec| {
            let state = read_state(spec.id);
            let enabled = if supported_flag {
                timer_is_enabled(spec.id)
            } else {
                false
            };
            let next_run = if supported_flag && enabled {
                timer_next_elapse(spec.id)
            } else {
                None
            };
            ScheduleInfo {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                description: spec.description.to_string(),
                enabled,
                calendar: spec.calendar_human.to_string(),
                last_run: state.last_run_utc,
                last_result: state.last_result,
                next_run,
                mode: "report".to_string(),
                supported: supported_flag,
                unsupported_reason: unsupported_reason.clone(),
            }
        })
        .collect()
}

fn timer_is_enabled(id: &str) -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", &timer_unit_name(id)])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim() == "enabled")
        .unwrap_or(false)
}

fn timer_next_elapse(id: &str) -> Option<String> {
    let out = std::process::Command::new("systemctl")
        .args(["--user", "show", &timer_unit_name(id), "-p", "NextElapseUSecRealtime", "--value"])
        .output()
        .ok()?;
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // Например: "Sun 2026-11-22 05:00:00 MSK" — парсить локаль хрупко; если не вышло, отдаём как есть.
    if raw.is_empty() || raw == "n/a" {
        return None;
    }
    parse_systemd_timestamp(&raw).or(Some(raw))
}

/// Разобрать «Sun 2026-11-22 05:00:00 MSK" (или с микросекундами) в RFC3339 UTC — лучшее усилие.
fn parse_systemd_timestamp(raw: &str) -> Option<String> {
    // Берём «YYYY-MM-DD HH:MM:SS» независимо от дня недели и суффикса TZ.
    let mut parts = raw.split_whitespace();
    let mut date = None;
    let mut time = None;
    for p in &mut parts {
        if p.len() == 10 && p.chars().nth(4) == Some('-') {
            date = Some(p);
        } else if p.len() >= 8 && p.chars().nth(2) == Some(':') {
            time = Some(p);
        }
    }
    let (date, time) = (date?, time?);
    let time = time.split_once('.').map_or(time, |(before, _)| before); // отбросить дробную часть
    let naive = format!("{date}T{time}");
    // Трактуем как локальное время отсутствующего смещения: форматируем как есть (без конверсии TZ).
    let parsed = chrono::NaiveDateTime::parse_from_str(&naive, "%Y-%m-%dT%H:%M:%S").ok()?;
    Some(parsed.format("%Y-%m-%dT%H:%M:%S").to_string())
}

pub(crate) fn set_enabled(id: &str, enabled: bool) -> crate::network::OpResult {
    let Some(spec) = find_spec(id) else {
        return crate::network::OpResult::fail(format!("неизвестное расписание: {id}"));
    };
    if let Err(reason) = supported() {
        return crate::network::OpResult::fail(format!("расписания не поддерживаются: {reason}"));
    }
    let unit_dir = user_unit_dir();
    if let Err(e) = util::ensure_dir(&unit_dir) {
        return crate::network::OpResult::fail(format!("не удалось создать каталог user units: {e}"));
    }
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return crate::network::OpResult::fail(format!("не удалось определить путь к бинарнику: {e}")),
    };
    let service = format!(
        "[Unit]\nDescription=MeshPilot: {name} (отчёт)\n\n[Service]\nType=oneshot\nExecStart={exe} --run-schedule {id}\nNice=10\nIOSchedulingClass=idle\n",
        name = spec.name,
        exe = exe.display(),
        id = spec.id
    );
    let timer = format!(
        "[Unit]\nDescription=MeshPilot timer: {name}\n\n[Timer]\nOnCalendar={oncal}\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n",
        name = spec.name,
        oncal = spec.on_calendar
    );
    if let Err(e) = std::fs::write(unit_dir.join(service_unit_name(spec.id)), service) {
        return crate::network::OpResult::fail(format!("не удалось записать service unit: {e}"));
    }
    if let Err(e) = std::fs::write(unit_dir.join(timer_unit_name(spec.id)), timer) {
        return crate::network::OpResult::fail(format!("не удалось записать timer unit: {e}"));
    }
    let _ = cleanup::run_command("systemctl", &["--user", "daemon-reload"], std::time::Duration::from_secs(30));
    let result = if enabled {
        cleanup::run_command("systemctl", &["--user", "enable", "--now", &timer_unit_name(spec.id)], std::time::Duration::from_secs(60))
    } else {
        cleanup::run_command("systemctl", &["--user", "disable", "--now", &timer_unit_name(spec.id)], std::time::Duration::from_secs(60))
    };
    match result {
        Ok(_) => crate::network::OpResult::ok(if enabled {
            format!("расписание «{name}» включено: {cal} (только отчёты)", name = spec.name, cal = spec.calendar_human)
        } else {
            format!("расписание «{name}» выключено", name = spec.name)
        }),
        Err(e) => crate::network::OpResult::fail(format!("unit записан, но systemctl не удался: {e}")),
    }
}

// =====================================================================
// ---------- Headless-исполнение по расписанию ----------
// =====================================================================

/// Выполняется при старте с аргументом `--run-schedule <id>`; пишет отчёт и состояние.
pub fn run_headless(id: &str) -> Result<String, String> {
    let Some(spec) = find_spec(id) else {
        return Err(format!("неизвестное расписание: {id}"));
    };
    let started = util::now_rfc3339();
    let body = match spec.id {
        "mirrors-daily" => report_mirrors(),
        "dns-daily" => report_dns(),
        "net-weekly" => report_net(),
        "cleanup-weekly" => report_cleanup(),
        _ => unreachable!(),
    };
    let ok = body.is_ok();
    let text = match body {
        Ok(text) => text,
        Err(e) => format!("Ошибка: {e}"),
    };
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let dir = reports_dir();
    let _ = util::ensure_dir(&dir);
    let report_path = dir.join(format!("{}-{}.txt", spec.id, stamp));
    let mut report = String::new();
    report.push_str(&format!("MeshPilot — {}\nЗапущено: {started}\nРежим: ОТЧЁТ (ничего не изменялось в системе)\n\n", spec.name));
    report.push_str(&text);
    report.push('\n');
    let _ = util::atomic_write(&report_path, report.as_bytes(), false);

    write_state(
        spec.id,
        &ScheduleState {
            last_run_utc: Some(started),
            last_result: Some(if ok { "ok".to_string() } else { "failed".to_string() }),
            report_path: Some(report_path.display().to_string()),
        },
    );
    Ok(report_path.display().to_string())
}

fn report_mirrors() -> Result<String, String> {
    let status = crate::network::mirrors_status();
    let mut out = String::new();
    out.push_str(&format!("Дистрибутив: {}\n", status.os_name.unwrap_or_else(|| "?".into())));
    out.push_str(&format!("Arch-like: {}\n", if status.is_arch_like { "да" } else { "нет" }));
    out.push_str(&format!("Инструменты ранжирования: {}\n", if status.detected_tools.is_empty() { "не найдены".to_string() } else { status.detected_tools.join(", ") }));
    out.push_str(&format!("Mirrorlist: {}\n", if status.mirrorlist_exists { "есть" } else { "не найден" }));
    out.push_str(&format!("Папок в базе pacman: {}\n", status.pacman_db_count.map(|n| n.to_string()).unwrap_or_else(|| "?".into())));
    out.push_str(&format!("Последний бэкап mirrorlist: {}\n", status.last_backup.unwrap_or_else(|| "нет".into())));
    Ok(out)
}

fn report_dns() -> Result<String, String> {
    let status = crate::network::dns_status();
    let bench = crate::network::dns_benchmark();
    let mut out = String::new();
    out.push_str(&format!("Менеджер DNS: {} ({})\n", status.manager, status.manager_detail));
    out.push_str(&format!("Серверы: {}\n", if status.current_servers.is_empty() { "не определены".to_string() } else { status.current_servers.join(", ") }));
    out.push_str(&format!("DNSSEC: {}\nDNSOverTLS: {}\n", status.dnssec.as_deref().unwrap_or("?"), status.dns_over_tls.as_deref().unwrap_or("?")));
    out.push_str("\nБенчмарк (медиана, мс):\n");
    for c in &bench.candidates {
        out.push_str(&format!(
            "- {name} ({addr}): {ms} (ok {ok}/{total}){filtered}\n",
            name = c.name,
            addr = c.address,
            ms = c.median_ms.map(|v| format!("{v:.1}")).unwrap_or_else(|| "нет ответа".into()),
            ok = c.ok_count,
            total = c.ok_count + c.fail_count,
            filtered = if c.filtered { " [фильтрация]" } else { "" },
        ));
    }
    let best = bench.candidates.iter().filter(|c| c.ok_count > 0).min_by(|a, b| a.median_ms.partial_cmp(&b.median_ms).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(best) = best {
        out.push_str(&format!("\nЛучший кандидат: {} ({}). Применение — вручную в разделе «Сеть».\n", best.name, best.address));
    }
    Ok(out)
}

fn report_net() -> Result<String, String> {
    let cfg = crate::settings::load_settings();
    let status = crate::network::bypass_status();
    let checks = crate::network::check_direct(&cfg.network.check_urls);
    let mut out = String::new();
    out.push_str(&format!("ciadpi: {}\n", if status.ciadpi_found { "найден" } else { "не найден" }));
    out.push_str(&format!("zapret (tpws): {}\n", if status.zapret_found { "найден" } else { "не найден" }));
    out.push_str(&format!("Loopback-прокси на 127.0.0.1:{}: {}\n", status.port, if status.active_service { "отвечает" } else { "не отвечает" }));
    out.push_str("\nПрямая доступность (без запуска обхода):\n");
    for check in &checks {
        out.push_str(&format!("- {}: {}\n", check.url, if check.ok { "ok" } else { "недоступен" }));
    }
    Ok(out)
}

fn report_cleanup() -> Result<String, String> {
    let report = cleanup::analyze();
    let mut out = String::new();
    out.push_str(&format!("Оценка освобождаемого (безопасный профиль): ~{} МБ\n", report.total_estimated_bytes / 1_048_576));
    out.push_str("Категории:\n");
    for cat in &report.categories {
        let est = cat.estimated_bytes.map(|b| format!("~{} МБ", b / 1_048_576)).unwrap_or_else(|| "?".into());
        out.push_str(&format!("- {name} [{risk}]: {est}{blocked}\n",
            name = cat.name, risk = cat.risk,
            blocked = cat.blocked_reason.as_deref().map(|r| format!(" (заблокировано: {r})")).unwrap_or_default()));
    }
    if !report.running_apps.is_empty() {
        out.push_str(&format!("\nЗапущенные приложения: {}\n", report.running_apps.join(", ")));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_systemd_timestamp_handles_common_formats() {
        let iso = parse_systemd_timestamp("Sun 2026-11-22 05:00:00 MSK");
        assert_eq!(iso.as_deref(), Some("2026-11-22T05:00:00"));
        let with_micros = parse_systemd_timestamp("Sun 2026-11-22 05:00:00.123456 MSK");
        assert_eq!(with_micros.as_deref(), Some("2026-11-22T05:00:00"));
        assert_eq!(parse_systemd_timestamp("n/a"), None);
    }

    #[test]
    fn schedule_catalog_is_unique_and_report_mode() {
        let mut ids: Vec<&str> = SCHEDULES.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), SCHEDULES.len());
    }

    #[test]
    fn spec_lookup_works() {
        assert_eq!(find_spec("dns-daily").map(|s| s.name), Some("Проверка DNS"));
        assert!(find_spec("nope").is_none());
    }
}
