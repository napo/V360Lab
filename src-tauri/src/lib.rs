//! V360Lab native backend.
//!
//! ```text
//! React UI --invoke--> commands --> camera::CameraClient --> virb::GarminVirb360Client --> VIRB HTTP API
//!                                                        \-> virb::MockVirb360Client (development)
//! ```

pub mod activity;
pub mod camera;
pub mod commands;
pub mod detect;
pub mod discovery;
pub mod downloads;
pub mod error;
pub mod geotag;
pub mod library;
pub mod media_protocol;
pub mod models;
pub mod preview;
pub mod settings;
pub mod state;
pub mod telemetry;
pub mod updates;
pub mod virb;
pub mod wifi;

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

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());
    // Self-update (signed, from the GitHub release) on desktop only.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    builder
        .register_asynchronous_uri_scheme_protocol(
            media_protocol::SCHEME,
            |ctx, request, responder| {
                let app = ctx.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    responder.respond(media_protocol::handle(&app, request).await);
                });
            },
        )
        .setup(|app| {
            let paths = app.path();
            let settings_path = paths.app_config_dir()?.join("settings.json");
            // Android has no accessible Downloads/home directory for apps:
            // fall back to the app's own data directory.
            let download_root = paths
                .download_dir()
                .or_else(|_| paths.home_dir())
                .or_else(|_| paths.app_data_dir())?
                .join("V360Lab");
            let settings = SettingsStore::load(settings_path.clone());
            apply_log_level(settings.get().debug_mode);
            log::info!(
                "V360Lab {} starting (settings: {})",
                env!("CARGO_PKG_VERSION"),
                settings_path.display()
            );
            let models_dir = paths.app_data_dir()?.join("models");
            app.manage(AppState::new(settings, download_root, models_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_settings,
            commands::update_settings,
            commands::connect_camera,
            commands::discover_cameras,
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
            commands::start_preview,
            commands::stop_preview,
            commands::get_wifi_networks,
            commands::add_wifi_network,
            commands::connect_wifi_network,
            commands::remove_wifi_network,
            commands::get_supported_commands,
            commands::locate_camera,
            commands::get_sensors,
            commands::get_media_directories,
            commands::standby_camera,
            commands::set_media_favorite,
            commands::get_media_telemetry,
            commands::export_track,
            commands::save_frame,
            commands::write_frames_index,
            commands::detect_objects,
            commands::cancel_detection,
            commands::benchmark_detection,
            commands::check_for_update,
            commands::list_models,
            commands::check_model_download,
            commands::download_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running V360Lab");
}
