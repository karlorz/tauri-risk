// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;

use tauri::{generate_handler, Builder};

fn main() {
    Builder::default()
    .invoke_handler(generate_handler![backend::risk_normalization_command])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
