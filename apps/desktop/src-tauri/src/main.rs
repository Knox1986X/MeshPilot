#![forbid(unsafe_code)]

use std::process::ExitCode;

/// Снимок с фиксированным источником. Окно не передаёт ни путь, ни параметры команды.
#[tauri::command]
fn read_ram() -> Option<meshpilot_core::RamObservation> {
    #[cfg(target_os = "linux")]
    {
        meshpilot_linux::ram_snapshot()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[tauri::command]
async fn read_cpu() -> Option<meshpilot_core::CpuObservation> {
    #[cfg(target_os = "linux")]
    {
        // Не блокируем UI или async executor. Даже прямые вызовы IPC не создают очередь замеров.
        static IN_FLIGHT: tauri::async_runtime::Mutex<()> =
            tauri::async_runtime::Mutex::const_new(());
        let guard = IN_FLIGHT.try_lock().ok()?;
        tauri::async_runtime::spawn_blocking(move || {
            let _guard = guard;
            meshpilot_linux::cpu_snapshot()
        })
        .await
        .ok()
        .flatten()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    if rustix::process::geteuid().is_root() {
        eprintln!("MeshPilot: запустите окно от обычного пользователя, не от root");
        return ExitCode::from(77);
    }

    match tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![read_ram, read_cpu])
        .run(tauri::generate_context!())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Не удалось открыть окно MeshPilot: {error}");
            ExitCode::FAILURE
        }
    }
}
