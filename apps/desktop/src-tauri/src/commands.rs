//! Команды Tauri IPC. Каждая команда валидирует вход и возвращает DTO с camelCase.
//! Тяжёлые синхронные операции выполняются в spawn_blocking, чтобы не блокировать окно.
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::cleanup;
use crate::error::{AppError, CmdResult};
use crate::llm;
use crate::network;
use crate::perms;
use crate::schedules;
use crate::secrets::{self, KeyLocation, SecretsState};
use crate::settings;
use crate::snapshot::{MetricsState, Snapshot};
use crate::util;

// =====================================================================
// Обзор
// =====================================================================

#[tauri::command]
pub async fn get_snapshot(state: State<'_, Arc<MetricsState>>) -> CmdResult<Snapshot> {
    let state = state.inner().clone();
    let snap = tauri::async_runtime::spawn_blocking(move || crate::snapshot::collect(&state))
        .await
        .map_err(|e| AppError::internal(format!("задача снимка завершилась с ошибкой: {e}")))?;
    Ok(snap)
}

// =====================================================================
// Очистка
// =====================================================================

#[tauri::command]
pub async fn cleanup_analyze() -> CmdResult<cleanup::CleanupReport> {
    tauri::async_runtime::spawn_blocking(cleanup::analyze)
        .await
        .map_err(|e| AppError::internal(format!("задача анализа завершилась с ошибкой: {e}")))
}

#[tauri::command]
pub async fn cleanup_apply(req: cleanup::CleanupApplyRequest) -> CmdResult<cleanup::CleanupApplyResult> {
    if req.category_ids.is_empty() {
        return Err(AppError::new("validation", "не выбрана ни одна категория"));
    }
    if req.category_ids.len() > 32 {
        return Err(AppError::new("validation", "слишком много категорий за один вызов"));
    }
    let mut result = tauri::async_runtime::spawn_blocking(move || cleanup::apply(&req))
        .await
        .map_err(|e| AppError::internal(format!("задача очистки завершилась с ошибкой: {e}")))?;
    // Повторный анализ, чтобы UI показал актуальные остатки.
    let report = tauri::async_runtime::spawn_blocking(cleanup::analyze)
        .await
        .map_err(|e| AppError::internal(format!("повторный анализ завершился с ошибкой: {e}")))?;
    result.report = Some(report);
    Ok(result)
}

// =====================================================================
// Зеркала / DNS / обход
// =====================================================================

#[tauri::command]
pub async fn mirrors_status() -> CmdResult<network::MirrorsStatus> {
    tauri::async_runtime::spawn_blocking(network::mirrors_status)
        .await
        .map_err(|e| AppError::internal(format!("mirrors_status: {e}")))
}

fn require_confirm(confirm: bool) -> CmdResult<()> {
    if confirm {
        Ok(())
    } else {
        Err(AppError::new("confirmation-required", "пользователь не подтвердил операцию"))
    }
}

#[tauri::command]
pub async fn mirrors_apply(confirm: bool) -> CmdResult<network::OpResult> {
    require_confirm(confirm)?;
    tauri::async_runtime::spawn_blocking(network::mirrors_apply)
        .await
        .map_err(|e| AppError::internal(format!("mirrors_apply: {e}")))
}

#[tauri::command]
pub async fn dns_status() -> CmdResult<network::DnsStatus> {
    tauri::async_runtime::spawn_blocking(network::dns_status)
        .await
        .map_err(|e| AppError::internal(format!("dns_status: {e}")))
}

#[tauri::command]
pub async fn dns_benchmark() -> CmdResult<network::DnsBenchmarkResult> {
    tauri::async_runtime::spawn_blocking(network::dns_benchmark)
        .await
        .map_err(|e| AppError::internal(format!("dns_benchmark: {e}")))
}

#[tauri::command]
pub async fn dns_apply(confirm: bool) -> CmdResult<network::OpResult> {
    require_confirm(confirm)?;
    tauri::async_runtime::spawn_blocking(network::dns_apply)
        .await
        .map_err(|e| AppError::internal(format!("dns_apply: {e}")))
}

