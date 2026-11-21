//! LLM-провайдеры (Groq, OpenRouter) — OpenAI-совместимый API.
//! Ключи приходят из secrets.rs и никогда не попадают в ответы/логи.
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

use crate::settings::{normalize_provider, Settings};

#[derive(Clone, Copy)]
pub struct ProviderConf {
    pub id: &'static str,
    pub chat_url: &'static str,
    pub models_url: &'static str,
}

pub const PROVIDERS: &[ProviderConf] = &[
    ProviderConf { id: "groq", chat_url: "https://api.groq.com/openai/v1/chat/completions", models_url: "https://api.groq.com/openai/v1/models" },
    ProviderConf { id: "openrouter", chat_url: "https://openrouter.ai/api/v1/chat/completions", models_url: "https://openrouter.ai/api/v1/models" },
];

pub fn provider_conf(id: &str) -> Option<ProviderConf> {
    PROVIDERS.iter().copied().find(|p| p.id == id)
}

const SECRET_ACCOUNT_PREFIX: &str = "llm-";

pub fn secret_account(provider: &str) -> String {
    format!("{SECRET_ACCOUNT_PREFIX}{provider}")
}

pub fn build_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("MeshPilot/", "0.2.0", " (+https://github.com/Knox1986X/MeshPilot)"))
        .redirect(reqwest::redirect::Policy::limited(3))
        .timeout(timeout)
        .build()
        .expect("reqwest client")
}

/// Безопасное извлечение текста ошибки из ответа провайдера (без ключей).
fn provider_error_message(body: &Value) -> Option<String> {
    body.get("error")
        .and_then(|e| e.get("message").or(Some(e)))
        .and_then(|m| m.as_str().map(|s| s.to_string()))
        .map(|s| s.chars().filter(|c| !c.is_control()).take(300).collect())
}

// ---------- Проверка подключения ----------

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    pub ok: bool,
    pub code: &'static str,
    pub message: String,
    pub latency_ms: Option<u64>,
}

impl ConnectionTest {
    fn result(code: &'static str, message: impl Into<String>, latency_ms: Option<u64>) -> Self {
        let ok = code == "ok";
        Self { ok, code, message: message.into(), latency_ms }
    }
}

pub async fn test_connection(provider: &str, model: &str, key: &str, timeout_secs: u64) -> ConnectionTest {
    let Some(conf) = provider_conf(provider) else {
        return ConnectionTest::result("unknown", "неизвестный провайдер", None);
    };
    let client = build_client(Duration::from_secs(timeout_secs.clamp(5, 60)));
    let started = Instant::now();
    let resp = match client.get(conf.models_url).bearer_auth(key).send().await {
        Ok(r) => r,
        Err(e) if e.is_timeout() => return ConnectionTest::result("timeout", "время ожидания ответа истекло", None),
        Err(e) if e.is_connect() => return ConnectionTest::result("no-network", "нет сетевого соединения с провайдером", None),
        Err(e) => return ConnectionTest::result("unknown", format!("ошибка сети: {e}"), None),
    };
    let latency = Some(started.elapsed().as_millis() as u64);
    let status = resp.status().as_u16();
    match status {
        200 => {
            let body: Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => return ConnectionTest::result("unknown", "ответ провайдера не похож на JSON", latency),
            };
            let ids: Vec<&str> = body
                .get("data")
                .and_then(|d| d.as_array())
                .map(|arr| arr.iter().filter_map(|m| m.get("id").and_then(|v| v.as_str())).collect())
                .unwrap_or_default();
            if model.is_empty() || ids.contains(&model) {
                ConnectionTest::result("ok", "соединение установлено, модель доступна", latency)
            } else {
                ConnectionTest::result("model-unavailable", format!("модель «{model}» не найдена у провайдера"), latency)
            }
        }
        401 | 403 => ConnectionTest::result("invalid-key", "ключ отклонён провайдером (401/403)", latency),
        404 => ConnectionTest::result("model-unavailable", "эндпоинт моделей не найден (404)", latency),
        429 => ConnectionTest::result("rate-limited", "превышен лимит запросов (429)", latency),
        500..=599 => ConnectionTest::result("server-error", format!("ошибка сервера провайдера ({status})"), latency),
        other => {
            let body: Value = resp.json().await.unwrap_or(Value::Null);
            let detail = provider_error_message(&body).unwrap_or_else(|| format!("неожиданный статус {other}"));
            ConnectionTest::result("unknown", detail, latency)
        }
    }
}

