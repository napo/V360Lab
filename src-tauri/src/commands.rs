//! Tauri commands: the only entry points the React UI can call.
//!
//! Commands are camera-agnostic: they operate on the connected
//! [`CameraClient`], whichever implementation it is.

use std::sync::Arc;

use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::camera::address::DEFAULT_CAMERA_ADDRESS;
use crate::camera::{
    CameraClient, CameraError, CameraKind, CameraStatus, CommandAck, DeviceInfo, FeatureList, MediaItem,
};
use crate::downloads::{self, DownloadOptions, DownloadProgress, DownloadReport};
use crate::error::AppError;
use crate::settings::Settings;
use crate::state::AppState;
use crate::virb::{GarminVirb360Client, MockVirb360Client};

type CommandResult<T> = Result<T, AppError>;

/// Event emitted while files are being downloaded.
pub const DOWNLOAD_PROGRESS_EVENT: &str = "download-progress";
/// Thumbnails larger than this are not displayed.
const MAX_THUMBNAIL_BYTES: u64 = 2 * 1024 * 1024;

/// Logs failures with technical details before they are sent to the UI.
fn logged<T>(context: &str, result: CommandResult<T>) -> CommandResult<T> {
    if let Err(e) = &result {
        match e.detail() {
            Some(detail) => log::warn!("{context} failed [{}]: {e} ({detail})", e.kind()),
            None => log::warn!("{context} failed [{}]: {e}", e.kind()),
        }
    }
    result
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    name: &'static str,
    version: &'static str,
    debug_build: bool,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    log::debug!("Frontend requested app_info");
    AppInfo {
        name: "V360Lab",
        version: env!("CARGO_PKG_VERSION"),
        debug_build: cfg!(debug_assertions),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    settings: Settings,
    /// Download root actually in use (configured or default).
    effective_download_directory: String,
    default_camera_address: &'static str,
}

fn settings_view(state: &AppState) -> SettingsView {
    SettingsView {
        settings: state.settings.get(),
        effective_download_directory: state.download_root().display().to_string(),
        default_camera_address: DEFAULT_CAMERA_ADDRESS,
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> SettingsView {
    log::debug!("Frontend requested settings");
    settings_view(&state)
}

#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, settings: Settings) -> CommandResult<SettingsView> {
    let saved = logged("update_settings", state.settings.update(|current| *current = settings))?;
    crate::apply_log_level(saved.debug_mode);
    Ok(settings_view(&state))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    kind: CameraKind,
    address: String,
    device_info: DeviceInfo,
    status: Option<CameraStatus>,
}

/// Connects to a camera: verifies it is reachable by requesting its device
/// information, reads its status, then keeps the client for later commands.
#[tauri::command]
pub async fn connect_camera(
    state: State<'_, AppState>,
    address: String,
    mock: bool,
) -> CommandResult<ConnectionInfo> {
    let result = async {
        let client: Arc<dyn CameraClient> = if mock {
            Arc::new(MockVirb360Client::new())
        } else {
            Arc::new(GarminVirb360Client::new(&address)?)
        };
        log::info!("Connecting to {}", client.address());
        let device_info = client.device_info().await.map_err(|e| match e {
            CameraError::UnsupportedCommand { .. }
            | CameraError::MalformedResponse { .. }
            | CameraError::Http { .. } => AppError::NotACamera {
                address: client.address(),
                source: e,
            },
            other => other.into(),
        })?;
        let status = match client.status().await {
            Ok(status) => Some(status),
            Err(e) => {
                log::warn!("Connected, but status is unavailable: {e}");
                None
            }
        };
        state.set_camera(Some(client.clone())).await;
        let persisted = state.settings.update(|s| {
            s.mock_mode = mock;
            if !mock {
                s.last_camera_address = Some(address.trim().to_string());
            }
        });
        if let Err(e) = persisted {
            log::warn!("Could not persist camera address: {e}");
        }
        Ok(ConnectionInfo {
            kind: client.kind(),
            address: client.address(),
            device_info,
            status,
        })
    }
    .await;
    logged("connect_camera", result)
}

#[tauri::command]
pub async fn disconnect_camera(state: State<'_, AppState>) -> CommandResult<()> {
    state.set_camera(None).await;
    log::info!("Disconnected");
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveConnection {
    kind: CameraKind,
    address: String,
}

/// The backend keeps the connection across frontend reloads (e.g. during
/// development); the UI uses this to restore its state.
#[tauri::command]
pub async fn get_active_connection(
    state: State<'_, AppState>,
) -> CommandResult<Option<ActiveConnection>> {
    Ok(state.camera().await.ok().map(|c| ActiveConnection {
        kind: c.kind(),
        address: c.address(),
    }))
}

#[tauri::command]
pub async fn get_device_info(state: State<'_, AppState>) -> CommandResult<DeviceInfo> {
    let result = async { Ok(state.camera().await?.device_info().await?) }.await;
    logged("get_device_info", result)
}

#[tauri::command]
pub async fn get_camera_status(state: State<'_, AppState>) -> CommandResult<CameraStatus> {
    let result = async { Ok(state.camera().await?.status().await?) }.await;
    logged("get_camera_status", result)
}

#[tauri::command]
pub async fn get_camera_features(state: State<'_, AppState>) -> CommandResult<FeatureList> {
    let result = async { Ok(state.camera().await?.features().await?) }.await;
    logged("get_camera_features", result)
}

#[tauri::command]
pub async fn start_recording(state: State<'_, AppState>) -> CommandResult<CommandAck> {
    let result = async { Ok(state.camera().await?.start_recording().await?) }.await;
    logged("start_recording", result)
}

#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> CommandResult<CommandAck> {
    let result = async { Ok(state.camera().await?.stop_recording().await?) }.await;
    logged("stop_recording", result)
}

#[tauri::command]
pub async fn snap_picture(state: State<'_, AppState>) -> CommandResult<CommandAck> {
    let result = async { Ok(state.camera().await?.snap_picture().await?) }.await;
    logged("snap_picture", result)
}

#[tauri::command]
pub async fn get_media_list(state: State<'_, AppState>) -> CommandResult<Vec<MediaItem>> {
    let result = async { Ok(state.camera().await?.media_list().await?) }.await;
    logged("get_media_list", result)
}

/// Fetches a thumbnail through the backend and returns it as a data URL, so
/// the UI never talks to the camera directly.
#[tauri::command]
pub async fn fetch_thumbnail(state: State<'_, AppState>, url: String) -> CommandResult<String> {
    let result = async {
        let resource = state.camera().await?.fetch_resource(&url, MAX_THUMBNAIL_BYTES).await?;
        let content_type = resource
            .content_type
            .filter(|t| t.starts_with("image/"))
            .unwrap_or_else(|| "image/jpeg".to_string());
        let encoded = base64::engine::general_purpose::STANDARD.encode(&resource.bytes);
        Ok(format!("data:{content_type};base64,{encoded}"))
    }
    .await;
    logged("fetch_thumbnail", result)
}

fn progress_emitter(app: AppHandle) -> impl Fn(DownloadProgress) + Send + Sync {
    move |progress| {
        if let Err(e) = app.emit(DOWNLOAD_PROGRESS_EVENT, progress) {
            log::debug!("Could not emit download progress: {e}");
        }
    }
}

#[tauri::command]
pub async fn download_media(
    app: AppHandle,
    state: State<'_, AppState>,
    item: MediaItem,
    include_fit: bool,
    include_thumbnail: bool,
) -> CommandResult<DownloadReport> {
    let result = async {
        let camera = state.camera().await?;
        let emit = progress_emitter(app);
        let options = DownloadOptions {
            include_fit,
            include_thumbnail,
        };
        downloads::download_media(camera.as_ref(), &state.download_root(), &item, options, &emit).await
    }
    .await;
    logged("download_media", result)
}

#[tauri::command]
pub async fn download_fit(
    app: AppHandle,
    state: State<'_, AppState>,
    item: MediaItem,
) -> CommandResult<DownloadReport> {
    let result = async {
        let camera = state.camera().await?;
        let emit = progress_emitter(app);
        downloads::download_fit(camera.as_ref(), &state.download_root(), &item, &emit).await
    }
    .await;
    logged("download_fit", result)
}
