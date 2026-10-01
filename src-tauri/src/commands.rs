//! Tauri commands: the only entry points the React UI can call.
//!
//! Commands are camera-agnostic: they operate on the connected
//! [`CameraClient`], whichever implementation it is.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use base64::Engine;
use serde::Serialize;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, State};

use crate::activity::{ActivityEvent, Step, ACTIVITY_EVENT};
use crate::camera::address::DEFAULT_CAMERA_ADDRESS;
use crate::camera::{
    CameraClient, CameraError, CameraKind, CameraStatus, CommandAck, DeviceInfo, FeatureList,
    MediaItem, SensorInfo, WifiNetworks, WifiSecurity,
};
use crate::discovery::{self, DiscoveredCamera};
use crate::downloads::{self, DownloadOptions, DownloadProgress, DownloadReport};
use crate::error::AppError;
use crate::library::{self, DeleteReport};
use crate::preview::{self, PreviewFailure};
use crate::settings::Settings;
use crate::state::AppState;
use crate::telemetry::{self, export::TrackFormat, summary::VideoTelemetry};
use crate::virb::{GarminVirb360Client, MockVirb360Client};
use crate::{geotag, wifi};

type CommandResult<T> = Result<T, AppError>;

/// Event emitted while files are being downloaded.
pub const DOWNLOAD_PROGRESS_EVENT: &str = "download-progress";
/// Thumbnails larger than this are not displayed.
const MAX_THUMBNAIL_BYTES: u64 = 2 * 1024 * 1024;