#[tauri::command]
pub async fn dns_rollback(confirm: bool) -> CmdResult<network::OpResult> {
    require_confirm(confirm)?;
    tauri::async_runtime::spawn_blocking(network::dns_rollback)
        .await
        .map_err(|e| AppError::internal(format!("dns_rollback: {e}")))
}

#[tauri::command]
pub async fn bypass_status() -> CmdResult<network::BypassStatus> {
    tauri::async_runtime::spawn_blocking(network::bypass_status)
        .await
        .map_err(|e| AppError::internal(format!("bypass_status: {e}")))
}

fn validate_test_urls(urls: &[String]) -> CmdResult<Vec<String>> {
    if urls.is_empty() {
        return Err(AppError::new("validation", "укажите хотя бы один URL"));
    }
    if urls.len() > 10 {
        return Err(AppError::new("validation", "максимум 10 URL за раз"));
    }
    let mut out = Vec::with_capacity(urls.len());
    for raw in urls {
        let u: String = raw.chars().filter(|c| !c.is_control()).take(300).collect();
        let trimmed = u.trim();
        if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
            return Err(AppError::new("validation", format!("URL без http(s):// отклонён: {trimmed}")));
        }
        out.push(trimmed.to_string());
    }
    Ok(out)
}

#[tauri::command]
pub async fn bypass_test(urls: Vec<String>) -> CmdResult<network::BypassTestResult> {
    let urls = validate_test_urls(&urls)?;
    tauri::async_runtime::spawn_blocking(move || network::bypass_test(&urls))
        .await
        .map_err(|e| AppError::internal(format!("bypass_test: {e}")))
}

#[tauri::command]
pub async fn bypass_enable(confirm: bool) -> CmdResult<network::OpResult> {
    require_confirm(confirm)?;
    tauri::async_runtime::spawn_blocking(network::bypass_enable)
        .await
        .map_err(|e| AppError::internal(format!("bypass_enable: {e}")))
}

#[tauri::command]
pub async fn bypass_disable(confirm: bool) -> CmdResult<network::OpResult> {
    require_confirm(confirm)?;
    tauri::async_runtime::spawn_blocking(network::bypass_disable)
        .await
        .map_err(|e| AppError::internal(format!("bypass_disable: {e}")))
}

// =====================================================================
// Расписания
// =====================================================================

#[tauri::command]
pub fn schedules_list() -> CmdResult<Vec<schedules::ScheduleInfo>> {
    Ok(schedules::list())
}

#[tauri::command]
pub async fn schedule_set(id: String, enabled: bool) -> CmdResult<network::OpResult> {
    let id = id.trim().to_string();
    if id.len() > 64 {
        return Err(AppError::new("validation", "некорректный id расписания"));
    }
    tauri::async_runtime::spawn_blocking(move || schedules::set_enabled(&id, enabled))
        .await
        .map_err(|e| AppError::internal(format!("schedule_set: {e}")))
}

// =====================================================================
// Разрешения
// =====================================================================

#[tauri::command]
pub async fn permission_checks() -> CmdResult<Vec<perms::PermissionInfo>> {
    tauri::async_runtime::spawn_blocking(perms::checks)
        .await
        .map_err(|e| AppError::internal(format!("permission_checks: {e}")))
}

// =====================================================================
// LLM: статус, ключи, тест, модели
// =====================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub provider: String,
    pub has_key: bool,
    pub storage: String, // keyring | session | none
    pub keyring_available: bool,
}

fn checked_provider(provider: &str) -> CmdResult<String> {
    settings::normalize_provider(provider.trim())
        .ok_or_else(|| AppError::new("validation", "неизвестный провайдер (только groq | openrouter)"))
}

