#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod commands;
mod downloader;
mod installations;
mod ping;
mod settings;
mod storage;

use std::collections::VecDeque;
use tauri::Manager;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub client_logs: Mutex<VecDeque<String>>,
    pub installations_lock: Mutex<()>,
}

fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
        unsafe { std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "0") };
    }

    let builder = tauri_specta::Builder::new().commands(tauri_specta::collect_commands![
        commands::get_all_accounts,
        commands::add_account,
        commands::remove_account,
        commands::ensure_assets,
        commands::get_versions,
        commands::refresh_account,
        commands::get_skin_url,
        commands::get_patch_notes,
        commands::get_patch_content,
        commands::launch_game,
        commands::get_client_logs,
        commands::load_launcher_settings,
        commands::set_launcher_language,
        commands::set_keep_launcher_open,
        commands::set_launch_with_console,
        commands::ping_server,
        commands::load_servers,
        commands::save_servers,
        commands::load_installations,
        commands::create_installation,
        commands::delete_installation,
        commands::duplicate_installation,
        commands::edit_installation,
        commands::get_downloaded_versions,
    ]);

    #[cfg(debug_assertions)]
    if let Err(e) = builder.export(
        specta_typescript::Typescript::default(),
        "../src/bindings.ts",
    ) {
        panic!("{e}");
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            storage::ensure_dirs();
            app.manage(AppState::default());
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("failed to run Pomme launcher");
}
