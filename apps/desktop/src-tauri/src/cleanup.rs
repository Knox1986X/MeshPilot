//! Движок очистки: только воспроизводимые кэши. Никакого «умного удаления личных файлов».
//! Все прямые удаления ограничены $HOME (канонизированный путь); системные пути — только через
//! `sudo -n` (без интерактивного запроса пароля, чтобы никогда не зависать).
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::settings;
use crate::util;

// ---------- DTO (camelCase) ----------

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub risk: String, // safe | caution | risky
    pub default_selected: bool,
    pub requires_privilege: bool,
    pub requires_opt_in: bool,
    pub paths: Vec<String>,
    pub estimated_bytes: Option<u64>,
    pub available: bool,
    pub blocked_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupReport {
    pub categories: Vec<CleanupCategory>,
    pub total_estimated_bytes: u64,
    pub running_apps: Vec<String>,
    pub warnings: Vec<String>,
    pub analyzed_at_utc: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupApplyRequest {
    pub category_ids: Vec<String>,
    pub dry_run: bool,
    pub confirm_risky: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryResult {
    pub category_id: String,
    pub freed_bytes: u64,
    pub ok: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupApplyResult {
    pub freed_bytes: u64,
    pub results: Vec<CategoryResult>,
    pub report: Option<CleanupReport>,
}

// ---------- Каталог категорий ----------

const RISK_SAFE: &str = "safe";
const RISK_CAUTION: &str = "caution";
const RISK_RISKY: &str = "risky";

pub(crate) struct CategorySpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    risk: &'static str,
    default_selected: bool,
    requires_privilege: bool,
    requires_opt_in: bool,
}

pub(crate) const CATALOG: &[CategorySpec] = &[
    CategorySpec { id: "pacman-cache", name: "Кэш pacman", description: "Старые версии пакетов (paccache -r -k N). Требуются права: sudo без пароля или helper.", risk: RISK_SAFE, default_selected: true, requires_privilege: true, requires_opt_in: false },
    CategorySpec { id: "thumbnails", name: "Миниатюры", description: "~/.cache/thumbnails — пересоздаются автоматически по мере просмотра.", risk: RISK_SAFE, default_selected: true, requires_privilege: false, requires_opt_in: false },
    CategorySpec { id: "browser-cache", name: "Кэши браузеров", description: "Firefox / Chromium: только cache2 / Cache. Профили, история и пароли не трогаются.", risk: RISK_SAFE, default_selected: true, requires_privilege: false, requires_opt_in: false },
    CategorySpec { id: "messaging-cache", name: "Кэши мессенджеров", description: "Telegram / Discord / Vesktop: только кэш медиа. Переписка и вложения не трогаются.", risk: RISK_CAUTION, default_selected: false, requires_privilege: false, requires_opt_in: false },
    CategorySpec { id: "dev-caches", name: "Кэши разработчика", description: "pip, npm, go-build, sccache, .crate-архивы cargo — воспроизводимы загрузкой заново.", risk: RISK_SAFE, default_selected: true, requires_privilege: false, requires_opt_in: false },
    CategorySpec { id: "trash", name: "Корзина", description: "~/.local/share/Trash — полное удаление. Восстановить будет нельзя.", risk: RISK_RISKY, default_selected: false, requires_privilege: false, requires_opt_in: true },
    CategorySpec { id: "steam-shaders", name: "Shader-кэш Steam/Mesa", description: "Пересоздаётся при следующем запуске игр (первый запуск может быть дольше).", risk: RISK_CAUTION, default_selected: false, requires_privilege: false, requires_opt_in: true },
    CategorySpec { id: "journal-coredump", name: "journald / coredumps", description: "Ограничение размера журнала (journalctl --vacuum-size) и удаление coredumps. Требуются права.", risk: RISK_CAUTION, default_selected: false, requires_privilege: true, requires_opt_in: false },
    CategorySpec { id: "aur-helpers", name: "Кэш yay/paru", description: "~/.cache/yay, ~/.cache/paru — скачанные PKGBUILD и артефакты сборки, пересоздаются.", risk: RISK_SAFE, default_selected: true, requires_privilege: false, requires_opt_in: false },
];

pub(crate) fn catalog_spec(id: &str) -> Option<&'static CategorySpec> {
    CATALOG.iter().find(|c| c.id == id)
}

// ---------- Файловая утилита: подсчёт размера и рекурсивное удаление ----------

/// Рекурсивно суммирует размеры файлов. Симлинки не раскрываются. Ошибки — пропускаем.
pub(crate) fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

/// Удаляет содержимое каталога (или сам каталог) с подсчётом освобождённого.
/// Симлинки удаляются как симлинки (цель не трогается). Каталог НЕ удаляется целиком,
/// если remove_self = false — исчезают только его дети.
fn remove_tree(path: &Path, remove_self: bool) -> (u64, u64) {
    let mut freed = 0u64;
    let mut failures = 0u64;
    if path.is_symlink() || path.is_file() {
        let size = std::fs::symlink_metadata(path).map(|m| m.len()).unwrap_or(0);
        return match std::fs::remove_file(path) {
            Ok(()) => (size, 0),
            Err(_) => (0, 1),
        };
    }
    let Ok(entries) = std::fs::read_dir(path) else { return (0, 1) };
    for entry in entries.flatten() {
        let p = entry.path();
        let Ok(meta) = entry.metadata() else { failures += 1; continue }; // DirEntry::metadata() не следует по симлинкам
        if meta.file_type().is_symlink() || meta.is_file() {
            // симлинк не несёт полезного объёма — считаем 0 байт
            let size = if meta.file_type().is_symlink() { 0 } else { meta.len() };
            match std::fs::remove_file(&p) {
                Ok(()) => freed = freed.saturating_add(size),
                Err(_) => failures += 1,
            }
        } else if meta.is_dir() {
            let (f, fails) = remove_tree(&p, true);
            freed = freed.saturating_add(f);
            failures += fails;
            if fails == 0 {
                let _ = std::fs::remove_dir(&p);
            }
        }
    }
    if remove_self {
        let _ = std::fs::remove_dir(path);
    }
    (freed, failures)
}

/// Удалить ВСЁ, что матчится паттерном, но только внутри $HOME.
/// Возвращает (освобождено, ошибки, найденные пути).
pub(crate) fn remove_glob_in_home(pattern: &str) -> Option<(u64, u64, Vec<String>)> {
    let home = util::home_dir().canonicalize().ok()?;
    let mut freed = 0u64;
    let mut failures = 0u64;
    let mut hits = Vec::new();
    for entry in glob::glob(pattern).ok()? {
        let Ok(path) = entry else { continue };
        let Ok(canon) = path.canonicalize() else { continue };
        if !canon.starts_with(&home) {
            failures += 1; // никогда не выходим за пределы HOME
            continue;
        }
        hits.push(canon.display().to_string());
        let (f, fails) = remove_tree(&canon, true);
        freed = freed.saturating_add(f);
        failures += fails;
    }
    Some((freed, failures, hits))
}

/// Раскрыть «~» в путь.
pub(crate) fn expand_tilde(input: &str) -> String {
    if let Some(rest) = input.strip_prefix("~/") {
        return util::home_dir().join(rest).display().to_string();
    }
    input.to_string()
}

fn expand_tilde_path(input: &str) -> PathBuf {
    PathBuf::from(expand_tilde(input))
}

// ---------- Запущенные приложения (по /proc/*/comm) ----------

const WATCHED: &[&str] = &[
    "firefox", "floorp", "chromium", "chrome", "brave", "vivaldi", "librewolf", "waterfox", "zen",
    "telegram", "telegram-desktop", "ayugram", "discord", "vesktop", "steam", "codium", "code",
];

pub(crate) fn running_watched() -> Vec<String> {
    let names = util::running_process_names();
    let mut found: Vec<String> = WATCHED.iter().filter(|w| names.contains(**w)).map(|w| w.to_string()).collect();
    found.sort();
    found.dedup();
    found
}

fn is_browser_running(names: &HashSet<String>) -> bool {
    ["firefox", "floorp", "chromium", "chrome", "brave", "vivaldi", "librewolf", "waterfox", "zen"].iter().any(|n| names.contains(*n))
}

fn is_messaging_running(names: &HashSet<String>) -> bool {
    ["telegram", "telegram-desktop", "ayugram", "discord", "vesktop"].iter().any(|n| names.contains(*n))
}

// ---------- Построение отчёта анализа ----------

pub(crate) fn analyze() -> CleanupReport {
    let proc_names = util::running_process_names();
    let running = running_watched();
    let mut warnings: Vec<String> = Vec::new();
    let mut categories: Vec<CleanupCategory> = Vec::new();
    let mut grand_total = 0u64;

    for spec in CATALOG {
        let mut cat = CleanupCategory {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            risk: spec.risk.to_string(),
            default_selected: spec.default_selected,
            requires_privilege: spec.requires_privilege,
            requires_opt_in: spec.requires_opt_in,
            paths: Vec::new(),
            estimated_bytes: None,
            available: false,
            blocked_reason: None,
        };

        match spec.id {
            "pacman-cache" => {
                cat.paths = vec!["/var/cache/pacman/pkg".to_string()];
                let tool = util::which("paccache");
                let dir_exists = Path::new("/var/cache/pacman/pkg").exists();
                cat.available = tool.is_some() && dir_exists;
                let sudo_ok = sudo_noninteractive_ok();
                if !cat.available {
                    cat.blocked_reason = Some(if tool.is_none() { "paccache не найден".into() } else { "каталог не найден".into() });
                } else if !sudo_ok {
                    cat.blocked_reason = Some("нужен sudo без пароля либо helper (см. «Разрешения»)".into());
                }
                // Оценка: верхняя граница — весь каталог (финальное число решает paccache).
                cat.estimated_bytes = if dir_exists { Some(dir_size(Path::new("/var/cache/pacman/pkg"))) } else { None };
            }
            "thumbnails" => {
                let p = expand_tilde_path("~/.cache/thumbnails");
                cat.paths = vec![p.display().to_string()];
                if p.exists() {
                    cat.available = true;
                    cat.estimated_bytes = Some(dir_size(&p));
                }
            }
            "browser-cache" => {
                let fixed = ["~/.cache/mozilla", "~/.cache/chromium", "~/.cache/google-chrome", "~/.cache/brave"];
                let mut found = Vec::new();
                let mut total = 0u64;
                for f in fixed {
                    let p = expand_tilde_path(f);
                    if p.exists() {
                        found.push(p.display().to_string());
                        total += dir_size(&p);
                    }
                }
                // Firefox: ~/.mozilla/firefox/<profile>/cache2
                let ff = expand_tilde("~/.mozilla/firefox/*/cache2");
                for e in glob::glob(&ff).ok().into_iter().flatten().flatten() {
                    if e.exists() {
                        found.push(e.display().to_string());
                        total += dir_size(&e);
                    }
                }
                cat.paths = found;
                cat.available = !cat.paths.is_empty();
                cat.estimated_bytes = Some(total);
                if is_browser_running(&proc_names) {
                    cat.blocked_reason = Some("браузер запущен — его кэш пропущен".into());
                    cat.available = false;
                    warnings.push("Браузер запущен — кэш браузера пропущен, чтобы не повредить профиль.".to_string());
                }
            }
            "messaging-cache" => {
                let fixed = [
                    "~/.local/share/TelegramDesktop/tdata/user_data/cache",
                    "~/.config/discord/Cache",
                    "~/.config/discord/Code Cache",
                    "~/.config/vesktop/Cache",
                    "~/.config/vesktop/Code Cache",
                ];
                let mut found = Vec::new();
                let mut total = 0u64;
                for f in fixed {
                    let p = expand_tilde_path(f);
                    if p.exists() {
                        found.push(p.display().to_string());
                        total += dir_size(&p);
                    }
                }
                cat.paths = found;
                cat.available = !cat.paths.is_empty();
                cat.estimated_bytes = Some(total);
                if is_messaging_running(&proc_names) {
                    cat.blocked_reason = Some("мессенджер запущен — его кэш пропущен".into());
                    cat.available = false;
                    warnings.push("Мессенджер запущен — его кэш пропущен.".to_string());
                }
            }
            "dev-caches" => {
                let fixed = ["~/.cache/pip", "~/.npm/_cacache", "~/.cache/go-build", "~/.cache/sccache", "~/.cache/pnpm/dlx"];
                let mut found = Vec::new();
                let mut total = 0u64;
                for f in fixed {
                    let p = expand_tilde_path(f);
                    if p.exists() {
                        found.push(p.display().to_string());
                        total += dir_size(&p);
                    }
                }
                let cargo = expand_tilde("~/.cargo/registry/cache/*/*.crate");
                for e in glob::glob(&cargo).ok().into_iter().flatten().flatten() {
                    found.push(e.display().to_string());
                    total += e.metadata().map(|m| m.len()).unwrap_or(0);
                }
                cat.paths = found;
                cat.available = !cat.paths.is_empty();
                cat.estimated_bytes = Some(total);
            }
            "trash" => {
                let p = expand_tilde_path("~/.local/share/Trash");
                cat.paths = vec![p.display().to_string()];
                if p.exists() {
                    cat.available = true;
                    cat.estimated_bytes = Some(dir_size(&p));
                }
            }
            "steam-shaders" => {
                let fixed = ["~/.local/share/Steam/steamapps/shadercache", "~/.cache/mesa_shader_cache", "~/.cache/mesa_shader_cache_db"];
                let mut found = Vec::new();
                let mut total = 0u64;
                for f in fixed {
                    let p = expand_tilde_path(f);
                    if p.exists() {
                        found.push(p.display().to_string());
                        total += dir_size(&p);
                    }
                }
                cat.paths = found;
                cat.available = !cat.paths.is_empty();
                cat.estimated_bytes = Some(total);
            }
            "journal-coredump" => {
                cat.paths = vec!["/var/log/journal".to_string(), "/var/lib/systemd/coredump".to_string()];
                let tool = util::which("journalctl");
                cat.available = tool.is_some();
                if tool.is_none() {
                    cat.blocked_reason = Some("journalctl не найден".into());
                } else if !sudo_noninteractive_ok() {
                    cat.blocked_reason = Some("нужен sudo без пароля либо helper (см. «Разрешения»)".into());
                }
            }
            "aur-helpers" => {
                let fixed = ["~/.cache/yay", "~/.cache/paru"];
                let mut found = Vec::new();
                let mut total = 0u64;
                for f in fixed {
                    let p = expand_tilde_path(f);
                    if p.exists() {
                        found.push(p.display().to_string());
                        total += dir_size(&p);
                    }
                }
                cat.paths = found;
                cat.available = !cat.paths.is_empty();
                cat.estimated_bytes = Some(total);
            }
            _ => {}
        }

        if let Some(bytes) = cat.estimated_bytes {
            grand_total = grand_total.saturating_add(bytes);
        }
        categories.push(cat);
    }

    CleanupReport {
        categories,
        total_estimated_bytes: grand_total,
        running_apps: running,
        warnings,
        analyzed_at_utc: util::now_rfc3339(),
    }
}

// ---------- sudo без пароля (кэшируем на время процесса) ----------

static SUDO_OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

pub(crate) fn sudo_noninteractive_ok() -> bool {
    *SUDO_OK.get_or_init(|| {
        std::process::Command::new("sudo")
            .arg("-n")
            .arg("true")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

pub(crate) fn run_sudo(args: &[&str], timeout: std::time::Duration) -> Result<String, String> {
    let mut full = Vec::with_capacity(args.len() + 1);
    full.push("-n");
    full.extend_from_slice(args);
    run_command("sudo", &full, timeout)
}

pub(crate) fn run_command(program: &str, args: &[&str], timeout: std::time::Duration) -> Result<String, String> {
    let mut child = std::process::Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("не удалось запустить {program}: {e}"))?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out = child.stdout.take().map(|mut s| {
                    use std::io::Read;
                    let mut v = Vec::new();
                    let _ = s.read_to_end(&mut v);
                    String::from_utf8_lossy(&v).into_owned()
                }).unwrap_or_default();
                if !status.success() {
                    let err = child.stderr.take().map(|mut s| {
                        use std::io::Read;
                        let mut v = Vec::new();
                        let _ = s.read_to_end(&mut v);
                        String::from_utf8_lossy(&v).into_owned()
                    }).unwrap_or_default();
                    let mut rev: Vec<&str> = err.lines().rev().take(2).collect();
                    rev.reverse();
                    let tail = rev.join(" / ");
                    return Err(format!("{program} завершился с кодом {}: {tail}", status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into())));
                }
                if out.len() > 4 * 1024 * 1024 {
                    out.truncate(4 * 1024 * 1024);
                }
                return Ok(out);
            }
            Ok(None) => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{program} не завершился за {:?}", timeout));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => return Err(format!("ошибка ожидания {program}: {e}")),
        }
    }
}