/// Список моделей у провайдера. Ошибка — AppError с безопасным сообщением.
pub async fn list_models(provider: &str, key: &str, timeout_secs: u64) -> crate::error::CmdResult<Vec<String>> {
    let conf = provider_conf(provider).ok_or_else(|| crate::error::AppError::new("validation", "неизвестный провайдер"))?;
    let client = build_client(Duration::from_secs(timeout_secs.clamp(5, 60)));
    let resp = client
        .get(conf.models_url)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| crate::error::AppError::new("network", format!("ошибка сети: {e}")))?;
    let status = resp.status().as_u16();
    if status == 401 || status == 403 {
        return Err(crate::error::AppError::new("invalid-key", "ключ отклонён провайдером"));
    }
    if status == 429 {
        return Err(crate::error::AppError::new("rate-limited", "превышен лимит запросов (429)"));
    }
    if !(200..300).contains(&status) {
        return Err(crate::error::AppError::new("provider", format!("провайдер ответил статусом {status}")));
    }
    let body: Value = resp.json().await.map_err(|_| crate::error::AppError::new("provider", "ответ не похож на JSON"))?;
    let mut ids: Vec<String> = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| arr.iter().filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())).collect())
        .unwrap_or_default();
    ids.sort();
    ids.dedup();
    ids.truncate(300);
    Ok(ids)
}

// ---------- Джарвис: чат с инструментами ----------

#[derive(Clone, Debug)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct ToolCallRecord {
    pub tool: String,
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

const TOOL_OPEN_SECTION: &str = "open_section";

fn tools_spec() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": TOOL_OPEN_SECTION,
                "description": "Открыть раздел интерфейса приложения в главном окне.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "section": { "type": "string", "enum": ["overview", "jarvis", "cleanup", "network", "permissions", "settings"] }
                    },
                    "required": ["section"],
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "cleanup_analyze",
                "description": "Read-only анализ того, что можно очистить (кэши). Ничего не удаляет. Возвращает оценку в байтах и список категорий.",
                "parameters": { "type": "object", "properties": {}, "additionalProperties": false }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_snapshot",
                "description": "Текущие метрики машины: CPU, память, диск, сеть, аптайм.",
                "parameters": { "type": "object", "properties": {}, "additionalProperties": false }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "dns_status",
                "description": "Текущий DNS: менеджер, серверы, DoT/DNSSEC. Read-only.",
                "parameters": { "type": "object", "properties": {}, "additionalProperties": false }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "mirrors_status",
                "description": "Статус зеркал pacman: дистрибутив, найденные ранжировщики, наличие mirrorlist. Read-only.",
                "parameters": { "type": "object", "properties": {}, "additionalProperties": false }
            }
        }
    ])
}

fn section_ru(section: &str) -> &'static str {
    match section {
        "overview" => "Обзор",
        "jarvis" => "Джарвис",
        "cleanup" => "Очистка",
        "network" => "Сеть",
        "permissions" => "Разрешения",
        "settings" => "Настройки",
        _ => "раздел",
    }
}

fn short_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KIB { return format!("{bytes} Б"); }
    let units = ["КБ", "МБ", "ГБ", "ТБ"];
    let mut v = b / KIB;
    let mut u = 0;
    while v >= KIB && u < units.len() - 1 { v /= KIB; u += 1; }
    format!("{v:.1} {}", units[u])
}

