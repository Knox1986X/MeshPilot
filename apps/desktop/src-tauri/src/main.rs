#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    if rustix::process::geteuid().is_root() {
        eprintln!("MeshPilot: запустите окно от обычного пользователя, не от root");
        return ExitCode::from(77);
    }

    match tauri::Builder::default().run(tauri::generate_context!()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Не удалось открыть окно MeshPilot: {error}");
            ExitCode::FAILURE
        }
    }
}
