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

#[cfg(target_os = "linux")]
use tauri::async_runtime::Mutex;

#[cfg(target_os = "linux")]
async fn snapshot_in_worker<T: Send + 'static>(
    in_flight: &'static Mutex<()>,
    read: fn() -> Option<T>,
) -> Option<T> {
    // Отдельный лимит на источник. Не блокировать окно и не копить запросы, если ФС медленная.
    let guard = in_flight.try_lock().ok()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        read()
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
async fn read_root_fs() -> Option<meshpilot_core::RootFsObservation> {
    #[cfg(target_os = "linux")]
    {
        static IN_FLIGHT: Mutex<()> = Mutex::const_new(());
        snapshot_in_worker(&IN_FLIGHT, meshpilot_linux::root_fs_snapshot).await
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
        static IN_FLIGHT: Mutex<()> = Mutex::const_new(());
        snapshot_in_worker(&IN_FLIGHT, meshpilot_linux::cpu_snapshot).await
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
        .invoke_handler(tauri::generate_handler![read_ram, read_cpu, read_root_fs])
        .run(tauri::generate_context!())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Не удалось открыть окно MeshPilot: {error}");
            ExitCode::FAILURE
        }
    }
}
