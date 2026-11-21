//! MeshPilot — Джарвис и инструменты обслуживания CachyOS/Arch.
//! Архитектура: веб-view (React) → Tauri IPC → локальные системные вызовы.
//! Никакой телеметрии, никаких сетевых вызовов в фоне; сеть используется только по явному действию.
pub mod cleanup;
pub mod commands;
pub mod error;
pub mod llm;
pub mod network;
pub mod perms;
pub mod schedules;
pub mod secrets;
pub mod settings;
pub mod snapshot;
pub mod util;

use std::sync::Arc;

/// Разбор аргументов для headless-расписания: `meshpilot-desktop --run-schedule <id>`.
/// Если аргумент есть — выполняем отчёт и выходим до запуска окна.
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--run-schedule") {
        let id = args.get(pos + 1).cloned().unwrap_or_default();
        match schedules::run_headless(&id) {
            Ok(path) => {
                eprintln!("meshpilot: отчёт расписания сохранён: {path}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("meshpilot: расписание «{id}» завершилось с ошибкой: {e}");
                std::process::exit(1);
            }
        }
    }

    let metrics = Arc::new(snapshot::MetricsState::new());

    tauri::Builder::default()
        .manage(metrics)
        .manage(secrets::SecretsState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::cleanup_analyze,
            commands::cleanup_apply,
            commands::mirrors_status,
            commands::mirrors_apply,
            commands::dns_status,
            commands::dns_benchmark,
            commands::dns_apply,
            commands::dns_rollback,
            commands::bypass_status,
            commands::bypass_test,
            commands::bypass_enable,
            commands::bypass_disable,
            commands::schedules_list,
            commands::schedule_set,
            commands::permission_checks,
            commands::llm_provider_status,
            commands::llm_save_key,
            commands::llm_delete_key,
            commands::llm_test,
            commands::llm_list_models,
            commands::jarvis_send,
            commands::history_load,
            commands::history_save,
            commands::history_clear,
            commands::settings_get,
            commands::settings_update,
            commands::settings_reset,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска MeshPilot");
}
