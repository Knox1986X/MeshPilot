//! Оптимизатор CachyOS: чистка, зеркала, DNS, Zapret2, система.
//! Вызывает pacman/paccache/reflector/systemctl напрямую (без Python).
use std::process::Command;

fn run(cmd: &str, args: &[&str], sudo: bool) -> String {
    let out = if sudo {
        Command::new("sudo").arg("-n").arg(cmd).args(args).output()
    } else {
        Command::new(cmd).args(args).output()
    };
    match out {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            if s.trim().is_empty() { "ok".to_string() } else { s.trim().to_string() }
        }
        Err(e) => format!("Ошибка: {e}"),
    }
}

#[tauri::command]
pub fn clean_pacman_cache(dry_run: bool) -> String {
    if dry_run {
        return run("bash", &["-c", "du -sh /var/cache/pacman/pkg 2>/dev/null; echo 'Dry-run: paccache -rk2'"], false);
    }
    run("paccache", &["-rk", "2"], true)
}

#[tauri::command]
pub fn remove_orphans(dry_run: bool) -> String {
    let orphans = run("pacman", &["-Qdtq"], false);
    if orphans.trim().is_empty() || orphans.starts_with("Ошибка") {
        return "Orphan-пакетов нет".to_string();
    }
    if dry_run {
        return format!("Dry-run: orphan-пакеты:\n{orphans}");
    }
    let list: Vec<&str> = orphans.lines().collect();
    let mut args = vec!["-Rns", "--noconfirm"];
    args.extend(list);
    run("pacman", &args, true)
}

#[tauri::command]
pub fn vacuum_journal(dry_run: bool) -> String {
    if dry_run {
        return run("journalctl", &["--disk-usage"], false);
    }
    run("journalctl", &["--vacuum-time=7d"], true)
}

#[tauri::command]
pub fn clean_all(dry_run: bool) -> String {
    let mut out = String::new();
    out.push_str(&format!("=== pacman cache ===\n{}\n", clean_pacman_cache(dry_run)));
    out.push_str(&format!("=== orphans ===\n{}\n", remove_orphans(dry_run)));
    out.push_str(&format!("=== journal ===\n{}\n", vacuum_journal(dry_run)));
    out.push_str(&format!("=== flatpak ===\n{}\n", run("flatpak", &["uninstall", "--unused", "-y"], false)));
    out.push_str(&format!("=== docker ===\n{}\n", run("docker", &["system", "prune", "-af", "-f"], false)));
    out
}

#[tauri::command]
pub fn optimize_mirrors(dry_run: bool) -> String {
    if dry_run {
        return "Dry-run: rate-mirrors/reflector + бенчмарк зеркал".to_string();
    }
    // rate-mirrors если есть
    let r = run("rate-mirrors", &["--allow-root", "--save", "/tmp/meshpilot-mirrors", "arch", "cachyos"], false);
    if r.contains("ok") || !r.starts_with("Ошибка") {
        return run("bash", &["-c", "sudo cp /tmp/meshpilot-mirrors /etc/pacman.d/mirrorlist && cat /etc/pacman.d/mirrorlist"], false);
    }
    run("reflector", &["--country", "Russia,Germany", "--protocol", "https", "--latest", "20", "--sort", "rate", "--save", "/tmp/meshpilot-mirrors"], false)
}

#[tauri::command]
pub fn optimize_dns(dry_run: bool) -> String {
    if dry_run {
        return "Dry-run: бенчмарк DNS (Cloudflare/Google/AdGuard/Yandex) + systemd-resolved".to_string();
    }
    run("bash", &["-c", "echo '[Resolve]\\nDNS=1.1.1.1 8.8.8.8\\nFallbackDNS=9.9.9.9' | sudo tee /etc/systemd/resolved.conf.d/meshpilot.conf && sudo systemctl restart systemd-resolved"], false)
}

#[tauri::command]
pub fn zapret_status() -> String {
    let installed = std::path::Path::new("/opt/zapret2").exists();
    let active = run("systemctl", &["is-active", "zapret2"], false);
    format!("Zapret2: installed={installed} active={active}")
}

#[tauri::command]
pub fn zapret_action(action: String) -> String {
    match action.as_str() {
        "start" => run("systemctl", &["start", "zapret2"], true),
        "stop" => run("systemctl", &["stop", "zapret2"], true),
        "restart" => run("systemctl", &["restart", "zapret2"], true),
        "logs" => run("journalctl", &["-u", "zapret2", "-n", "50", "--no-pager"], false),
        _ => format!("Неизвестное действие: {action}"),
    }
}

#[tauri::command]
pub fn system_health() -> String {
    let mut out = String::new();
    out.push_str(&run("bash", &["-c", "df -h / | tail -1; free -h | head -2; uptime"], false));
    out.push_str(&format!("\n{}\n", run("journalctl", &["--disk-usage"], false)));
    out
}