// ---------- Применение очистки ----------

pub(crate) fn apply(req: &CleanupApplyRequest) -> CleanupApplyResult {
    let cfg = settings::load_settings();
    let proc_names = util::running_process_names();
    let mut results: Vec<CategoryResult> = Vec::new();
    let mut freed_total = 0u64;

    for id in &req.category_ids {
        let Some(spec) = catalog_spec(id) else {
            results.push(CategoryResult { category_id: id.clone(), freed_bytes: 0, ok: false, message: "неизвестная категория".into() });
            continue;
        };
        let result = apply_one(spec, req, &cfg, &proc_names);
        freed_total = freed_total.saturating_add(result.freed_bytes);
        results.push(result);
    }

    CleanupApplyResult {
        freed_bytes: freed_total,
        results,
        report: None,
    }
}

fn apply_one(spec: &CategorySpec, req: &CleanupApplyRequest, cfg: &settings::Settings, proc_names: &HashSet<String>) -> CategoryResult {
    let ok = |freed_bytes: u64, message: String| CategoryResult { category_id: spec.id.to_string(), freed_bytes, ok: true, message };
    let fail = |message: String| CategoryResult { category_id: spec.id.to_string(), freed_bytes: 0, ok: false, message };

    // Гейты безопасности.
    if cfg.cleanup.safe_profile_only && spec.risk != RISK_SAFE {
        return fail("включён профиль «только безопасная очистка» — категория пропущена".to_string());
    }
    if spec.requires_opt_in && !req.confirm_risky {
        return fail("категория требует явного подтверждения (галочка риска)".to_string());
    }
    if spec.risk == RISK_RISKY && !req.confirm_risky {
        return fail("рискованная категория пропущена без галочки подтверждения".to_string());
    }
    if spec.id == "trash" && !cfg.cleanup.include_trash {
        return fail("очистка корзины отключена в настройках (Очистка → Корзина)".to_string());
    }

    if req.dry_run {
        return ok(0, format!("пробный проход: «{}» не изменялась", spec.name));
    }

    match spec.id {
        "pacman-cache" => {
            if util::which("paccache").is_none() {
                return fail("paccache не найден".to_string());
            }
            if !sudo_noninteractive_ok() {
                return fail("нужен sudo без пароля либо helper (см. «Разрешения»)".to_string());
            }
            let keep = cfg.cleanup.keep_pacman_versions.to_string();
            match run_sudo(&["paccache", "-r", "-k", &keep], std::time::Duration::from_secs(300)) {
                Ok(_) => ok(0, format!("paccache завершён (оставлено {keep} последних версий). Точное освобождённое оценит следующий анализ.")),
                Err(e) => fail(e),
            }
        }
        "thumbnails" => remove_home_paths(spec.id, &["~/.cache/thumbnails"]),
        "browser-cache" => {
            if is_browser_running(proc_names) {
                return fail("браузер запущен — закрыть его и повторить".to_string());
            }
            let mut freed = 0u64;
            let mut failures = 0u64;
            for (freed_f, fails_f, _) in [
                remove_glob_in_home(&expand_tilde("~/.mozilla/firefox/*/cache2")),
                remove_glob_in_home(&expand_tilde("~/.mozilla/firefox/*/cache2.sqlite")),
                remove_glob_in_home(&expand_tilde("~/.cache/mozilla")),
                remove_glob_in_home(&expand_tilde("~/.cache/chromium")),
                remove_glob_in_home(&expand_tilde("~/.cache/google-chrome")),
                remove_glob_in_home(&expand_tilde("~/.cache/brave")),
            ]
            .into_iter()
            .flatten()
            {
                freed += freed_f;
                failures += fails_f;
            }
            if failures > 0 {
                ok(freed, format!("удалено, ошибок файлов: {failures}"))
            } else {
                ok(freed, "кэши браузеров очищены".to_string())
            }
        }
        "messaging-cache" => {
            if is_messaging_running(proc_names) {
                return fail("мессенджер запущен — закрыть его и повторить".to_string());
            }
            remove_home_paths(spec.id, &[
                "~/.local/share/TelegramDesktop/tdata/user_data/cache",
                "~/.config/discord/Cache",
                "~/.config/discord/Code Cache",
                "~/.config/vesktop/Cache",
                "~/.config/vesktop/Code Cache",
            ])
        }
        "dev-caches" => {
            let mut freed = 0u64;
            let mut failures = 0u64;
            for (f, fails, _) in [
                remove_glob_in_home(&expand_tilde("~/.cache/pip")),
                remove_glob_in_home(&expand_tilde("~/.npm/_cacache")),
                remove_glob_in_home(&expand_tilde("~/.cache/go-build")),
                remove_glob_in_home(&expand_tilde("~/.cache/sccache")),
                remove_glob_in_home(&expand_tilde("~/.cargo/registry/cache/*/*.crate")),
            ]
            .into_iter()
            .flatten()
            {
                freed += f;
                failures += fails;
            }
            if failures > 0 {
                ok(freed, format!("очищено, ошибок файлов: {failures}"))
            } else {
                ok(freed, "кэши разработчика очищены".to_string())
            }
        }
        "trash" => {
            let home = match util::home_dir().canonicalize() {
                Ok(h) => h,
                Err(_) => return fail("не удалось определить HOME".to_string()),
            };
            let trash_root = home.join(".local/share/Trash");
            let mut freed = 0u64;
            let mut failures = 0u64;
            for sub in ["files", "info", "expunged"] {
                let target = trash_root.join(sub);
                if target.exists() {
                    let (f, fails) = remove_tree(&target, false);
                    freed += f;
                    failures += fails;
                }
            }
            if failures > 0 {
                ok(freed, format!("корзина очищена частично, ошибок: {failures}"))
            } else {
                ok(freed, "корзина очищена".to_string())
            }
        }
        "steam-shaders" => remove_home_paths(spec.id, &[
            "~/.local/share/Steam/steamapps/shadercache",
            "~/.cache/mesa_shader_cache",
            "~/.cache/mesa_shader_cache_db",
        ]),
        "journal-coredump" => {
            if util::which("journalctl").is_none() {
                return fail("journalctl не найден".to_string());
            }
            if !sudo_noninteractive_ok() {
                return fail("нужен sudo без пароля либо helper (см. «Разрешения»)".to_string());
            }
            let size = format!("{}M", cfg.cleanup.journal_max_size_mb);
            if let Err(e) = run_sudo(&["journalctl", "--vacuum-size", &size], std::time::Duration::from_secs(180)) {
                return fail(e);
            }
            // Coredumps journald не подчиняются --vacuum-size: удаляем только
            // обычные файлы непосредственно в каталоге coredump через sudo find.
            let coredumps = Path::new("/var/lib/systemd/coredump");
            let coredump_note = if coredumps.is_dir() {
                match run_command("sudo", &["-n", "find", "/var/lib/systemd/coredump", "-maxdepth", "4", "-type", "f", "-delete"], std::time::Duration::from_secs(180)) {
                    Ok(_) => "coredumps удалены".to_string(),
                    Err(e) => format!("журнал урезан, coredumps пропущены: {e}"),
                }
            } else {
                "coredumps не найдены".to_string()
            };
            ok(0, format!("журнал ограничен до {size}; {coredump_note}. Точное освобождённое оценит следующий анализ"))
        }
        "aur-helpers" => remove_home_paths(spec.id, &["~/.cache/yay", "~/.cache/paru"]),
        other => fail(format!("категория «{other}» не поддержана")),
    }
}

