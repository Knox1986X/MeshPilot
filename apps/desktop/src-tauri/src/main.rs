#![forbid(unsafe_code)]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Не удалось открыть окно MeshPilot");
}
