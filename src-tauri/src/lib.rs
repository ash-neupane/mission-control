pub mod commands;
pub mod git;
pub mod naming;
pub mod notifications;
pub mod projects;
pub mod pty;
pub mod session;
pub mod status;

use commands::*;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(session::SessionManagerState::default())
        .manage(pty::PtyPoolState::default())
        .manage(projects::ProjectRegistryState::default())
        .manage(projects::ConfigState::default())
        .invoke_handler(tauri::generate_handler![
            create_session,
            kill_session,
            list_sessions,
            get_session,
            write_to_pty,
            resize_pty,
            list_projects,
            add_project,
            remove_project,
            create_branch,
            get_current_branch,
            get_config,
            update_config,
            open_url,
        ])
        .setup(|app| {
            // Spawn stuck detection background thread
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(30));

                    // Collect detectors under the pool lock, then release the lock
                    // before calling check_stuck (which needs the AppHandle).
                    let detectors: Vec<_> = {
                        let pool_state: tauri::State<pty::PtyPoolState> =
                            app_handle.state();
                        let pool = match pool_state.0.lock() {
                            Ok(p) => p,
                            Err(_) => continue,
                        };
                        pool.status_detectors
                            .values()
                            .cloned()
                            .collect()
                    };

                    for detector in &detectors {
                        if let Ok(mut det) = detector.lock() {
                            det.check_stuck(&app_handle);
                        }
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