pub fn system_prompt(settings: &Settings) -> String {
    let mut p = String::from(
        "Ты — Джарвис, встроенный ассистент приложения MeshPilot для обслуживания рабочей станции на CachyOS Linux (KDE Plasma, Wayland, pacman). \
Отвечай кратко и по делу на русском языке. \
Инструменты, которые тебе доступны, — только чтение и навигация по приложению. \
Ты НИКОГДА не удаляешь файлы, не меняешь DNS, зеркала, службы и настройки, и не утверждаешь, что сделал это: такие действия выполняет сам пользователь в интерфейсе MeshPilot с подтверждением. \
Если пользователь просит «почистить» или «применить» — объясни, что именно сделает соответствующий раздел, и открой его инструментом open_section. \
Не выдумывай результаты системных проверок: вызывай инструмент и опирайся на его ответ. \
Если инструмент вернул ошибку — честно скажи об этом и предложи, что сделать.",
    );
    if settings.assistant.sir_addressing {
        p.push_str(" Обращайся к пользователю «сэр».");
    }
    let name = settings.assistant.user_name.trim();
    if !name.is_empty() {
        p.push_str(&format!(" Имя пользователя: {name}."));
    }
    p
}

/// Исполняет один вызов инструмента. Все инструменты — read-only или навигация.
/// Возвращает (record для UI, JSON-результат для модели).
/// `metrics` — общее состояние опроса: дельты CPU/сети считаются от последнего замера UI.
pub fn execute_tool(tool: &str, args: &Value, metrics: &crate::snapshot::MetricsState) -> (ToolCallRecord, Value) {
    match tool {
        TOOL_OPEN_SECTION => {
            let section = args.get("section").and_then(|v| v.as_str()).unwrap_or("");
            let valid = ["overview", "jarvis", "cleanup", "network", "permissions", "settings"];
            if valid.contains(&section) {
                (
                    ToolCallRecord { tool: tool.into(), label: format!("Открыть раздел «{}»", section_ru(section)), ok: true, detail: section.to_string() },
                    json!({ "ok": true, "opened": section }),
                )
            } else {
                (
                    ToolCallRecord { tool: tool.into(), label: "Открыть раздел".into(), ok: false, detail: format!("недопустимый раздел: {section}") },
                    json!({ "ok": false, "error": "недопустимый раздел" }),
                )
            }
        }
        "cleanup_analyze" => {
            let report = crate::cleanup::analyze();
            let summary = json!({
                "estimatedBytes": report.total_estimated_bytes,
                "estimatedHuman": short_bytes(report.total_estimated_bytes),
                "categories": report.categories.iter().filter(|c| c.available).map(|c| c.id.clone()).collect::<Vec<_>>(),
                "warnings": report.warnings,
            });
            (
                ToolCallRecord {
                    tool: tool.into(),
                    label: "Анализ очистки".into(),
                    ok: true,
                    detail: format!("оценка: {}", short_bytes(report.total_estimated_bytes)),
                },
                summary,
            )
        }
        "get_snapshot" => {
            let metrics = crate::snapshot::collect(metrics);
            let mem_pct = match (metrics.memory_used_bytes, metrics.memory_total_bytes) {
                (Some(u), Some(t)) if t > 0 => format!("{:.0}%", (u as f64 / t as f64) * 100.0),
                _ => "?".to_string(),
            };
            let cpu = metrics.cpu_total_percent.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "?".to_string());
            let summary = json!({
                "cpu": cpu, "memoryUsedPercent": mem_pct,
                "disks": metrics.disks.len(),
                "uptimeSeconds": metrics.uptime_seconds,
                "os": metrics.os_name,
            });
            (
                ToolCallRecord { tool: tool.into(), label: "Телеметрия системы".into(), ok: true, detail: format!("CPU {cpu}, память {mem_pct}") },
                summary,
            )
        }
        "dns_status" => {
            let status = crate::network::dns_status();
            let summary = json!({
                "manager": status.manager, "servers": status.current_servers, "dnsOverTls": status.dns_over_tls, "dnssec": status.dnssec,
            });
            let detail = if status.current_servers.is_empty() { "серверы не определены".to_string() } else { status.current_servers.join(", ") };
            (
                ToolCallRecord { tool: tool.into(), label: "Статус DNS".into(), ok: true, detail },
                summary,
            )
        }
        "mirrors_status" => {
            let status = crate::network::mirrors_status();
            let summary = json!({
                "os": status.os_name, "isArchLike": status.is_arch_like, "tools": status.detected_tools, "mirrorlistExists": status.mirrorlist_exists,
            });
            let detail = if status.is_arch_like {
                format!("arch-like; инструменты: {}", if status.detected_tools.is_empty() { "нет".into() } else { status.detected_tools.join(", ") })
            } else {
                "не arch-like дистрибутив".into()
            };
            (
                ToolCallRecord { tool: tool.into(), label: "Статус зеркал".into(), ok: true, detail },
                summary,
            )
        }
        _ => (
            ToolCallRecord { tool: tool.into(), label: tool.into(), ok: false, detail: "инструмент не поддерживается".into() },
            json!({ "ok": false, "error": "инструмент не поддерживается" }),
        ),
    }
}

