//! Диагностика разрешений/возможностей системы. Ничего не меняет — только читает.
use serde::Serialize;

use crate::cleanup::sudo_noninteractive_ok;
use crate::util;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionInfo {
    pub id: String,
    pub title: String,
    pub why: String,
    pub needs_privilege: bool,
    pub status: String, // available | missing-tool | not-supported | needs-helper
    pub status_detail: String,
}

fn sudo_hint() -> &'static str {
    "нужен sudo без пароля либо helper"
}

pub(crate) fn checks() -> Vec<PermissionInfo> {
    let os = util::read_os_release();
    let arch_like = crate::network::is_arch_like(&os);
    let sudo_ok = sudo_noninteractive_ok();
    let resolved_present = std::path::Path::new("/run/systemd/resolve").exists() || std::path::Path::new("/etc/systemd/resolved.conf").exists();
    let home_writable = util::home_dir().join(".cache").exists() || std::fs::create_dir_all(util::home_dir().join(".cache")).is_ok();
    let xdg_runtime = std::env::var_os("XDG_RUNTIME_DIR").is_some();
    let systemctl = util::which("systemctl").is_some();

    let mut out = Vec::new();

    // 1) pacman: paccache
    {
        let (status, detail) = if util::which("paccache").is_none() {
            ("missing-tool", "paccache не найден (пакет pacman-contrib)".to_string())
        } else if !sudo_ok {
            ("needs-helper", sudo_hint().to_string())
        } else {
            ("available", "paccache + sudo -n: работает без интерактивного пароля".to_string())
        };
        out.push(PermissionInfo { id: "paccache".into(), title: "Очистка кэша pacman".into(), why: "Устаревшие версии пакетов в /var/cache/pacman/pkg можно удалить только с root-правами".into(), needs_privilege: true, status: status.into(), status_detail: detail });
    }

    // 2) DNS через systemd-resolved drop-in
    {
        let (status, detail) = if !resolved_present {
            ("not-supported", "systemd-resolved не обнаружен на этой машине".to_string())
        } else if !sudo_ok {
            ("needs-helper", format!("запись /etc/systemd/resolved.conf.d требует root: {}", sudo_hint()))
        } else {
            ("available", "drop-in + restart systemd-resolved через sudo -n".to_string())
        };
        out.push(PermissionInfo { id: "resolved-write".into(), title: "Применение DNS (systemd-resolved)".into(), why: "Резолвер читает конфигурацию с root-доступом; без него изменения не применятся".into(), needs_privilege: true, status: status.into(), status_detail: detail });
    }

    // 3) Зеркала pacman
    {
        let tool = ["cachyos-rate-mirrors", "rate-mirrors", "reflector"].iter().find(|t| util::which(t).is_some());
        let (status, detail) = if !arch_like {
            ("not-supported", "не arch-like дистрибутив".to_string())
        } else if tool.is_none() {
            ("missing-tool", "не найден ни cachyos-rate-mirrors, ни rate-mirrors, ни reflector".to_string())
        } else if !sudo_ok {
            ("needs-helper", format!("запись /etc/pacman.d/mirrorlist требует root: {}", sudo_hint()))
        } else {
            ("available", format!("{} + sudo tee", tool.unwrap()))
        };
        out.push(PermissionInfo { id: "mirrorlist".into(), title: "Применение зеркал pacman".into(), why: "mirrorlist читает только root; MeshPilot пишет его атомарно с бэкапом".into(), needs_privilege: true, status: status.into(), status_detail: detail });
    }

    // 4) journald / coredumps
    {
        let (status, detail) = if util::which("journalctl").is_none() {
            ("missing-tool", "journalctl не найден (systemd не установлен?)".to_string())
        } else if !sudo_ok {
            ("needs-helper", sudo_hint().to_string())
        } else {
            ("available", "journalctl --vacuum-size через sudo -n".to_string())
        };
        out.push(PermissionInfo { id: "journal".into(), title: "Ограничение журнала journald".into(), why: "Журнал хранится в /var/log/journal; vacuum выполняется от root".into(), needs_privilege: true, status: status.into(), status_detail: detail });
    }

    // 5) Открытие приложений Джарвисом (только allowlist, без shell)
    {
        let launcher = util::which("gio").map(|_| "gio").or_else(|| util::which("xdg-open").map(|_| "xdg-open"));
        let (status, detail) = match launcher {
            Some(l) => ("available", format!("запуск через {l}, без shell и без повышения прав")),
            None => ("missing-tool", "не найден ни gio, ни xdg-open".to_string()),
        };
        out.push(PermissionInfo { id: "open-apps".into(), title: "Открытие приложений Джарвисом".into(), why: "Запуск только из allowlist идентификаторов desktop-entry".into(), needs_privilege: false, status: status.into(), status_detail: detail });
    }

    // 6) Пользовательские кэши
    {
        let (status, detail) = if home_writable {
            ("available", "всё в $HOME, владелец — пользователь".to_string())
        } else {
            ("not-supported", "$HOME недоступен для записи?".to_string())
        };
        out.push(PermissionInfo { id: "user-caches".into(), title: "Очистка пользовательских кэшей".into(), why: "Файлы в ~/.cache и подобных каталогах принадлежат пользователю".into(), needs_privilege: false, status: status.into(), status_detail: detail });
    }

    // 7) byedpi / zapret
    {
        let ciadpi = util::which("ciadpi");
        let tpws = util::which("tpws");
        let (status, detail) = match (&ciadpi, &tpws) {
            (Some(_), _) => ("available", "ciadpi найден — тесты стратегий и user-сервис доступны".to_string()),
            (None, Some(_)) => ("available", "zapret (tpws) найден; автотест стратегий поддерживает только byedpi".to_string()),
            _ => ("missing-tool", "не найдены ни ciadpi (byedpi), ни tpws (zapret)".to_string()),
        };
        out.push(PermissionInfo { id: "byedpi".into(), title: "Обход блокировок (loopback)".into(), why: "Локальный прокси на 127.0.0.1; трафик никуда не отправляется без вашего включения".into(), needs_privilege: false, status: status.into(), status_detail: detail });
    }

    // 8) Расписания (user systemd)
    {
        let (status, detail) = if !systemctl {
            ("missing-tool", "systemctl не найден".to_string())
        } else if !xdg_runtime {
            ("not-supported", "XDG_RUNTIME_DIR не задан — user systemd недоступен в этом сеансе".to_string())
        } else {
            ("available", "systemd --user: таймеры без root, только отчёты".to_string())
        };
        out.push(PermissionInfo { id: "systemd-user".into(), title: "Расписания (user systemd)".into(), why: "Таймеры пользователя выполняют read-only отчёты MeshPilot".into(), needs_privilege: false, status: status.into(), status_detail: detail });
    }

    out
}
