pub mod agent;
pub mod bridge;
pub mod commands;
pub mod engine;
pub mod model;
pub mod scheduler;
pub mod store;
pub mod sources;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::agent::discovery::{CmdRunner, RealCmdRunner};
use crate::bridge::BackendState;
use crate::commands::load_app_config;
use crate::sources::{HttpClient, ReqwestClient};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Fix the GUI-launch PATH so child processes (agents, source fetches)
    // inherit the user's shell PATH. Guarded out of tests so unit tests never
    // mutate the process environment.
    #[cfg(not(test))]
    {
        // UNVERIFIED: live path
        let _ = fix_path_env::fix();
    }

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Webview,
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("freshet".into()),
                    },
                ))
                .level(log::LevelFilter::Info)
                .level_for("freshet_tmp_lib", log::LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Log the resolved log file path so it's discoverable in the log itself.
            // macOS: ~/Library/Logs/<bundle-identifier>/freshet.log
            // UNVERIFIED: live path
            if let Ok(log_dir) = app.path().app_log_dir() {
                let log_file = log_dir.join("freshet.log");
                log::info!(
                    "Freshet log file: {} | bundle: com.seedz.freshet-tmp | tail: tail -f {:?}",
                    log_file.display(),
                    log_file,
                );
            }

            // Auto-open DevTools in debug builds so failures are immediately visible.
            #[cfg(debug_assertions)]
            {
                if let Some(w) = app.get_webview_window("main") {
                    w.open_devtools();
                }
            }

            // Resolve the app-config dir (outside any stream root) and load the
            // app config from disk.
            // UNVERIFIED: live path
            let config_dir = app
                .path()
                .app_config_dir()
                .expect("failed to resolve app config dir");
            let config = load_app_config(&config_dir);

            // The real runner + HTTP client live in managed state.
            // UNVERIFIED: live path
            let runner: Arc<dyn CmdRunner> = Arc::new(RealCmdRunner);
            let http: Arc<dyn HttpClient> =
                Arc::new(ReqwestClient::new().expect("failed to build HTTP client"));

            app.manage(BackendState {
                config_dir,
                config: Mutex::new(config),
                runner,
                http,
            });

            // Kick off deferred detection, startup refresh, and the scheduler
            // tick — all off the UI thread.
            // UNVERIFIED: live path
            bridge::spawn_background_tasks(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bridge::get_config,
            bridge::get_onboarding_state,
            bridge::list_agents,
            bridge::recheck_agents,
            bridge::set_root_folder,
            bridge::set_default_agent,
            bridge::complete_onboarding,
            bridge::list_streams,
            bridge::get_stream,
            bridge::save_notes,
            bridge::set_stream_status,
            bridge::generate_first_draft,
            bridge::create_stream,
            bridge::refresh_stream,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
