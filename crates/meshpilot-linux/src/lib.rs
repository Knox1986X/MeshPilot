//! Пассивные пробы фиксированных источников Linux. Внешние программы не запускаются.
mod cpu;
mod interval;
mod network_io;
mod ram;
mod root_fs;
mod uptime;
pub use cpu::cpu_snapshot;
pub use network_io::network_io_snapshot;
pub use ram::ram_snapshot;
pub use root_fs::root_fs_snapshot;
pub use uptime::uptime_snapshot;

use meshpilot_core::{Check, DoctorReport, Finding, SCHEMA_VERSION, StatusReport};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
    time::SystemTime,
};

// Целые счётчики должны оставаться точными после передачи JSON в WebView.
const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;

// os-release и маркер дистрибутива — данные. Не source, без расширения переменных.
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let mut values = text.lines().filter_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k == key).then_some(v)
    });
    let raw = values.next()?;
    if values.next().is_some() {
        return None;
    }
    if raw.starts_with('"') {
        raw.strip_prefix('"')?.strip_suffix('"')
    } else if raw.starts_with('\'') {
        raw.strip_prefix('\'')?.strip_suffix('\'')
    } else {
        Some(raw)
    }
}

fn parse_id(text: &str) -> Option<String> {
    let id = field(text, "ID")?;
    (!id.is_empty()
        && id.len() <= 64
        && id.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        }))
    .then(|| id.to_owned())
}

fn bounded_text(file: impl Read) -> Option<String> {
    let mut text = String::new();
    file.take(65537).read_to_string(&mut text).ok()?;
    (text.len() <= 65536).then_some(text)
}

fn utc_stamp(time: SystemTime) -> Option<String> {
    let millis = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_millis();
    let stamp = jiff::Timestamp::from_millisecond(millis.try_into().ok()?).ok()?;
    Some(format!("{stamp:.3}"))
}

struct Platform {
    id: Option<String>,
    cachyos: Option<bool>,
}

fn identify(release: &str, marker: impl FnOnce() -> Option<String>) -> Platform {
    let id = parse_id(release);
    let cachyos = match id.as_deref() {
        Some("cachyos") => Some(true),
        Some("arch") if field(release, "NAME") == Some("CachyOS") => {
            // Известная разметка Live ISO. Один branding или ID_LIKE недостаточен.
            marker()
                .as_deref()
                .and_then(|m| (field(m, "DISTRIB_ID") == Some("CachyOS")).then_some(true))
        }
        Some(_) => Some(false),
        None => None,
    };
    Platform { id, cachyos }
}

fn platform() -> Platform {
    let release = File::open("/etc/os-release")
        .or_else(|e| match e.kind() {
            io::ErrorKind::NotFound => File::open("/usr/lib/os-release"),
            _ => Err(e),
        })
        .ok()
        .and_then(bounded_text);
    identify(release.as_deref().unwrap_or_default(), || {
        File::open("/etc/cachyos-release")
            .ok()
            .and_then(bounded_text)
    })
}

pub fn status() -> StatusReport {
    let platform = platform();
    let (cpu, network_io) =
        interval::pair::<cpu::Counters, network_io::Counters>(std::thread::sleep);
    StatusReport {
        cpu,
        network_io,
        ram: ram_snapshot(),
        root_fs: root_fs_snapshot(),
        uptime: uptime_snapshot(),
        ..StatusReport::unmeasured(platform.id, platform.cachyos)
    }
}

// На CachyOS используются системные пути Arch. Это проверка файла, не dlopen/ldd.
enum FileRole {
    Utility,
    DesktopLibrary,
}