pub struct JarvisOutcome {
    pub content: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub error: Option<String>,
}

fn error_outcome(message: String) -> JarvisOutcome {
    JarvisOutcome {
        content: "Не удалось получить ответ от провайдера. Проверьте сохранённый ключ (Настройки → Джарвис → Подключения) и сетевое соединение.".to_string(),
        tool_calls: Vec::new(),
        error: Some(message),
    }
}

/// Полный цикл диалога: до 3 раундов вызовов инструментов.
pub async fn run_conversation(
    settings: &Settings,
    history: &[ChatTurn],
    metrics: &crate::snapshot::MetricsState,
    resolve_key: impl Fn(&str) -> Option<String>,
) -> JarvisOutcome {
    // Порядок: основной провайдер, затем резервный (если включён и отличается).
    let mut candidates: Vec<String> = vec![settings.assistant.provider.clone()];
    if settings.assistant.backup_enabled {
        if let Some(backup) = settings.assistant.backup_provider.as_deref().and_then(normalize_provider) {
            if backup != settings.assistant.provider {
                candidates.push(backup);
            }
        }
    }

    let mut last_error = "нет настроенного провайдера".to_string();
    let mut attempted = false;

    for provider in candidates {
        let Some(conf) = provider_conf(&provider) else { continue };
        let Some(key) = resolve_key(&provider) else {
            last_error = format!("{provider}: ключ не сохранён");
            continue;
        };
        attempted = true;
        let model = if provider == settings.assistant.provider {
            settings.assistant.model.clone()
        } else {
            settings
                .assistant
                .backup_model
                .clone()
                .unwrap_or_else(|| settings.assistant.model.clone())
        };
        match converse(settings, conf, &model, &key, history, metrics).await {
            Ok(outcome) => return outcome,
            Err(message) => last_error = format!("{provider}: {message}"),
        }
    }

    if !attempted {
        return error_outcome(format!("{last_error}. Откройте Настройки → Джарвис → Подключения и сохраните ключ."));
    }
    error_outcome(last_error)
}

