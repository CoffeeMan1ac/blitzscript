//! blitzscript core library. `main.rs` is a thin wrapper around `run`.

pub mod commands;
pub mod exec;

use blitzscript_core::db::Db;
use exec::RunRegistry;
use tauri::Manager;

/// Build and run the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(RunRegistry::default())
        .setup(|app| {
            // The SQLite file lives in the per-app data dir. It is a derived
            // cache + history log; deleting it and relaunching rebuilds cleanly.
            let dir = app
                .path()
                .app_data_dir()
                .expect("could not resolve app data dir");
            std::fs::create_dir_all(&dir).ok();
            let db_path = dir.join("blitzscript.db");
            let db = Db::open(&db_path).expect("failed to open database");
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_root,
            commands::set_root,
            commands::scan,
            commands::list_commands,
            commands::assess_command,
            commands::run_command,
            commands::cancel_run,
            commands::list_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running blitzscript");
}
