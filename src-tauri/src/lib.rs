pub mod app_state;
pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod services;

use app_state::AppState;
use std::{fs, path::PathBuf};
use tauri::Manager;
use tokio::time::{sleep, Duration};

use crate::services::{
    comparison_run_service::ComparisonRunService, iterm_session_service::ItermSessionService,
    window_binding_service::WindowBindingService,
};

fn workbench_database_path(base_dir: PathBuf) -> PathBuf {
    base_dir.join("workbench.db")
}

fn spawn_window_binding_sync_task(pool: sqlx::SqlitePool) {
    tauri::async_runtime::spawn(async move {
        let session_service = ItermSessionService::new();
        let binding_service = WindowBindingService::new(pool.clone());
        let run_service = ComparisonRunService::new(pool);

        loop {
            match session_service.list_sessions().await {
                Ok(sessions) => {
                    let online_session_ids = sessions
                        .iter()
                        .map(|session| session.session_id.clone())
                        .collect::<Vec<_>>();
                    let _ = binding_service
                        .sync_with_online_sessions(&online_session_ids)
                        .await;
                    let _ = run_service.reconcile_closed_sessions(&online_session_ids).await;
                }
                Err(_) => {}
            }

            sleep(Duration::from_secs(15)).await;
        }
    });
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

            spawn_window_binding_sync_task(pool.clone());
            app.manage(AppState { pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::profile_commands::create_profile,
            commands::profile_commands::list_profiles,
            commands::window_binding_commands::create_window_binding,
            commands::window_binding_commands::list_window_bindings,
            commands::window_binding_commands::update_window_binding,
            commands::window_binding_commands::delete_window_binding,
            commands::window_binding_commands::list_iterm_sessions,
            commands::window_binding_commands::refresh_window_binding_presence,
            commands::evaluation_case_commands::create_evaluation_case,
            commands::evaluation_case_commands::list_evaluation_cases,
            commands::evaluation_case_commands::update_evaluation_case,
            commands::evaluation_case_commands::delete_evaluation_case,
            commands::comparison_commands::create_comparison_run,
            commands::comparison_commands::start_comparison_run,
            commands::comparison_commands::send_comparison_run_message,
            commands::comparison_commands::get_comparison_run,
            commands::comparison_commands::list_comparison_runs,
            commands::comparison_commands::list_comparison_targets,
            commands::comparison_commands::get_comparison_summary,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri app");
}