fn file_finding(path: &Path, role: FileRole) -> Finding {
    match fs::metadata(path) {
        Ok(meta) if !meta.is_file() => Finding::Unsupported,
        Ok(_) if matches!(role, FileRole::DesktopLibrary) => Finding::Detected,
        #[cfg(unix)]
        Ok(meta) => {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o111 != 0 {
                Finding::Detected
            } else {
                Finding::Unsupported
            }
        }
        #[cfg(not(unix))]
        Ok(_) => Finding::Unsupported,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Finding::Missing,
        Err(_) => Finding::Unknown,
    }
}

pub fn doctor() -> DoctorReport {
    let platform = platform();
    let mut checks = vec![Check {
        id: "os",
        required: true,
        finding: match platform.cachyos {
            Some(true) => Finding::Detected,
            Some(false) => Finding::Unsupported,
            None => Finding::Unknown,
        },
        detail: match (platform.id.as_deref(), platform.cachyos) {
            (Some("arch"), Some(true)) => {
                "CachyOS: ID=arch, NAME=CachyOS и маркер /etc/cachyos-release совпали".into()
            }
            (Some("arch"), None) => {
                "ID=arch и NAME=CachyOS; маркер дистрибутива не подтверждён".into()
            }
            (Some(id), _) => id.into(),
            (None, _) => "не удалось определить ID системы".into(),
        },
    }];
    for (id, path, required) in [
        ("pacman", "/usr/bin/pacman", true),
        ("flatpak", "/usr/bin/flatpak", false),
    ] {
        checks.push(Check {
            id,
            required,
            finding: file_finding(Path::new(path), FileRole::Utility),
            detail: path.into(),
        });
    }
    if platform.cachyos == Some(true) {
        for (id, path) in [
            ("desktop-webkitgtk", "/usr/lib/libwebkit2gtk-4.1.so.0"),
            (
                "desktop-javascriptcoregtk",
                "/usr/lib/libjavascriptcoregtk-4.1.so.0",
            ),
        ] {
            checks.push(Check {
                id,
                required: false, // Только для окна, CLI работает без WebView.
                finding: file_finding(Path::new(path), FileRole::DesktopLibrary),
                detail: format!("для окна: {path}; проверяется наличие файла, не запуск"),
            });
        }
    }
    DoctorReport {
        schema_version: SCHEMA_VERSION,
        read_only: true,
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_release_is_data_and_id_like_is_not_the_os() {
        assert_eq!(
            parse_id("ID_LIKE=arch\nID=\"cachyos\""),
            Some("cachyos".into())
        );
        assert_eq!(parse_id("ID_LIKE=cachyos"), None);
        assert_eq!(parse_id("ID=$(hostname)"), None);
        assert_eq!(parse_id("ID=\"cachyos"), None);
    }

    #[test]
    fn official_live_layout_keeps_arch_id_and_requires_both_signals() {
        let live = "NAME=\"CachyOS\"\nID=arch\nBUILD_ID=rolling\n";
        let ok = identify(live, || Some("DISTRIB_ID=CachyOS\n".into()));
        assert_eq!(ok.id.as_deref(), Some("arch"));
        assert_eq!(ok.cachyos, Some(true));
        for marker in [
            None,
            Some("DISTRIB_ID=Arch"),
            Some("DISTRIB_ID=CachyOS\nDISTRIB_ID=other"),
        ] {
            assert_eq!(identify(live, || marker.map(str::to_owned)).cachyos, None);
        }
    }

    #[test]
    fn other_systems_are_not_promoted_by_id_like_or_a_leftover_marker() {
        for release in [
            "ID=arch\nNAME=Arch Linux",
            "NAME=CachyOS\nID=debian\nID_LIKE=cachyos",
        ] {
            assert_eq!(
                identify(release, || panic!("маркер не нужен")).cachyos,
                Some(false)
            );
        }
        assert_eq!(identify("ID_LIKE=cachyos", || None).cachyos, None);
        assert_eq!(identify("ID=cachyos\nID=arch", || None).cachyos, None);
        assert_eq!(
            identify("ID=cachyos", || panic!("маркер не нужен")).cachyos,
            Some(true)
        );
    }
}
