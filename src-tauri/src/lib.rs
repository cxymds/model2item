pub mod app_state;
pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod services;

use app_state::AppState;
use std::{fs, path::PathBuf};
use tauri::Manager;

fn workbench_database_path(base_dir: PathBuf) -> PathBuf {
    base_dir.join("workbench.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data directory");
            fs::create_dir_all(&app_data_dir).expect("failed to create app data directory");

            let database_path = workbench_database_path(app_data_dir);
            let pool = tauri::async_runtime::block_on(async move {
                db::connect_file(database_path)
                    .await
                    .expect("failed to initialize database")
            });

            app.manage(AppState { pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::profile_commands::create_profile,
            commands::profile_commands::list_profiles,
            commands::window_binding_commands::create_window_binding,
            commands::window_binding_commands::list_window_bindings,
            commands::window_binding_commands::list_iterm_sessions,
            commands::window_binding_commands::refresh_window_binding_presence,
            commands::evaluation_case_commands::create_evaluation_case,
            commands::evaluation_case_commands::list_evaluation_cases,
            commands::comparison_commands::create_comparison_run,
            commands::comparison_commands::start_comparison_run,
            commands::comparison_commands::get_comparison_run,
            commands::comparison_commands::list_comparison_targets,
            commands::comparison_commands::get_comparison_summary,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri app");
}
