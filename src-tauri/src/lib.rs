//! V360Lab native backend.
//!
//! ```text
//! React UI --invoke--> commands --> camera::CameraClient --> virb::GarminVirb360Client --> VIRB HTTP API
//!                                                        \-> virb::MockVirb360Client (development)
//! ```

pub mod camera;
pub mod commands;
pub mod downloads;
pub mod error;
pub mod library;
pub mod settings;
pub mod state;
pub mod telemetry;
pub mod virb;

use tauri::Manager;

use settings::SettingsStore;
use state::AppState;

fn init_logging() {
    // RUST_LOG overrides the default filter.
    let env = env_logger::Env::default().default_filter_or("warn,v360lab_lib=debug");
    let _ = env_logger::Builder::from_env(env)
        .format_timestamp_millis()
        .try_init();
}

/// Debug builds always log at debug level; release builds only when the
/// user enables debug mode. An explicit RUST_LOG is left untouched.
pub fn apply_log_level(debug_mode: bool) {
    if std::env::var_os("RUST_LOG").is_some() {
        return;
    }
    let level = if cfg!(debug_assertions) || debug_mode {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    log::set_max_level(level);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = app.path();
            let settings_path = paths.app_config_dir()?.join("settings.json");
            let download_root = paths
                .download_dir()
                .or_else(|_| paths.home_dir())?
                .join("V360Lab");
            let settings = SettingsStore::load(settings_path.clone());
            apply_log_level(settings.get().debug_mode);
            log::info!(
                "V360Lab {} starting (settings: {})",
                env!("CARGO_PKG_VERSION"),
                settings_path.display()
            );
            app.manage(AppState::new(settings, download_root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_settings,
            commands::update_settings,
            commands::connect_camera,
            commands::disconnect_camera,
            commands::get_active_connection,
            commands::get_device_info,
            commands::get_camera_status,
            commands::get_camera_features,
            commands::start_recording,
            commands::stop_recording,
            commands::snap_picture,
            commands::stop_still_recording,
            commands::update_feature,
            commands::delete_media,
            commands::get_media_list,
            commands::fetch_thumbnail,
            commands::download_media,
            commands::download_fit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running V360Lab");
}
