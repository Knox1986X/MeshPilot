//! Настройки приложения: ~/.config/meshpilot/settings.json (без секретов).
//! Ключи провайдеров хранятся отдельно в secrets.rs.
use serde::{Deserialize, Serialize};

use crate::util;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssistantSettings {
    pub provider: String, // "groq" | "openrouter"
    pub model: String,
    #[serde(default)]
    pub backup_enabled: bool,
    #[serde(default)]
    pub backup_provider: Option<String>,
    #[serde(default)]
    pub backup_model: Option<String>,
    pub timeout_seconds: u64,
    pub max_tokens: u32,
    pub temperature: f64,
    pub sir_addressing: bool,
    #[serde(default)]
    pub user_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JarvisSettings {
    pub save_history: bool,
    pub history_limit: u32,
    pub wake_word: bool,
    pub push_to_talk_hotkey: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CleanupSettings {
    pub safe_profile_only: bool,
    pub include_trash: bool,
    pub keep_pacman_versions: u32,
    pub journal_max_size_mb: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkSettings {
    pub bypass_port: u16,
    pub check_urls: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UiSettings {
    pub demo_mode: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub assistant: AssistantSettings,
    pub jarvis: JarvisSettings,
    pub cleanup: CleanupSettings,
    pub network: NetworkSettings,
    pub ui: UiSettings,
}

pub fn default_settings() -> Settings {
    Settings {
        schema_version: SCHEMA_VERSION,
        assistant: AssistantSettings {
            provider: "groq".to_string(),
            model: "llama-3.3-70b-versatile".to_string(),
            backup_enabled: false,
            backup_provider: None,
            backup_model: None,
            timeout_seconds: 30,
            max_tokens: 1024,
            temperature: 0.4,
            sir_addressing: true,
            user_name: String::new(),
        },
        jarvis: JarvisSettings {
            save_history: true,
            history_limit: 200,
            wake_word: false,
            push_to_talk_hotkey: "Ctrl+Space".to_string(),
        },
        cleanup: CleanupSettings {
            safe_profile_only: true,
            include_trash: false,
            keep_pacman_versions: 2,
            journal_max_size_mb: 500,
        },
        network: NetworkSettings {
            bypass_port: 18080,
            check_urls: vec![
                "https://www.youtube.com/generate_204".to_string(),
                "https://discord.com/robots.txt".to_string(),
            ],
        },
        ui: UiSettings { demo_mode: false },
    }
}

fn settings_file() -> std::path::PathBuf {
    util::config_dir().join("settings.json")
}

/// Загрузить настройки; отсутствие/битый файл → дефолт (с резервной копией битого файла).
pub fn load_settings() -> Settings {
    let file = settings_file();
    let raw = match std::fs::read_to_string(&file) {
        Ok(v) => v,
        Err(_) => return default_settings(),
    };
    match serde_json::from_str::<Settings>(&raw) {
        Ok(s) => normalize(s),
        Err(_) => {
            // Не теряем файл молча: откладываем его рядом и возвращаем дефолт.
            let backup = file.with_extension("broken.json");
            let _ = std::fs::rename(&file, &backup);
            default_settings()
        }
    }
}

/// Нормализация: зажимаем числа в допустимые диапазоны, отбрасываем мусор.
pub fn normalize(mut s: Settings) -> Settings {
    s.schema_version = SCHEMA_VERSION;
    s.assistant.provider = normalize_provider(&s.assistant.provider).unwrap_or_else(|| "groq".to_string());
    s.assistant.backup_provider = s.assistant.backup_provider.as_deref().and_then(normalize_provider);
    s.assistant.model = clean_label(&s.assistant.model, 200, "llama-3.3-70b-versatile");
    s.assistant.backup_model = s
        .assistant
        .backup_model
        .as_deref()
        .map(|m| clean_label(m, 200, ""))
        .filter(|m| !m.is_empty());
    s.assistant.user_name = clean_label(&s.assistant.user_name, 80, "");
    s.assistant.timeout_seconds = s.assistant.timeout_seconds.clamp(5, 120);
    s.assistant.max_tokens = s.assistant.max_tokens.clamp(64, 8192);
    s.assistant.temperature = s.assistant.temperature.clamp(0.0, 2.0);
    s.jarvis.history_limit = s.jarvis.history_limit.clamp(1, 1000);
    s.jarvis.push_to_talk_hotkey = clean_label(&s.jarvis.push_to_talk_hotkey, 60, "Ctrl+Space");
    s.cleanup.keep_pacman_versions = s.cleanup.keep_pacman_versions.clamp(0, 10);
    s.cleanup.journal_max_size_mb = s.cleanup.journal_max_size_mb.clamp(50, 10_000);
    s.network.bypass_port = s.network.bypass_port.clamp(1024, 65535);
    s.network.check_urls = s
        .network
        .check_urls
        .iter()
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
        .map(|u| clean_label(u, 300, ""))
        .filter(|u| !u.is_empty())
        .take(10)
        .collect();
    s
}

pub fn normalize_provider(p: &str) -> Option<String> {
    match p {
        "groq" | "openrouter" => Some(p.to_string()),
        _ => None,
    }
}

fn clean_label(raw: &str, max_len: usize, fallback: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).take(max_len).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn save_settings(s: &Settings) -> std::io::Result<()> {
    let body = serde_json::to_vec_pretty(s).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    util::atomic_write(&settings_file(), &body, true)
}

/// Глубокое слияние JSON-объектов: `patch` накладывается поверх `base`.
fn deep_merge(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(b), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                match b.get_mut(k) {
                    Some(slot) => deep_merge(slot, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (slot, p) => *slot = p.clone(),
    }
}

/// Применить частичный патч (как Partial<Settings> с фронтенда) к текущим настройкам.
pub fn apply_patch(base: &Settings, patch: &serde_json::Value) -> Result<Settings, String> {
    let mut merged = serde_json::to_value(base).map_err(|e| e.to_string())?;
    if !patch.is_object() {
        return Err("патч настроек должен быть JSON-объектом".to_string());
    }
    // Не позволяем подменять версию схемы через патч.
    if let Some(obj) = patch.as_object() {
        let mut cleaned = obj.clone();
        cleaned.remove("schemaVersion");
        deep_merge(&mut merged, &serde_json::Value::Object(cleaned));
    }
    let parsed: Settings = serde_json::from_value(merged).map_err(|e| format!("некорректные настройки: {e}"))?;
    Ok(normalize(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_normalizable() {
        let d = default_settings();
        let n = normalize(d.clone());
        assert_eq!(d, n);
    }

    #[test]
    fn patch_merges_sections_and_pins_schema() {
        let base = default_settings();
        let patch = serde_json::json!({ "cleanup": { "safeProfileOnly": false }, "schemaVersion": 999 });
        let out = apply_patch(&base, &patch).expect("merge");
        assert!(!out.cleanup.safe_profile_only);
        assert!(!out.cleanup.include_trash); // не задан — из базовых
        assert_eq!(out.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn out_of_range_numbers_are_clamped() {
        let base = default_settings();
        let patch = serde_json::json!({ "assistant": { "timeoutSeconds": 10_000, "temperature": 99.0 } });
        let out = apply_patch(&base, &patch).expect("merge");
        assert_eq!(out.assistant.timeout_seconds, 120);
        assert_eq!(out.assistant.temperature, 2.0);
    }

    #[test]
    fn unknown_provider_is_rejected_down_to_default() {
        let base = default_settings();
        let patch = serde_json::json!({ "assistant": { "provider": "evil" } });
        let out = apply_patch(&base, &patch).expect("merge");
        assert_eq!(out.assistant.provider, "groq");
    }

    #[test]
    fn urls_without_scheme_are_dropped() {
        let base = default_settings();
        let patch = serde_json::json!({ "network": { "checkUrls": ["https://a.example/x", "ftp://b", "javascript:alert(1)"] } });
        let out = apply_patch(&base, &patch).expect("merge");
        assert_eq!(out.network.check_urls, vec!["https://a.example/x".to_string()]);
    }
}
