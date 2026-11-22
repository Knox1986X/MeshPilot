#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands {
    pub mod llm;
    pub mod optimizer;
    pub mod system;
}

use commands::{llm, optimizer, system};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            system::get_metrics,
            optimizer::clean_pacman_cache,
            optimizer::remove_orphans,
            optimizer::vacuum_journal,
            optimizer::clean_all,
            optimizer::optimize_mirrors,
            optimizer::optimize_dns,
            optimizer::zapret_status,
            optimizer::zapret_action,
            optimizer::system_health,
            llm::chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MeshPilot");
}