async fn converse(
    settings: &Settings,
    conf: ProviderConf,
    model: &str,
    key: &str,
    history: &[ChatTurn],
    metrics: &crate::snapshot::MetricsState,
) -> Result<JarvisOutcome, String> {
    let timeout = Duration::from_secs(settings.assistant.timeout_seconds.clamp(5, 120));
    let client = build_client(timeout);

    let mut messages = vec![json!({ "role": "system", "content": system_prompt(settings) })];
    for turn in history.iter().take(20) {
        let role = match turn.role.as_str() {
            "user" => "user",
            "assistant" => "assistant",
            _ => continue,
        };
        let content: String = turn.content.chars().take(16_000).collect();
        messages.push(json!({ "role": role, "content": content }));
    }

    let mut records: Vec<ToolCallRecord> = Vec::new();

    for _round in 0..3 {
        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools_spec(),
            "tool_choice": "auto",
            "max_tokens": settings.assistant.max_tokens,
            "temperature": settings.assistant.temperature,
        });
        let resp = client
            .post(conf.chat_url)
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    "время ожидания истекло".to_string()
                } else if e.is_connect() {
                    "нет сетевого соединения".to_string()
                } else {
                    format!("ошибка сети: {e}")
                }
            })?;
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.map_err(|_| "ответ провайдера не похож на JSON".to_string())?;
        if !(200..300).contains(&status) {
            let detail = provider_error_message(&body).unwrap_or_else(|| format!("статус {status}"));
            return Err(detail);
        }

        let message = body
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .cloned()
            .ok_or_else(|| "пустой ответ провайдера".to_string())?;

        let tool_calls = message.get("tool_calls").and_then(|t| t.as_array()).cloned();
        match tool_calls {
            Some(calls) if !calls.is_empty() => {
                messages.push(message);
                for call in calls.iter().take(8) {
                    let name = call.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                    let raw_args = call.get("function").and_then(|f| f.get("arguments")).and_then(|a| a.as_str()).unwrap_or("{}");
                    let args: Value = serde_json::from_str(raw_args).unwrap_or(Value::Null);
                    let call_id = call.get("id").and_then(|i| i.as_str()).unwrap_or("call").to_string();
                    let (record, result) = execute_tool(name, &args, metrics);
                    records.push(record);
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": result.to_string(),
                    }));
                }
            }
            _ => {
                let content = message.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                return Ok(JarvisOutcome { content, tool_calls: records, error: None });
            }
        }
    }

    // Модель не уложилась в раунды — честно сообщаем.
    Ok(JarvisOutcome {
        content: "Модель исчерпала лимит вызовов инструментов за один запрос. Попробуйте переформулировать или разбейте задачу на шаги.".to_string(),
        tool_calls: records,
        error: Some("лимит вызовов инструментов".to_string()),
    })
}

// Тихая утилита для тестов/логов: маскирует ключ, если он где-то всплывёт.
#[allow(dead_code)]
pub fn redact_key(input: &str) -> String {
    if input.len() <= 8 {
        return "«ключ»".to_string();
    }
    format!("{}…{}", &input[..4], &input[input.len() - 2..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_contains_rules_and_optional_parts() {
        let mut s = crate::settings::default_settings();
        let p = system_prompt(&s);
        assert!(p.contains("Джарвис"));
        assert!(p.contains("«сэр»"));
        s.assistant.sir_addressing = false;
        s.assistant.user_name = "Андрей".to_string();
        let p2 = system_prompt(&s);
        assert!(!p2.contains("«сэр»"));
        assert!(p2.contains("Андрей"));
    }

    fn test_metrics() -> crate::snapshot::MetricsState {
        crate::snapshot::MetricsState::new()
    }

    #[test]
    fn open_section_tool_validates_enum() {
        let m = test_metrics();
        let (rec, _) = execute_tool(TOOL_OPEN_SECTION, &json!({"section": "network"}), &m);
        assert!(rec.ok);
        let (rec2, _) = execute_tool(TOOL_OPEN_SECTION, &json!({"section": "../../etc"}), &m);
        assert!(!rec2.ok);
        let (rec3, _) = execute_tool(TOOL_OPEN_SECTION, &Value::Null, &m);
        assert!(!rec3.ok);
    }

    #[test]
    fn unknown_tool_is_rejected() {
        let m = test_metrics();
        let (rec, _) = execute_tool("rm_rf", &json!({}), &m);
        assert!(!rec.ok);
    }
}
