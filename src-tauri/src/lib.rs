// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod backend;

use backend::risk_normalization_command;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![risk_normalization_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
