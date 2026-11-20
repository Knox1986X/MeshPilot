//! Пассивные пробы фиксированных системных файлов. Внешние программы не запускаются.
use meshpilot_core::{Check, DoctorReport, Finding, SCHEMA_VERSION, StatusReport};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

fn parse_id(text: &str) -> Option<String> {
    let raw = text.lines().find_map(|line| line.strip_prefix("ID="))?;
    let id = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| raw.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(raw);
    (!id.is_empty()
        && id.len() <= 64
        && id.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        }))
    .then(|| id.to_owned())
}

fn os_id() -> Option<String> {
    let file = File::open("/etc/os-release")
        .or_else(|e| match e.kind() {
            io::ErrorKind::NotFound => File::open("/usr/lib/os-release"),
            _ => Err(e),
        })
        .ok()?;
    let mut text = String::new();
    file.take(65537).read_to_string(&mut text).ok()?;
    (text.len() <= 65536).then(|| parse_id(&text)).flatten()
}

pub fn status() -> StatusReport {
    StatusReport::unmeasured(os_id())
}

fn executable(path: &Path) -> Finding {
    match fs::metadata(path) {
        #[cfg(unix)]
        Ok(meta) => {
            use std::os::unix::fs::PermissionsExt;
            if meta.is_file() && meta.permissions().mode() & 0o111 != 0 {
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
    let id = os_id();
    let mut checks = vec![Check {
        id: "os",
        required: true,
        finding: match id.as_deref() {
            Some("cachyos") => Finding::Detected,
            Some(_) => Finding::Unsupported,
            None => Finding::Unknown,
        },
        detail: id.unwrap_or_else(|| "не удалось определить ID системы".into()),
    }];
    for (id, path, required) in [
        ("pacman", "/usr/bin/pacman", true),
        ("flatpak", "/usr/bin/flatpak", false),
    ] {
        checks.push(Check {
            id,
            required,
            finding: executable(Path::new(path)),
            detail: path.into(),
        });
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
}