/// Emits the steps of a slow operation as `activity` events tagged with the
/// id chosen by the UI, which shows them as they happen.
fn activity_reporter(app: AppHandle, activity_id: String) -> impl Fn(Step) + Send + Sync {
    move |step| {
        log::debug!("[{activity_id}] {} {:?}", step.code, step.params);
        let event = ActivityEvent {
            activity_id: activity_id.clone(),
            step,
        };
        if let Err(e) = app.emit(ACTIVITY_EVENT, event) {
            log::debug!("Could not emit activity step: {e}");
        }
    }
}

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
pub fn update_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> CommandResult<SettingsView> {
    let saved = logged(
        "update_settings",
        state.settings.update(|current| *current = settings),
    )?;
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
    app: AppHandle,
    state: State<'_, AppState>,
    address: String,
    mock: bool,
    activity_id: String,
) -> CommandResult<ConnectionInfo> {
    let report = activity_reporter(app, activity_id);
    let result = async {
        let client: Arc<dyn CameraClient> = if mock {
            Arc::new(MockVirb360Client::new())
        } else {
            Arc::new(GarminVirb360Client::new(&address)?)
        };
        log::info!("Connecting to {}", client.address());
        report(Step::info("connectContacting").param("address", client.address()));
        let device_info = client.device_info().await.map_err(|e| match e {
            CameraError::UnsupportedCommand { .. }
            | CameraError::MalformedResponse { .. }
            | CameraError::Http { .. } => AppError::NotACamera {
                address: client.address(),
                source: e,
            },
            other => other.into(),
        })?;
        report(
            Step::info("connectIdentified")
                .param("model", device_info.model.clone().unwrap_or_default())
                .param("firmware", device_info.firmware.clone().unwrap_or_default()),
        );
        report(Step::info("connectReadingStatus"));
        let status = match client.status().await {
            Ok(status) => Some(status),
            Err(e) => {
                log::warn!("Connected, but status is unavailable: {e}");
                report(Step::warning("connectStatusUnavailable"));
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
        report(Step::success("connected").param("address", client.address()));
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

/// Looks for VIRB cameras: last used address, the camera's own Wi-Fi
/// address, then a scan of the local network. Progress is reported as
/// `activity` events.
#[tauri::command]
pub async fn discover_cameras(
    app: AppHandle,
    state: State<'_, AppState>,
    activity_id: String,
) -> CommandResult<Vec<DiscoveredCamera>> {
    let report = activity_reporter(app, activity_id);
    let mut candidates = Vec::new();
    candidates.extend(state.settings.get().last_camera_address);
    candidates.push(DEFAULT_CAMERA_ADDRESS.to_string());
    let found = discovery::discover(&candidates, &report).await;
    log::info!("Discovery found {} camera(s)", found.len());
    Ok(found)
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
pub async fn stop_still_recording(state: State<'_, AppState>) -> CommandResult<CommandAck> {
    let result = async { Ok(state.camera().await?.stop_still_recording().await?) }.await;
    logged("stop_still_recording", result)
}

/// Changes a camera feature (mode, lens format, photo mode, …) and returns
/// the updated feature list.
#[tauri::command]
pub async fn update_feature(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> CommandResult<FeatureList> {
    let result = async {
        log::info!("Setting camera feature {key} = {value}");
        Ok(state.camera().await?.update_feature(&key, &value).await?)
    }
    .await;
    logged("update_feature", result)
}

/// Deletes files on the camera and verifies the result against the media
/// list. Per-item failures are reported in the result, not as an error.
#[tauri::command]
pub async fn delete_media(
    app: AppHandle,
    state: State<'_, AppState>,
    items: Vec<MediaItem>,
    activity_id: String,
) -> CommandResult<DeleteReport> {
    let reporter = activity_reporter(app, activity_id);
    let result = async {
        let camera = state.camera().await?;
        let report = library::delete_media(camera.as_ref(), &items, &reporter).await;
        for failure in &report.failed {
            log::warn!("Could not delete {}: {}", failure.name, failure.error);
        }
        Ok(report)
    }
    .await;
    logged("delete_media", result)
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
        let resource = state
            .camera()
            .await?
            .fetch_resource(&url, MAX_THUMBNAIL_BYTES)
            .await?;
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
        downloads::download_media(
            camera.as_ref(),
            &state.download_root(),
            &item,
            options,
            &emit,
        )
        .await
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

/// Starts the live preview. Video is sent on `channel` as binary messages
/// (see [`crate::preview`]); returns once the stream is playing.
#[tauri::command]
pub async fn start_preview(
    state: State<'_, AppState>,
    channel: Channel<InvokeResponseBody>,
) -> CommandResult<()> {
    let result = async {
        let camera = state.camera().await?;
        let url = camera.live_preview_url().await?.ok_or(AppError::Preview {
            reason: PreviewFailure::Unsupported,
            detail: "this camera has no live preview".into(),
        })?;
        let sink: preview::Sink =
            Box::new(move |message| channel.send(InvokeResponseBody::Raw(message)).is_ok());
        state
            .preview
            .start(&url, sink, keyframe_requester(camera))
            .await
    }
    .await;
    logged("start_preview", result)
}

/// Sends `enableIDR` in the background; stops trying once the camera says
/// it does not support it.
fn keyframe_requester(camera: Arc<dyn CameraClient>) -> preview::KeyframeRequest {
    let unsupported = Arc::new(AtomicBool::new(false));
    Box::new(move || {
        if unsupported.load(Ordering::Relaxed) {
            return;
        }
        let camera = camera.clone();
        let unsupported = unsupported.clone();
        tauri::async_runtime::spawn(async move {
            match camera.request_keyframe().await {
                Ok(()) => log::debug!("Live preview: keyframe requested"),
                Err(
                    e
                    @ (CameraError::UnsupportedCommand { .. } | CameraError::CommandFailed { .. }),
                ) => {
                    log::info!("Live preview: the camera does not take keyframe requests ({e})");
                    unsupported.store(true, Ordering::Relaxed);
                }
                Err(e) => log::debug!("Live preview: keyframe request failed: {e}"),
            }
        });
    })
}

#[tauri::command]
pub async fn stop_preview(state: State<'_, AppState>) -> CommandResult<()> {
    state.preview.stop().await;
    Ok(())
}

/// Wi-Fi networks saved on and visible to the camera.
#[tauri::command]
pub async fn get_wifi_networks(state: State<'_, AppState>) -> CommandResult<WifiNetworks> {
    let result = async { Ok(state.camera().await?.wifi_networks().await?) }.await;
    logged("get_wifi_networks", result)
}

/// Saves a network on the camera. The camera keeps its current network
/// until [`connect_wifi_network`] is called.
#[tauri::command]
pub async fn add_wifi_network(
    state: State<'_, AppState>,
    ssid: String,
    security: WifiSecurity,
    password: String,
) -> CommandResult<CommandAck> {
    let result = async {
        let password = wifi::validate(&ssid, security, &password)?;
        log::info!(
            "Saving Wi-Fi network \"{ssid}\" ({}) on the camera",
            security.as_str()
        );
        Ok(state
            .camera()
            .await?
            .configure_wifi_network(&ssid, security, password)
            .await?)
    }
    .await;
    logged("add_wifi_network", result)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiSwitch {
    /// False when the camera dropped the connection before answering: it
    /// has most likely started switching anyway.
    confirmed: bool,
}

/// Makes the camera join a saved network. The camera leaves its current
/// network, so the connection is closed: the user reconnects once this
/// device is on the same network.
#[tauri::command]
pub async fn connect_wifi_network(
    state: State<'_, AppState>,
    ssid: String,
) -> CommandResult<WifiSwitch> {
    let result = async {
        let camera = state.camera().await?;
        log::info!("Asking the camera to join Wi-Fi network \"{ssid}\"");
        let confirmed = match camera.connect_wifi_network(&ssid).await {
            Ok(_) => true,
            Err(
                e @ (CameraError::Unreachable { .. }
                | CameraError::Timeout { .. }
                | CameraError::Transfer { .. }),
            ) => {
                log::warn!("No answer to connectNetwork, the camera is probably switching: {e}");
                false
            }
            Err(e) => return Err(e.into()),
        };
        state.set_camera(None).await;
        Ok(WifiSwitch { confirmed })
    }
    .await;
    logged("connect_wifi_network", result)
}

#[tauri::command]
pub async fn remove_wifi_network(
    state: State<'_, AppState>,
    ssid: String,
) -> CommandResult<CommandAck> {
    let result = async {
        log::info!("Removing Wi-Fi network \"{ssid}\" from the camera");
        Ok(state.camera().await?.remove_wifi_network(&ssid).await?)
    }
    .await;
    logged("remove_wifi_network", result)
}

/// Commands the camera supports, or `None` when it cannot tell.
#[tauri::command]
pub async fn get_supported_commands(
    state: State<'_, AppState>,
) -> CommandResult<Option<Vec<String>>> {
    let result = async { Ok(state.camera().await?.supported_commands().await?) }.await;
    logged("get_supported_commands", result)
}

/// Starts (`on`) or stops the camera's locate signal (sound and lights).
#[tauri::command]
pub async fn locate_camera(state: State<'_, AppState>, on: bool) -> CommandResult<CommandAck> {
    let result = async { Ok(state.camera().await?.locate(on).await?) }.await;
    logged("locate_camera", result)
}

/// Sensors paired with the camera.
#[tauri::command]
pub async fn get_sensors(state: State<'_, AppState>) -> CommandResult<Vec<SensorInfo>> {
    let result = async { Ok(state.camera().await?.sensors().await?) }.await;
    logged("get_sensors", result)
}

/// Media folders on the camera's card.
#[tauri::command]
pub async fn get_media_directories(state: State<'_, AppState>) -> CommandResult<Vec<String>> {
    let result = async { Ok(state.camera().await?.media_directories().await?) }.await;
    logged("get_media_directories", result)
}

/// Puts the camera in standby and closes the connection: the camera no
/// longer answers until it is woken up on the camera itself.
#[tauri::command]
pub async fn standby_camera(state: State<'_, AppState>) -> CommandResult<CommandAck> {
    let result = async {
        let camera = state.camera().await?;
        log::info!("Putting the camera in standby");
        state.preview.stop().await;
        let ack = camera.standby().await?;
        state.set_camera(None).await;
        Ok(ack)
    }
    .await;
    logged("standby_camera", result)
}

/// Marks a media item as favourite (or not) and returns it as the camera
/// now lists it.
#[tauri::command]
pub async fn set_media_favorite(
    state: State<'_, AppState>,
    item: MediaItem,
    favorite: bool,
) -> CommandResult<MediaItem> {
    let result = async {
        let camera = state.camera().await?;
        let url = item
            .url
            .as_deref()
            .ok_or_else(|| AppError::MissingResource {
                name: item.name.clone(),
                resource: crate::error::Resource::DownloadUrl,
            })?;
        camera.set_favorite(url, favorite).await?;
        let updated = camera
            .media_list()
            .await?
            .into_iter()
            .find(|m| m.id == item.id)
            .ok_or_else(|| CameraError::CommandFailed {
                command: "setFavorite".into(),
                response: format!("\"{}\" is no longer in the media list", item.name),
            })?;
        if updated.favorite != Some(favorite) {
            return Err(CameraError::CommandFailed {
                command: "setFavorite".into(),
                response: format!("\"{}\" is still {:?}", item.name, updated.favorite),
            }
            .into());
        }
        Ok(updated)
    }
    .await;
    logged("set_media_favorite", result)
}

/// Largest FIT file read into memory (an hour of VIRB telemetry is a few MB).
const MAX_FIT_BYTES: u64 = 64 * 1024 * 1024;

/// Downloads and decodes the FIT file of a video.
async fn video_telemetry(
    state: &AppState,
    item: &MediaItem,
    max_points: usize,
) -> CommandResult<VideoTelemetry> {
    let url = item
        .fit_url
        .as_deref()
        .ok_or_else(|| AppError::MissingResource {
            name: item.name.clone(),
            resource: crate::error::Resource::FitFile,
        })?;
    let fit = state
        .camera()
        .await?
        .fetch_resource(url, MAX_FIT_BYTES)
        .await?;
    let track = telemetry::decode::decode(&fit.bytes).map_err(|e| AppError::Telemetry {
        detail: e.to_string(),
    })?;
    log::info!(
        "Telemetry of {}: {} samples, {} camera events",
        item.name,
        track.samples.len(),
        track.camera_events.len()
    );
    Ok(telemetry::summary::for_video(
        track,
        item.timestamp,
        item.duration_secs,
        max_points,
    ))
}

/// The GPS track recorded with a video, aligned with its timeline.
#[tauri::command]
pub async fn get_media_telemetry(
    state: State<'_, AppState>,
    item: MediaItem,
) -> CommandResult<VideoTelemetry> {
    let result = video_telemetry(&state, &item, telemetry::summary::MAX_POINTS).await;
    logged("get_media_telemetry", result)
}

/// Writes the full-resolution GPS track of a video as GPX or GeoJSON next
/// to its downloads, and returns the file path.
#[tauri::command]
pub async fn export_track(
    state: State<'_, AppState>,
    item: MediaItem,
    format: TrackFormat,
) -> CommandResult<String> {
    let result = async {
        let telemetry = video_telemetry(&state, &item, usize::MAX).await?;
        if !telemetry.summary.has_position {
            return Err(AppError::Telemetry {
                detail: "the FIT file has no GPS positions for this video".into(),
            });
        }
        let content = match format {
            TrackFormat::Gpx => telemetry::export::to_gpx(&telemetry, &item.name),
            TrackFormat::Geojson => {
                serde_json::to_string_pretty(&telemetry::export::to_geojson(&telemetry, &item.name))
                    .unwrap_or_default()
            }
        };
        let directory = downloads::item_directory(&state.download_root(), &item);
        tokio::fs::create_dir_all(&directory)
            .await
            .map_err(AppError::fs(&directory))?;
        let stem = directory
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "track".into());
        let path = directory.join(format!("{stem}.{}", format.extension()));
        tokio::fs::write(&path, content)
            .await
            .map_err(AppError::fs(&path))?;
        log::info!("Track of {} exported to {}", item.name, path.display());
        Ok(path.display().to_string())
    }
    .await;
    logged("export_track", result)
}

/// Header of [`save_frame`]: which video the frame comes from and where it
/// was taken (percent-encoded JSON, since headers are ASCII).
const FRAME_HEADER: &str = "x-v360lab-frame";

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameRequest {
    /// Media name and date, to find the video's download folder.
    name: String,
    timestamp: Option<i64>,
    index: u32,
    location: geotag::FrameLocation,
    /// Equirectangular 360° frame: adds the GPano panorama tags.
    spherical: bool,
    camera_model: Option<String>,
}

fn frames_directory(state: &AppState, name: &str, timestamp: Option<i64>) -> std::path::PathBuf {
    downloads::media_directory(&state.download_root(), name, timestamp).join("frames")
}

/// Saves one extracted video frame (the request body is the JPEG image)
/// with EXIF GPS and, for 360° frames, GPano tags. Returns the file path.
#[tauri::command]
pub async fn save_frame(
    state: State<'_, AppState>,
    request: tauri::ipc::Request<'_>,
) -> CommandResult<String> {
    let result = async {
        let invalid = |detail: &str| AppError::Telemetry {
            detail: format!("invalid frame: {detail}"),
        };
        let tauri::ipc::InvokeBody::Raw(jpeg) = request.body() else {
            return Err(invalid("the body is not an image"));
        };
        let header = request
            .headers()
            .get(FRAME_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| invalid("missing frame description"))?;
        let header = percent_encoding::percent_decode_str(header)
            .decode_utf8()
            .map_err(|_| invalid("frame description is not UTF-8"))?;
        let frame: FrameRequest =
            serde_json::from_str(&header).map_err(|e| invalid(&e.to_string()))?;
        let tagged = geotag::tag_jpeg(
            jpeg,
            &frame.location,
            frame.camera_model.as_deref().unwrap_or("VIRB 360"),
            frame.spherical,
        )
        .map_err(|e| invalid(&e.to_string()))?;

        let directory = frames_directory(&state, &frame.name, frame.timestamp);
        tokio::fs::create_dir_all(&directory)
            .await
            .map_err(AppError::fs(&directory))?;
        let stem = downloads::media_directory(std::path::Path::new(""), &frame.name, None)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "frame".into());
        let path = directory.join(format!("{stem}_{:05}.jpg", frame.index));
        tokio::fs::write(&path, tagged)
            .await
            .map_err(AppError::fs(&path))?;
        Ok(path.display().to_string())
    }
    .await;
    logged("save_frame", result)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameIndexEntry {
    file: String,
    /// Position in the video, in seconds.
    video_seconds: f64,
    location: geotag::FrameLocation,
}

/// Writes `frames.geojson` next to the extracted frames: one point per
/// frame with its file name, time and position in the video.
#[tauri::command]
pub async fn write_frames_index(
    state: State<'_, AppState>,
    item: MediaItem,
    frames: Vec<FrameIndexEntry>,
) -> CommandResult<String> {
    let result = async {
        let features: Vec<serde_json::Value> = frames
            .iter()
            .map(|frame| {
                let l = &frame.location;
                let mut coordinates = vec![l.longitude, l.latitude];
                coordinates.extend(l.altitude_m);
                serde_json::json!({
                    "type": "Feature",
                    "geometry": { "type": "Point", "coordinates": coordinates },
                    "properties": {
                        "file": std::path::Path::new(&frame.file)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned()),
                        "time": chrono::DateTime::<chrono::Utc>::from_timestamp_millis(l.unix_ms)
                            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
                        "videoSeconds": frame.video_seconds,
                        "headingDeg": l.heading_deg,
                        "speedMps": l.speed_mps,
                    }
                })
            })
            .collect();
        let geojson = serde_json::json!({
            "type": "FeatureCollection",
            "features": features,
            "properties": { "video": item.name, "creator": "V360Lab" },
        });
        let directory = frames_directory(&state, &item.name, item.timestamp);
        tokio::fs::create_dir_all(&directory)
            .await
            .map_err(AppError::fs(&directory))?;
        let path = directory.join("frames.geojson");
        tokio::fs::write(
            &path,
            serde_json::to_string_pretty(&geojson).unwrap_or_default(),
        )
        .await
        .map_err(AppError::fs(&path))?;
        log::info!(
            "{} frames of {} indexed in {}",
            frames.len(),
            item.name,
            path.display()
        );
        Ok(path.display().to_string())
    }
    .await;
    logged("write_frames_index", result)
}