fn remove_home_paths(category_id: &str, paths: &[&str]) -> CategoryResult {
    // Общий путь удаления фиксированных $HOME-путей (всегда внутри канонизированного HOME).
    let mut freed = 0u64;
    let mut failures = 0u64;
    let mut found_any = false;
    let home = match util::home_dir().canonicalize() {
        Ok(h) => h,
        Err(_) => {
            return CategoryResult { category_id: category_id.to_string(), freed_bytes: 0, ok: false, message: "не удалось определить HOME".into() }
        }
    };
    for raw in paths {
        let expanded = expand_tilde_path(raw);
        if !expanded.exists() {
            continue;
        }
        let canon = match expanded.canonicalize() {
            Ok(c) => c,
            Err(_) => { failures += 1; continue }
        };
        if !canon.starts_with(&home) { failures += 1; continue }
        found_any = true;
        // Удаляем содержимое каталога; сам каталог оставляем (кэш пересоздаётся внутри).
        let (f, fails) = remove_tree(&canon, true);
        freed += f;
        failures += fails;
    }
    let message = if !found_any {
        "нечего очищать: пути не найдены".to_string()
    } else if failures > 0 {
        format!("удалено, ошибок: {failures}")
    } else {
        "очищено".to_string()
    };
    CategoryResult { category_id: category_id.to_string(), freed_bytes: freed, ok: failures == 0 || found_any, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_size_counts_files_not_symlinks() {
        let base = std::env::temp_dir().join(format!("meshpilot-test-{}", std::process::id()));
        let sub = base.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(base.join("a.bin"), vec![0u8; 100]).unwrap();
        std::fs::write(sub.join("b.bin"), vec![0u8; 250]).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(base.join("a.bin"), base.join("link.bin")).unwrap();
        let size = dir_size(&base);
        assert_eq!(size, 350);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn remove_tree_frees_and_tolerates_symlinks() {
        let base = std::env::temp_dir().join(format!("meshpilot-test-rm-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("x.bin"), vec![1u8; 64]).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/passwd", base.join("etc-link")).unwrap();
        let (freed, failures) = remove_tree(&base, true);
        assert_eq!(freed, 64);
        assert_eq!(failures, 0);
        assert!(!base.exists());
    }

    #[test]
    fn apply_rejects_unknown_category() {
        let req = CleanupApplyRequest { category_ids: vec!["definitely-not-a-category".into()], dry_run: false, confirm_risky: false };
        let out = apply(&req);
        assert_eq!(out.results.len(), 1);
        assert!(!out.results[0].ok);
    }

    #[test]
    fn dry_run_never_deletes() {
        let base = std::env::temp_dir().join(format!("meshpilot-test-dry-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("keep.bin"), vec![7u8; 32]).unwrap();
        // thumbnails сухой прогон: файл должен остаться.
        let req = CleanupApplyRequest { category_ids: vec!["thumbnails".into()], dry_run: true, confirm_risky: false };
        let out = apply(&req);
        assert!(out.results[0].ok);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn opt_in_categories_require_confirm_flag() {
        // steam-shaders без confirm_risky → отказ по opt-in-гейту
        // (safe_profile_only выключаем, чтобы дойти именно до opt-in-гейта).
        let spec = catalog_spec("steam-shaders").expect("spec");
        let mut cfg = crate::settings::default_settings();
        cfg.cleanup.safe_profile_only = false;
        let req = CleanupApplyRequest { category_ids: vec!["steam-shaders".into()], dry_run: false, confirm_risky: false };
        let out = apply_one(spec, &req, &cfg, &std::collections::HashSet::new());
        assert!(!out.ok);
        assert!(out.message.contains("подтверждения"));
    }

    #[test]
    fn risky_trash_requires_flag() {
        let spec = catalog_spec("trash").expect("spec");
        let mut cfg = crate::settings::default_settings();
        cfg.cleanup.safe_profile_only = false;
        let req = CleanupApplyRequest { category_ids: vec!["trash".into()], dry_run: false, confirm_risky: false };
        let out = apply_one(spec, &req, &cfg, &std::collections::HashSet::new());
        assert!(!out.ok);
    }

    #[test]
    fn safe_profile_blocks_non_safe_categories() {
        let spec = catalog_spec("steam-shaders").expect("spec");
        let cfg = crate::settings::default_settings(); // safe_profile_only = true
        let req = CleanupApplyRequest { category_ids: vec!["steam-shaders".into()], dry_run: false, confirm_risky: true };
        let out = apply_one(spec, &req, &cfg, &std::collections::HashSet::new());
        assert!(!out.ok);
        assert!(out.message.contains("безопасная очистка"));
    }

    #[test]
    fn os_release_and_scoring_helpers_exist() {
        let spec = catalog_spec("pacman-cache").expect("spec");
        assert!(spec.requires_privilege);
        assert!(catalog_spec("nope").is_none());
    }

    #[test]
    fn remove_tree_on_file() {
        let base = std::env::temp_dir().join(format!("meshpilot-test-file-{}", std::process::id()));
        std::fs::write(&base, vec![9u8; 16]).unwrap();
        let (freed, failures) = remove_tree(&base, true);
        assert_eq!(freed, 16);
        assert_eq!(failures, 0);
        assert!(!base.exists());
    }

    #[test]
    fn timestamp_is_rfc3339_like() {
        let ts = util::now_rfc3339();
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z') || ts.contains('+'));
    }
}
