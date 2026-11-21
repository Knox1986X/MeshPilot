use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn xdg_dir(env_key: &str, fallback_rel: &str) -> PathBuf {
    if let Some(v) = std::env::var_os(env_key) {
        let p = PathBuf::from(v);
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    home_dir().join(fallback_rel)
}

pub fn config_dir() -> PathBuf {
    xdg_dir("XDG_CONFIG_HOME", ".config").join("meshpilot")
}

pub fn state_dir() -> PathBuf {
    xdg_dir("XDG_STATE_HOME", ".local/state").join("meshpilot")
}

pub fn data_dir() -> PathBuf {
    xdg_dir("XDG_DATA_HOME", ".local/share").join("meshpilot")
}

pub fn backup_dir() -> PathBuf {
    state_dir().join("backups")
}

pub fn reports_dir() -> PathBuf {
    state_dir().join("reports")
}

pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

/// Атомарная запись файла (временный файл в том же каталоге + rename).
/// mode 0600 для файлов с чувствительными данными (история чата).
pub fn atomic_write(path: &Path, contents: &[u8], mode_0600: bool) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "у файла нет родительского каталога")
    })?;
    ensure_dir(parent)?;
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("out");
    let tmp = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));

    let write_result = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents)?;
        f.sync_all()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if mode_0600 { 0o600 } else { 0o644 };
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode))?;
        }
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    write_result
}

/// Значения из /etc/os-release (NAME, PRETTY_NAME, ID, ID_LIKE, VERSION_ID).
pub fn read_os_release() -> HashMap<String, String> {
    let mut out = HashMap::new();
    let raw = match std::fs::read_to_string("/etc/os-release") {
        Ok(v) => v,
        Err(_) => return out,
    };
    parse_os_release(&raw, &mut out);
    out
}

pub fn parse_os_release(raw: &str, out: &mut HashMap<String, String>) {
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
        if !key.is_empty() {
            out.insert(key.to_string(), value);
        }
    }
}

/// Поиск исполняемого файла в PATH (без запуска внешних процессов).
pub fn which(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(binary);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// Имена запущенных процессов (нижний регистр) по /proc/*/comm. Лучшее усилие.
pub fn running_process_names() -> HashSet<String> {
    let mut names = HashSet::new();
    let Ok(entries) = std::fs::read_dir("/proc") else { return names };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else { continue };
        if !name_str.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let comm_path = entry.path().join("comm");
        if let Ok(comm) = std::fs::read_to_string(&comm_path) {
            let trimmed = comm.trim().to_lowercase();
            if !trimmed.is_empty() {
                names.insert(trimmed);
            }
        }
    }
    names
}

/// Канонизирует путь без следования по финальной ссылке сверх требуемого:
/// возвращает None, если путь выходит за пределы `root`.
pub fn ensure_within(path: &Path, root: &Path) -> Option<PathBuf> {
    let canon_root = root.canonicalize().ok()?;
    let canon = path.canonicalize().ok()?;
    if canon.starts_with(&canon_root) {
        Some(canon)
    } else {
        None
    }
}
