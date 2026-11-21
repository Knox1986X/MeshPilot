//! Сеть: зеркала pacman, DNS, обход блокировок (byedpi/zapret).
//! Все изменения — только через явный confirm от UI, с бэкапом. Никакого скрытого применения.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use crate::cleanup::{run_command, run_sudo, sudo_noninteractive_ok};
use crate::settings;
use crate::util;

// =====================================================================
// ---------- DTO (camelCase) ----------
// =====================================================================

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MirrorsStatus {
    pub os_name: Option<String>,
    pub is_arch_like: bool,
    pub detected_tools: Vec<String>,
    pub mirrorlist_path: Option<String>,
    pub mirrorlist_exists: bool,
    pub last_backup: Option<String>,
    pub pacman_db_count: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
}

impl OpResult {
    pub(crate) fn ok(message: impl Into<String>) -> Self {
        Self { ok: true, message: message.into(), backup_path: None }
    }
    pub(crate) fn ok_with_backup(message: impl Into<String>, backup: String) -> Self {
        Self { ok: true, message: message.into(), backup_path: Some(backup) }
    }
    pub(crate) fn fail(message: impl Into<String>) -> Self {
        Self { ok: false, message: message.into(), backup_path: None }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsStatus {
    pub manager: String, // networkmanager | systemd-resolved | resolv-conf | unknown
    pub manager_detail: String,
    pub current_servers: Vec<String>,
    pub dnssec: Option<String>,
    pub dns_over_tls: Option<String>,
    pub provider_hint: Option<String>,
    pub last_backup: Option<String>,
    pub needs_privilege: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DnsCandidate {
    pub name: String,
    pub address: String,
    pub tls_name: Option<String>,
    pub median_ms: Option<f64>,
    pub ok_count: u32,
    pub fail_count: u32,
    pub filtered: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsBenchmarkResult {
    pub candidates: Vec<DnsCandidate>,
    pub tested_at_utc: String,
    pub domains: Vec<String>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BypassStatus {
    pub ciadpi_found: bool,
    pub zapret_found: bool,
    pub active_service: bool,
    pub port: u16,
    pub check_urls: Vec<String>,
    pub last_profile: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BypassStrategyResult {
    pub strategy: Vec<String>,
    pub label: String,
    pub passed: u32,
    pub total: u32,
    pub works: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BypassTestResult {
    pub direct: Vec<UrlCheck>,
    pub strategies: Vec<BypassStrategyResult>,
    pub tested_at_utc: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlCheck {
    pub url: String,
    pub ok: bool,
}

// =====================================================================
// ---------- Зеркала ----------
// =====================================================================

const MIRRORLIST: &str = "/etc/pacman.d/mirrorlist";

pub(crate) fn is_arch_like(os_release: &std::collections::HashMap<String, String>) -> bool {
    let id = os_release.get("ID").map(|s| s.to_lowercase()).unwrap_or_default();
    let like = os_release.get("ID_LIKE").map(|s| s.to_lowercase()).unwrap_or_default();
    id.contains("arch") || like.split_whitespace().any(|w| w == "arch")
}

fn newest_backup(prefix: &str) -> Option<String> {
    let dir = util::backup_dir();
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(prefix) {
            continue;
        }
        if let Some(modified) = entry.metadata().ok().and_then(|m| m.modified().ok()) {
            let dominated = best.as_ref().map(|(t, _)| modified > *t).unwrap_or(true);
            if dominated {
                best = Some((modified, entry.path()));
            }
        }
    }
    best.map(|(_, p)| p.display().to_string())
}

pub(crate) fn mirrors_status() -> MirrorsStatus {
    let os = util::read_os_release();
    let arch_like = is_arch_like(&os);
    let tools: Vec<String> = ["cachyos-rate-mirrors", "rate-mirrors", "reflector"]
        .iter()
        .filter(|t| util::which(t).is_some())
        .map(|t| t.to_string())
        .collect();
    let mirrorlist_exists = PathBuf::from(MIRRORLIST).exists();
    let pacman_db_count = std::fs::read_dir("/var/lib/pacman/local")
        .ok()
        .map(|entries| entries.flatten().filter(|e| e.file_name().to_string_lossy().chars().next().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false)).count() as u64);
    MirrorsStatus {
        os_name: os.get("PRETTY_NAME").cloned(),
        is_arch_like: arch_like,
        detected_tools: tools,
        mirrorlist_path: Some(MIRRORLIST.to_string()),
        mirrorlist_exists,
        last_backup: newest_backup("mirrorlist-"),
        pacman_db_count,
    }
}

pub(crate) fn mirrors_apply() -> OpResult {
    let status = mirrors_status();
    if !status.is_arch_like {
        return OpResult::fail("дистрибутив не arch-like — зеркала pacman не применяются");
    }
    if !status.mirrorlist_exists {
        return OpResult::fail("mirrorlist не найден: /etc/pacman.d/mirrorlist");
    }
    // 1) Бэкап текущего mirrorlist.
    let backup_dir = util::backup_dir();
    if let Err(e) = util::ensure_dir(&backup_dir) {
        return OpResult::fail(format!("не удалось создать каталог бэкапов: {e}"));
    }
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_path = backup_dir.join(format!("mirrorlist-{stamp}"));
    match std::fs::read(MIRRORLIST) {
        Ok(bytes) => {
            if let Err(e) = util::atomic_write(&backup_path, &bytes, false) {
                return OpResult::fail(format!("не удалось сохранить бэкап mirrorlist: {e}"));
            }
        }
        Err(e) => return OpResult::fail(format!("не удалось прочитать mirrorlist (права?): {e}")),
    }

    if !sudo_noninteractive_ok() {
        return OpResult::fail("нужен sudo без пароля либо helper (см. «Разрешения»). Бэкап сохранён.");
    }

    let os_id = util::read_os_release().get("ID").cloned().unwrap_or_default();

    // 2) Ранжируем и записываем через выбранный инструмент.
    let write_via_tee = |content: &str| -> Result<(), String> {
        // printf | sudo tee — без shell, через pipe stdin.
        let mut child = std::process::Command::new("sudo")
            .args(["-n", "tee", MIRRORLIST])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("sudo tee не запустился: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(content.as_bytes());
        }
        let out = child.wait_with_output().map_err(|e| format!("sudo tee: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            let lossy = String::from_utf8_lossy(&out.stderr);
            let mut rev: Vec<&str> = lossy.lines().rev().take(2).collect();
            rev.reverse();
            let tail = rev.join(" / ");
            Err(format!("sudo tee завершился с ошибкой: {tail}"))
        }
    };

    let validate_servers = |body: &str| -> bool {
        body.lines().filter(|l| l.trim_start().starts_with("Server")).count() >= 3
    };

    let tool = status.detected_tools.first().cloned();
    let result: Result<String, String> = match tool.as_deref() {
        Some("cachyos-rate-mirrors") => {
            // CachyOS: инструмент сам записывает /etc/pacman.d/mirrorlist.
            run_sudo(&["cachyos-rate-mirrors"], Duration::from_secs(600)).map(|_| "cachyos-rate-mirrors".to_string())
        }
        Some("rate-mirrors") => {
            let distro = if os_id.contains("cachyos") { "cachyos" } else { "arch" };
            run_command("rate-mirrors", &[distro], Duration::from_secs(600)).and_then(|out| {
                if !validate_servers(&out) {
                    return Err("rate-mirrors вернул меньше 3 серверов — запись отменена".to_string());
                }
                write_via_tee(&out)?;
                Ok("rate-mirrors".to_string())
            })
        }
        Some("reflector") => {
            run_command("reflector", &["--protocol", "https", "--latest", "30", "--sort", "rate"], Duration::from_secs(600)).and_then(|out| {
                if !validate_servers(&out) {
                    return Err("reflector вернул меньше 3 серверов — запись отменена".to_string());
                }
                write_via_tee(&out)?;
                Ok("reflector".to_string())
            })
        }
        _ => Err("не найден ни cachyos-rate-mirrors, ни rate-mirrors, ни reflector".to_string()),
    };

    match result {
        Ok(used) => OpResult::ok_with_backup(format!("зеркала обновлены ({used}). Для применения: sudo pacman -Syyu"), backup_path.display().to_string()),
        Err(e) => OpResult::fail(format!("{e}. Бэкап сохранён: {}", backup_path.display())),
    }
}

// =====================================================================
// ---------- DNS ----------
// =====================================================================

const RESOLVED_DROPIN: &str = "/etc/systemd/resolved.conf.d/50-meshpilot.conf";

fn known_provider_hint(servers: &[String]) -> Option<String> {
    let mut hints: Vec<&'static str> = Vec::new();
    for s in servers {
        match s.as_str() {
            "1.1.1.1" | "1.0.0.1" | "2606:4700:4700::1111" | "2606:4700:4700::1001" => hints.push("Cloudflare"),
            "9.9.9.9" | "149.112.112.112" => hints.push("Quad9"),
            "8.8.8.8" | "8.8.4.4" => hints.push("Google"),
            "94.140.14.14" | "94.140.15.15" | "94.140.14.15" | "2a10:50c0::ad1:ff" => hints.push("AdGuard"),
            "208.67.222.222" | "208.67.220.220" => hints.push("OpenDNS"),
            "77.88.8.8" | "77.88.8.1" => hints.push("Yandex"),
            _ => {}
        }
    }
    hints.sort();
    hints.dedup();
    if hints.is_empty() { None } else { Some(hints.join(" / ")) }
}

fn detect_dns_manager() -> (String, String, Vec<String>, Option<String>, Option<String>) {
    // systemd-resolved?
    let resolved_present = std::path::Path::new("/run/systemd/resolve").exists() || std::path::Path::new("/etc/systemd/resolved.conf").exists();
    if resolved_present {
        if let Ok(out) = run_command("resolvectl", &["status", "--no-pager"], Duration::from_secs(10)) {
            let servers = parse_resolvectl_servers(&out);
            let dnssec = parse_kv_line(&out, "DNSSEC");
            let dot = parse_kv_line(&out, "DNSOverTLS");
            return ("systemd-resolved".into(), "resolvectl status".into(), servers, dnssec, dot);
        }
        return ("systemd-resolved".into(), "resolvectl недоступен".into(), vec![], None, None);
    }
    // NetworkManager?
    if util::which("nmcli").is_some() {
        if let Ok(out) = run_command("nmcli", &["-t", "-f", "IP4.DNS,IP6.DNS", "device", "show"], Duration::from_secs(10)) {
            let servers: Vec<String> = out
                .lines()
                .filter_map(|l| l.split_once(':'))
                .map(|(_, v)| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect();
            return ("networkmanager".into(), "nmcli device show".into(), servers, None, None);
        }
        return ("networkmanager".into(), "nmcli не отвечает".into(), vec![], None, None);
    }
    // /etc/resolv.conf напрямую?
    if let Ok(raw) = std::fs::read_to_string("/etc/resolv.conf") {
        let servers: Vec<String> = raw
            .lines()
            .filter_map(|l| l.trim().strip_prefix("nameserver"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let detail = if std::fs::symlink_metadata("/etc/resolv.conf").map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            "resolv.conf (символическая ссылка)".to_string()
        } else {
            "resolv.conf".to_string()
        };
        return ("resolv-conf".into(), detail, servers, None, None);
    }
    ("unknown".into(), "не удалось определить менеджер DNS".into(), vec![], None, None)
}

fn parse_resolvectl_servers(out: &str) -> Vec<String> {
    let mut servers = Vec::new();
    for line in out.lines() {
        let trimmed = line.trim();
        if let Some((_, rest)) = trimmed.split_once("DNS Servers:").or_else(|| trimmed.split_once("Current DNS Server:")) {
            for token in rest.split_whitespace() {
                if token.parse::<std::net::IpAddr>().is_ok() {
                    servers.push(token.to_string());
                }
            }
        }
    }
    servers.sort();
    servers.dedup();
    servers
}

fn parse_kv_line(out: &str, key: &str) -> Option<String> {
    for line in out.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let value = rest.trim_start_matches([':', ' ']).trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

pub(crate) fn dns_status() -> DnsStatus {
    let (manager, detail, servers, dnssec, dot) = detect_dns_manager();
    DnsStatus {
        manager,
        manager_detail: detail,
        current_servers: servers.clone(),
        dnssec,
        dns_over_tls: dot,
        provider_hint: known_provider_hint(&servers),
        last_backup: newest_backup("dns-"),
        needs_privilege: true,
    }
}

// ---------- Бенчмарк: сырой UDP DNS ----------

const BENCH_DOMAINS: [&str; 3] = ["google.com", "wikipedia.org", "github.com"];
const BENCH_CANDIDATES: [(&str, &str, Option<&str>, bool); 4] = [
    ("Cloudflare", "1.1.1.1", Some("cloudflare-dns.com"), false),
    ("Quad9", "9.9.9.9", Some("dns.quad9.net"), true),
    ("Google", "8.8.8.8", Some("dns.google"), false),
    ("AdGuard", "94.140.14.14", Some("dns.adguard-dns.com"), true),
];

fn build_dns_query(id: u16, name: &str) -> Vec<u8> {
    let mut q = Vec::with_capacity(name.len() + 18);
    q.extend_from_slice(&id.to_be_bytes());
    q.extend_from_slice(&[0x01, 0x00]); // RD=1
    q.extend_from_slice(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    for label in name.split('.') {
        let bytes = label.as_bytes();
        q.push(bytes.len() as u8);
        q.extend_from_slice(bytes);
    }
    q.push(0);
    q.extend_from_slice(&[0x00, 0x01]); // A
    q.extend_from_slice(&[0x00, 0x01]); // IN
    q
}

fn udp_dns_rtt_ms(server: &str, qname: &str, id: u16, timeout: Duration) -> Option<f64> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.set_read_timeout(Some(timeout)).ok()?;
    socket.set_write_timeout(Some(timeout)).ok()?;
    let query = build_dns_query(id, qname);
    let started = std::time::Instant::now();
    socket.send_to(&query, format!("{server}:53")).ok()?;
    let mut buf = [0u8; 512];
    let (len, _) = socket.recv_from(&mut buf).ok()?;
    if len < 12 {
        return None;
    }
    if u16::from_be_bytes([buf[0], buf[1]]) != id {
        return None;
    }
    let rcode = buf[3] & 0x0f;
    if rcode > 3 {
        return None; // FORMERR/SERVFAIL — считаем неудачей
    }
    Some(started.elapsed().as_secs_f64() * 1000.0)
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some((values[mid - 1] + values[mid]) / 2.0)
    }
}

pub(crate) fn dns_benchmark() -> DnsBenchmarkResult {
    let mut candidates = Vec::with_capacity(BENCH_CANDIDATES.len());
    let mut qid: u16 = (std::process::id() & 0x7fff) as u16 ^ 0x4d50;
    for (name, address, tls_name, filtered) in BENCH_CANDIDATES {
        let mut samples = Vec::new();
        let mut ok_count = 0u32;
        let mut fail_count = 0u32;
        for domain in BENCH_DOMAINS {
            for _ in 0..2 {
                qid = qid.wrapping_add(1);
                match udp_dns_rtt_ms(address, domain, qid, Duration::from_millis(1200)) {
                    Some(ms) => {
                        ok_count += 1;
                        samples.push(ms);
                    }
                    None => fail_count += 1,
                }
            }
        }
        candidates.push(DnsCandidate {
            name: name.to_string(),
            address: address.to_string(),
            tls_name: tls_name.map(|s| s.to_string()),
            median_ms: median(samples),
            ok_count,
            fail_count,
            filtered,
        });
    }
    let tested = DnsBenchmarkResult {
        candidates,
        tested_at_utc: util::now_rfc3339(),
        domains: BENCH_DOMAINS.iter().map(|s| s.to_string()).collect(),
        note: None,
    };
    // Сохраняем для dns_apply (решение «лучший из проверенных»).
    if let Ok(body) = serde_json::to_vec_pretty(&tested) {
        let _ = util::ensure_dir(&util::state_dir());
        let _ = util::atomic_write(&util::state_dir().join("dns-benchmark.json"), &body, true);
    }
    tested
}

fn last_benchmark() -> Option<DnsBenchmarkResult> {
    let raw = std::fs::read(util::state_dir().join("dns-benchmark.json")).ok()?;
    serde_json::from_slice(&raw).ok()
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DnsBackup {
    manager: String,
    created_at_utc: String,
    target: String,               // drop-in или resolv.conf
    previous_existed: bool,
    previous_content: Option<String>, // только для resolv.conf
}

pub(crate) fn dns_apply() -> OpResult {
    // 1) Выбор кандидата: лучший (минимальный median) из последнего сохранённого бенчмарка.
    let bench = match last_benchmark() {
        Some(b) => b,
        None => return OpResult::fail("нет сохранённого бенчмарка — сначала выполните «Проверить DNS»"),
    };
    let best = bench
        .candidates
        .iter()
        .filter(|c| c.ok_count > 0 && c.median_ms.is_some())
        .min_by(|a, b| a.median_ms.partial_cmp(&b.median_ms).unwrap_or(std::cmp::Ordering::Equal))
        .cloned();
    let Some(best) = best else {
        return OpResult::fail("в последнем бенчмарке нет ни одного отвечающего кандидата");
    };

    let (manager, ..) = detect_dns_manager();
    let backup_dir = util::backup_dir();
    if let Err(e) = util::ensure_dir(&backup_dir) {
        return OpResult::fail(format!("не удалось создать каталог бэкапов: {e}"));
    }
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_path = backup_dir.join(format!("dns-{stamp}.json"));

    if manager == "systemd-resolved" {
        let dropin_existed = std::path::Path::new(RESOLVED_DROPIN).exists();
        let backup = DnsBackup { manager: manager.clone(), created_at_utc: util::now_rfc3339(), target: RESOLVED_DROPIN.into(), previous_existed: dropin_existed, previous_content: None };
        if let Ok(body) = serde_json::to_vec_pretty(&backup) {
            let _ = util::atomic_write(&backup_path, &body, false);
        }
        if !sudo_noninteractive_ok() {
            return OpResult::fail("нужен sudo без пароля либо helper (см. «Разрешения»)");
        }
        let content = format!("# Создано MeshPilot {stamp}\n[Resolve]\nDNS={}\n", best.address);
        if let Err(e) = sudo_tee(RESOLVED_DROPIN, content.as_bytes()) {
            return OpResult::fail(format!("не удалось записать {RESOLVED_DROPIN}: {e}"));
        }
        if let Err(e) = run_sudo(&["systemctl", "restart", "systemd-resolved"], Duration::from_secs(60)) {
            // Восстанавливаем прежнее состояние при неудаче рестарта.
            let _ = run_sudo(&["rm", "-f", RESOLVED_DROPIN], Duration::from_secs(30));
            if dropin_existed {
                let _ = run_sudo(&["systemctl", "restart", "systemd-resolved"], Duration::from_secs(60));
            }
            return OpResult::fail(format!("рестарт systemd-resolved не удался: {e}. Изменения отменены."));
        }
        return OpResult::ok_with_backup(format!("DNS применён: {} ({}), systemd-resolved перезапущен", best.address, best.name), backup_path.display().to_string());
    }

    if manager == "resolv-conf" {
        let resolv = std::path::Path::new("/etc/resolv.conf");
        let is_symlink = std::fs::symlink_metadata(resolv).map(|m| m.file_type().is_symlink()).unwrap_or(false);
        if is_symlink {
            return OpResult::fail("/etc/resolv.conf — символическая ссылка (обычно управляется systemd-resolved или NetworkManager), запись отклонена");
        }
        let previous = std::fs::read_to_string(resolv).ok();
        let backup = DnsBackup { manager: manager.clone(), created_at_utc: util::now_rfc3339(), target: "/etc/resolv.conf".into(), previous_existed: previous.is_some(), previous_content: previous.clone() };
        if let Ok(body) = serde_json::to_vec_pretty(&backup) {
            let _ = util::atomic_write(&backup_path, &body, false);
        }
        if !sudo_noninteractive_ok() {
            return OpResult::fail("нужен sudo без пароля либо helper (см. «Разрешения»)");
        }
        let content = format!("# Создано MeshPilot {stamp}\nnameserver {}\n", best.address);
        if let Err(e) = sudo_tee("/etc/resolv.conf", content.as_bytes()) {
            return OpResult::fail(format!("не удалось записать /etc/resolv.conf: {e}"));
        }
        return OpResult::ok_with_backup(format!("DNS применён: {} ({}) в /etc/resolv.conf", best.address, best.name), backup_path.display().to_string());
    }

    OpResult::fail(format!("применение DNS для менеджера «{manager}» в этой версии не поддержано (поддерживаются systemd-resolved и статический /etc/resolv.conf). Используйте отчёт бенчмарка и примените вручную."))
}

pub(crate) fn dns_rollback() -> OpResult {
    let Some(backup_path_str) = newest_backup("dns-") else {
        return OpResult::fail("нет резервной копии DNS — откат невозможен");
    };
    let raw = match std::fs::read(&backup_path_str) {
        Ok(r) => r,
        Err(e) => return OpResult::fail(format!("не удалось прочитать бэкап: {e}")),
    };
    let backup: DnsBackup = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return OpResult::fail(format!("бэкап повреждён: {e}")),
    };
    if !sudo_noninteractive_ok() {
        return OpResult::fail("нужен sudo без пароля либо helper (см. «Разрешения»)");
    }
    if backup.target == RESOLVED_DROPIN {
        if let Err(e) = run_sudo(&["rm", "-f", RESOLVED_DROPIN], Duration::from_secs(30)) {
            return OpResult::fail(format!("не удалось удалить drop-in: {e}"));
        }
        if let Err(e) = run_sudo(&["systemctl", "restart", "systemd-resolved"], Duration::from_secs(60)) {
            return OpResult::fail(format!("drop-in удалён, но рестарт systemd-resolved не удался: {e}"));
        }
        return OpResult::ok("откат выполнен: drop-in MeshPilot удалён, systemd-resolved перезапущен");
    }
    if backup.target == "/etc/resolv.conf" {
        match backup.previous_content {
            Some(content) => {
                if let Err(e) = sudo_tee("/etc/resolv.conf", content.as_bytes()) {
                    return OpResult::fail(format!("не удалось восстановить /etc/resolv.conf: {e}"));
                }
                return OpResult::ok("откат выполнен: /etc/resolv.conf восстановлен из бэкапа");
            }
            None => return OpResult::fail("бэкап не содержит прежнего содержимого /etc/resolv.conf — восстановите вручную"),
        }
    }
    OpResult::fail(format!("неизвестный тип бэкапа: {}", backup.target))
}

fn sudo_tee(path: &str, content: &[u8]) -> Result<(), String> {
    let mut child = std::process::Command::new("sudo")
        .args(["-n", "tee", path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("sudo tee не запустился: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(content);
    }
    let out = child.wait_with_output().map_err(|e| format!("sudo tee: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let lossy = String::from_utf8_lossy(&out.stderr);
        let mut rev: Vec<&str> = lossy.lines().rev().take(2).collect();
        rev.reverse();
        let tail = rev.join(" / ");
        Err(format!("sudo tee: {tail}"))
    }
}

// =====================================================================
// ---------- Обход (byedpi/zapret) — loopback-only ----------
// =====================================================================

const TEST_PORT_BASE: u16 = 19000;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BypassProfile {
    strategy_label: String,
    args: Vec<String>,
    saved_at_utc: String,
}

fn bypass_profile_path() -> PathBuf {
    util::state_dir().join("bypass-profile.json")
}

fn read_bypass_profile() -> Option<BypassProfile> {
    let raw = std::fs::read(bypass_profile_path()).ok()?;
    serde_json::from_slice(&raw).ok()
}

pub(crate) fn bypass_status() -> BypassStatus {
    let cfg = settings::load_settings();
    let port = cfg.network.bypass_port;
    let ciadpi_found = util::which("ciadpi").is_some();
    let zapret_found = util::which("tpws").is_some();
    // Активность проверяем соединением на loopback — честнее, чем «unit существует».
    let active_service = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(300),
    )
    .is_ok();
    BypassStatus {
        ciadpi_found,
        zapret_found,
        active_service,
        port,
        check_urls: cfg.network.check_urls,
        last_profile: read_bypass_profile().map(|p| p.strategy_label),
    }
}

const STRATEGIES: &[(&str, &[&str])] = &[
    ("split @1", &["--dpi-desync=split", "--dpi-desync-split-pos=1"]),
    ("split midsld", &["--dpi-desync=split", "--dpi-desync-split-pos=midsld"]),
    ("disorder @1", &["--dpi-desync=disorder", "--dpi-desync-split-pos=1"]),
    ("fake ttl=1", &["--dpi-desync=fake", "--dpi-desync-ttl=1"]),
    ("fake+disorder midsld", &["--dpi-desync=fake,disorder", "--dpi-desync-split-pos=midsld"]),
    ("oob @1", &["--dpi-desync=oob", "--dpi-desync-split-pos=1"]),
];

/// Публичная для schedules/report: только прямые проверки, без запуска стратегий обхода.
pub(crate) fn check_direct(urls: &[String]) -> Vec<UrlCheck> {
    direct_check(urls)
}

fn direct_check(urls: &[String]) -> Vec<UrlCheck> {
    let client = match reqwest::blocking::Client::builder().timeout(Duration::from_secs(8)).redirect(reqwest::redirect::Policy::limited(2)).build() {
        Ok(c) => c,
        Err(_) => return urls.iter().map(|u| UrlCheck { url: u.clone(), ok: false }).collect(),
    };
    urls.iter()
        .map(|url| UrlCheck { url: url.clone(), ok: http_ok(&client, url) })
        .collect()
}

fn proxy_check(client_proxy: &reqwest::blocking::Client, url: &str) -> bool {
    http_ok(client_proxy, url)
}

fn http_ok(client: &reqwest::blocking::Client, url: &str) -> bool {
    match client.get(url).send() {
        Ok(resp) => resp.status().as_u16() < 400,
        Err(_) => false,
    }
}

pub(crate) fn bypass_test(urls: &[String]) -> BypassTestResult {
    let urls: Vec<String> = urls.iter().take(10).map(|u| u.chars().take(300).collect()).collect();
    let direct = direct_check(&urls);

    let ciadpi = util::which("ciadpi");
    let mut strategies = Vec::new();

    if let Some(ciadpi_path) = ciadpi {
        let total = urls.len() as u32;
        for (label, args) in STRATEGIES {
            // Тестовый порт: базовый + случайное смещение, проверяем свободность.
            let test_port = pick_test_port();
            let mut full_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            full_args.push("--port".to_string());
            full_args.push(test_port.to_string());
            full_args.push("--listen-addr".to_string());
            full_args.push("127.0.0.1".to_string());
            let mut child = match std::process::Command::new(&ciadpi_path)
                .args(&full_args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(_) => {
                    strategies.push(BypassStrategyResult { strategy: args.iter().map(|s| s.to_string()).collect(), label: label.to_string(), passed: 0, total, works: false });
                    continue;
                }
            };
            std::thread::sleep(Duration::from_millis(350));
            let passed = match reqwest::Proxy::http(format!("http://127.0.0.1:{test_port}")) {
                Ok(proxy) => match reqwest::blocking::Client::builder().proxy(proxy).timeout(Duration::from_secs(6)).redirect(reqwest::redirect::Policy::limited(2)).build() {
                    Ok(client) => urls.iter().filter(|u| proxy_check(&client, u)).count() as u32,
                    Err(_) => 0,
                },
                Err(_) => 0,
            };
            let _ = child.kill();
            let _ = child.wait();
            let works = passed >= 2 || (total == 1 && passed == 1);
            strategies.push(BypassStrategyResult { strategy: args.iter().map(|s| s.to_string()).collect(), label: label.to_string(), passed, total, works });
        }
        // Сохраняем лучшую рабочую стратегию для bypass_enable.
        if let Some(best) = strategies.iter().find(|s| s.works) {
            let profile = BypassProfile { strategy_label: best.label.clone(), args: best.strategy.clone(), saved_at_utc: util::now_rfc3339() };
            if let Ok(body) = serde_json::to_vec_pretty(&profile) {
                let _ = util::ensure_dir(&util::state_dir());
                let _ = util::atomic_write(&bypass_profile_path(), &body, true);
            }
        }
    }

    BypassTestResult {
        direct,
        strategies,
        tested_at_utc: util::now_rfc3339(),
    }
}

fn pick_test_port() -> u16 {
    for offset in 0..500u16 {
        let candidate = TEST_PORT_BASE.wrapping_add((std::process::id() as u16) % 997 + offset);
        if std::net::TcpListener::bind(("127.0.0.1", candidate)).is_ok() {
            return candidate;
        }
    }
    TEST_PORT_BASE
}

fn user_systemd_dir() -> PathBuf {
    util::home_dir().join(".config/systemd/user")
}

const UNIT_NAME: &str = "meshpilot-byedpi.service";

pub(crate) fn bypass_enable() -> OpResult {
    let ciadpi = match util::which("ciadpi") {
        Some(p) => p,
        None => return OpResult::fail("ciadpi не найден (пакет byedpi). Установите: sudo pacman -S byedpi"),
    };
    let cfg = settings::load_settings();
    let port = cfg.network.bypass_port;
    let profile = read_bypass_profile();
    let (args, source) = match &profile {
        Some(p) => (p.args.clone(), format!("стратегия «{}» из последнего теста", p.strategy_label)),
        None => (
            vec!["--dpi-desync=split".to_string(), "--dpi-desync-split-pos=midsld".to_string()],
            "стратегия по умолчанию (split midsld); запустите «Проверить» для подбора".to_string(),
        ),
    };
    let unit_dir = user_systemd_dir();
    if let Err(e) = util::ensure_dir(&unit_dir) {
        return OpResult::fail(format!("не удалось создать каталог user units: {e}"));
    }
    let unit = format!(
        "[Unit]\nDescription=MeshPilot byedpi (loopback-only)\nAfter=network.target\n\n[Service]\nExecStart={} {} --port {} --listen-addr 127.0.0.1\nRestart=on-failure\nRestartSec=2\n\n[Install]\nWantedBy=default.target\n",
        ciadpi.display(),
        args.join(" "),
        port
    );
    if let Err(e) = std::fs::write(unit_dir.join(UNIT_NAME), unit) {
        return OpResult::fail(format!("не удалось записать unit: {e}"));
    }
    let _ = run_command("systemctl", &["--user", "daemon-reload"], Duration::from_secs(30));
    match run_command("systemctl", &["--user", "enable", "--now", UNIT_NAME], Duration::from_secs(60)) {
        Ok(_) => OpResult::ok(format!("byedpi запущен на 127.0.0.1:{port} ({source})")),
        Err(e) => OpResult::fail(format!("unit записан, но systemctl --user enable --now не удался: {e}")),
    }
}

pub(crate) fn bypass_disable() -> OpResult {
    let unit_path = user_systemd_dir().join(UNIT_NAME);
    let _ = run_command("systemctl", &["--user", "disable", "--now", UNIT_NAME], Duration::from_secs(60));
    let removed = if unit_path.exists() {
        std::fs::remove_file(&unit_path).is_ok()
    } else {
        true
    };
    let _ = run_command("systemctl", &["--user", "daemon-reload"], Duration::from_secs(30));
    if removed {
        OpResult::ok("byedpi остановлен и отключён; unit удалён")
    } else {
        OpResult::fail("byedpi остановлен, но unit-файл удалить не удалось")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_detection() {
        let mut os = std::collections::HashMap::new();
        os.insert("ID".to_string(), "cachyos".to_string());
        os.insert("ID_LIKE".to_string(), "arch".to_string());
        assert!(is_arch_like(&os));
        os.insert("ID".to_string(), "ubuntu".to_string());
        os.insert("ID_LIKE".to_string(), "debian".to_string());
        assert!(!is_arch_like(&os));
    }

    #[test]
    fn dns_query_header_is_well_formed() {
        let q = build_dns_query(0xABCD, "example.com");
        assert_eq!(&q[..2], &[0xAB, 0xCD]);
        assert_eq!(q[2] & 0x01, 0x01); // RD
        assert_eq!(u16::from_be_bytes([q[4], q[5]]), 1); // QDCOUNT=1
        assert!(q.windows("example".len()).any(|w| w == b"example"));
        assert_eq!(q.len(), 12 + 8 + 4 + 1 + 4); // header + метка example + метка com + корневой 0 + qtype/qclass
    }

    #[test]
    fn median_of_samples() {
        assert_eq!(median(vec![]), None);
        assert_eq!(median(vec![5.0]), Some(5.0));
        assert_eq!(median(vec![3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(vec![4.0, 1.0, 2.0, 3.0]), Some(2.5));
    }

    #[test]
    fn provider_hint_matches_known_ips() {
        assert_eq!(known_provider_hint(&["1.1.1.1".into()]).as_deref(), Some("Cloudflare"));
        assert_eq!(known_provider_hint(&["9.9.9.9".into(), "8.8.8.8".into()]).as_deref(), Some("Google / Quad9"));
        assert_eq!(known_provider_hint(&["203.0.113.9".into()]), None);
    }

    #[test]
    fn resolvectl_parse_servers() {
        let out = "Global\n       Protocols: -LLMNR +mDNS -DNSOverTLS DNSSEC=allow-downgrade\nresolv.conf mode: stub\n\nLink 2 (wlan0)\n    Current DNS Server: 192.168.1.1\n       DNS Servers: 192.168.1.1 1.1.1.1\n        DNS Domain: lan\n";
        let servers = parse_resolvectl_servers(out);
        assert!(servers.contains(&"192.168.1.1".to_string()));
        assert!(servers.contains(&"1.1.1.1".to_string()));
    }

    #[test]
    fn kv_line_parse() {
        let out = "         DNSSEC: yes\n    DNSOverTLS: opportunistic\n";
        assert_eq!(parse_kv_line(out, "DNSSEC").as_deref(), Some("yes"));
        assert_eq!(parse_kv_line(out, "DNSOverTLS").as_deref(), Some("opportunistic"));
        assert_eq!(parse_kv_line(out, "Nope"), None);
    }

    #[test]
    fn validate_server_lines() {
        assert!("# comment\nServer = https://a/\nServer = https://b/\nServer = https://c/\n".lines().filter(|l| l.trim_start().starts_with("Server")).count() >= 3);
    }
}