#[tauri::command]
pub fn llm_provider_status(provider: String, state: State<'_, SecretsState>) -> CmdResult<ProviderStatus> {
    let provider = checked_provider(&provider)?;
    let account = llm::secret_account(&provider);
    let (secret, location) = secrets::get_secret(&state, &account);
    Ok(ProviderStatus {
        provider,
        has_key: secret.is_some(),
        storage: location.as_str().to_string(),
        keyring_available: secrets::keyring_available(),
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveKeyResult {
    pub ok: bool,
    pub storage: String,
    pub message: String,
}

#[tauri::command]
pub fn llm_save_key(provider: String, key: String, state: State<'_, SecretsState>) -> CmdResult<SaveKeyResult> {
    let provider = checked_provider(&provider)?;
    let key = key.trim().to_string();
    // Не принимаем совсем короткие/длинные ключи и ключи с управляющими символами.
    if key.len() < 8 || key.len() > 512 || key.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(AppError::new("validation", "ключ не похож на API-ключ (длина 8–512, без пробелов)"));
    }
    let location = secrets::store_secret(&state, &llm::secret_account(&provider), &key);
    Ok(match location {
        KeyLocation::Keyring => SaveKeyResult { ok: true, storage: "keyring".into(), message: "Ключ сохранён в системном кольце (Secret Service)".into() },
        KeyLocation::Session => SaveKeyResult { ok: true, storage: "session".into(), message: "Системное кольцо недоступно: ключ хранится только в памяти и будет забыт при выходе из приложения".into() },
        KeyLocation::None => SaveKeyResult { ok: false, storage: "none".into(), message: "Не удалось сохранить ключ".into() },
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteKeyResult {
    pub ok: bool,
    pub message: String,
}

#[tauri::command]
pub fn llm_delete_key(provider: String, state: State<'_, SecretsState>) -> CmdResult<DeleteKeyResult> {
    let provider = checked_provider(&provider)?;
    let removed = secrets::delete_secret(&state, &llm::secret_account(&provider));
    Ok(DeleteKeyResult {
        ok: true, // удаление идемпотентно
        message: match removed {
            KeyLocation::Keyring => "Ключ удалён из системного кольца".into(),
            KeyLocation::Session => "Ключ удалён из сессии".into(),
            KeyLocation::None => "Ключ не был сохранён — нечего удалять".into(),
        },
    })
}

#[tauri::command]
pub async fn llm_test(provider: String, model: String, state: State<'_, SecretsState>) -> CmdResult<llm::ConnectionTest> {
    let provider = checked_provider(&provider)?;
    let model = model.trim().chars().take(200).collect::<String>();
    let account = llm::secret_account(&provider);
    let (secret, _) = secrets::get_secret(&state, &account);
    let Some(key) = secret else {
        return Ok(llm::ConnectionTest { ok: false, code: "invalid-key", message: "ключ не сохранён".to_string(), latency_ms: None });
    };
    let timeout_secs = settings::load_settings().assistant.timeout_seconds;
    Ok(llm::test_connection(&provider, &model, &key, timeout_secs).await)
}

#[tauri::command]
pub async fn llm_list_models(provider: String, state: State<'_, SecretsState>) -> CmdResult<Vec<String>> {
    let provider = checked_provider(&provider)?;
    let account = llm::secret_account(&provider);
    let (secret, _) = secrets::get_secret(&state, &account);
    let Some(key) = secret else {
        return Err(AppError::new("no-key", "ключ не сохранён"));
    };
    let timeout_secs = settings::load_settings().assistant.timeout_seconds;
    llm::list_models(&provider, &key, timeout_secs).await
}

// =====================================================================
// Джарвис и история
// =====================================================================

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JarvisSendRequest {
    pub messages: Vec<JarvisTurnInput>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JarvisTurnInput {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at_utc: String,
    pub tool_calls: Vec<ToolCallInfoOwned>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallInfoOwned {
    pub tool: String,
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JarvisSendResult {
    pub reply: ChatMessage,
}

#[tauri::command]
pub async fn jarvis_send(req: JarvisSendRequest, state: State<'_, SecretsState>, metrics: State<'_, Arc<MetricsState>>) -> CmdResult<JarvisSendResult> {
    let history: Vec<llm::ChatTurn> = req
        .messages
        .iter()
        .rev()
        .take(20)
        .rev()
        .filter(|t| t.role == "user" || t.role == "assistant")
        .map(|t| llm::ChatTurn { role: t.role.clone(), content: t.content.chars().take(16_000).collect() })
        .collect();

    let settings_snapshot = settings::load_settings();
    let metrics_arc = metrics.inner().clone();
    let secrets_state = state.inner();
    // resolve_key читает ключи из системного кольца/сессии; в сеть не ходит.
    let outcome = llm::run_conversation(
        &settings_snapshot,
        &history,
        metrics_arc.as_ref(),
        |provider| {
            let (secret, _) = secrets::get_secret(secrets_state, &llm::secret_account(provider));
            secret
        },
    )
    .await;

    Ok(JarvisSendResult {
        reply: ChatMessage {
            id: format!("r-{}", chrono::Utc::now().timestamp_millis()),
            role: "assistant".to_string(),
            content: outcome.content,
            created_at_utc: util::now_rfc3339(),
            tool_calls: outcome
                .tool_calls
                .into_iter()
                .map(|t| ToolCallInfoOwned { tool: t.tool, label: t.label, ok: t.ok, detail: t.detail.chars().take(400).collect() })
                .collect(),
            error: outcome.error,
        },
    })
}

fn history_file() -> std::path::PathBuf {
    util::state_dir().join("history.json")
}

fn validate_history_message(m: &ChatMessage) -> Result<(), String> {
    if m.id.len() > 128 {
        return Err("слишком длинное поле id".into());
    }
    if m.role != "user" && m.role != "assistant" {
        return Err("роль должна быть user или assistant".into());
    }
    if m.content.len() > 65_536 {
        return Err("слишком длинное содержимое сообщения".into());
    }
    if m.tool_calls.len() > 32 {
        return Err("слишком много вызовов инструментов".into());
    }
    for t in &m.tool_calls {
        if t.tool.len() > 64 || t.label.len() > 200 || t.detail.len() > 400 {
            return Err("слишком длинные поля вызова инструмента".into());
        }
    }
    if let Some(e) = &m.error {
        if e.len() > 4096 {
            return Err("слишком длинное сообщение об ошибке".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub fn history_load() -> CmdResult<Vec<ChatMessage>> {
    let path = history_file();
    let raw = match std::fs::read(&path) {
        Ok(r) => r,
        Err(_) => return Ok(vec![]), // файла нет — история пуста
    };
    match serde_json::from_slice::<Vec<ChatMessage>>(&raw) {
        Ok(messages) => Ok(messages.into_iter().filter(|m| validate_history_message(m).is_ok()).collect()),
        Err(_) => {
            let backup = path.with_extension("broken.json");
            let _ = std::fs::rename(&path, &backup);
            Ok(vec![])
        }
    }
}

#[tauri::command]
pub fn history_save(messages: Vec<ChatMessage>) -> CmdResult<()> {
    if messages.len() > 1000 {
        return Err(AppError::new("validation", "максимум 1000 сообщений в истории"));
    }
    for m in &messages {
        validate_history_message(m).map_err(|e| AppError::new("validation", e))?;
    }
    let body = serde_json::to_vec(&messages).map_err(|e| AppError::internal(format!("сериализация истории: {e}")))?;
    util::atomic_write(&history_file(), &body, true).map_err(|e| AppError::internal(format!("запись истории: {e}")))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearHistoryResult {
    pub ok: bool,
    pub message: String,
}

#[tauri::command]
pub fn history_clear() -> CmdResult<ClearHistoryResult> {
    let path = history_file();
    let removed = if path.exists() { std::fs::remove_file(&path).is_ok() } else { true };
    Ok(ClearHistoryResult {
        ok: removed,
        message: if removed { "История очищена".into() } else { "Не удалось удалить файл истории".into() },
    })
}

// =====================================================================
// Настройки
// =====================================================================

#[tauri::command]
pub fn settings_get() -> CmdResult<settings::Settings> {
    Ok(settings::load_settings())
}

#[tauri::command]
pub fn settings_update(patch: Value) -> CmdResult<settings::Settings> {
    let base = settings::load_settings();
    let merged = settings::apply_patch(&base, &patch).map_err(|e| AppError::new("validation", e))?;
    settings::save_settings(&merged).map_err(|e| AppError::internal(format!("запись настроек: {e}")))?;
    Ok(merged)
}

#[tauri::command]
pub fn settings_reset() -> CmdResult<settings::Settings> {
    // Ключи провайдеров НЕ трогаем — они хранятся отдельно в кольце/сессии.
    let defaults = settings::default_settings();
    settings::save_settings(&defaults).map_err(|e| AppError::internal(format!("запись настроек: {e}")))?;
    Ok(defaults)
}
