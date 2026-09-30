//! Camera-agnostic abstractions.
//!
//! The rest of the backend (Tauri commands, downloads) talks to a camera only
//! through [`CameraClient`]. Garmin VIRB specifics (including the mock camera
//! used for UI development) live in [`crate::virb`].
//!
//! Supporting another 360 camera means adding a new `CameraClient`
//! implementation that maps its protocol onto the models in [`models`].

pub mod address;
pub mod error;
pub mod models;

use std::path::Path;

use async_trait::async_trait;

pub use error::CameraError;
pub use models::*;

/// Progress callback for transfers: `(received_bytes, total_bytes_if_known)`.
pub type ProgressFn<'a> = &'a (dyn Fn(u64, Option<u64>) + Send + Sync);

#[async_trait]
pub trait CameraClient: Send + Sync {
    /// Which implementation this is (used by the UI and in metadata files).
    fn kind(&self) -> CameraKind;

    /// Human-readable address of the device, e.g. `http://192.168.0.1/`.
    fn address(&self) -> String;

    async fn device_info(&self) -> Result<DeviceInfo, CameraError>;
    async fn status(&self) -> Result<CameraStatus, CameraError>;
    async fn features(&self) -> Result<FeatureList, CameraError>;
    async fn start_recording(&self) -> Result<CommandAck, CameraError>;
    async fn stop_recording(&self) -> Result<CommandAck, CameraError>;
    async fn snap_picture(&self) -> Result<CommandAck, CameraError>;
    /// Ends a still capture sequence (photo time-lapse, burst).
    async fn stop_still_recording(&self) -> Result<CommandAck, CameraError>;
    /// Sets a camera feature and returns the updated feature list.
    /// Fails if the camera does not report the requested value afterwards.
    async fn update_feature(&self, key: &str, value: &str) -> Result<FeatureList, CameraError>;
    /// Deletes files on the camera in one request. `media_urls` are URLs
    /// exactly as reported by the media list. A success response does not
    /// guarantee anything was deleted; callers verify with the media list.
    async fn delete_files(&self, media_urls: &[String]) -> Result<CommandAck, CameraError>;
    async fn media_list(&self) -> Result<Vec<MediaItem>, CameraError>;

    /// Starts the camera's live preview and returns its RTSP URL, or `None`
    /// when this camera has no live preview.
    async fn live_preview_url(&self) -> Result<Option<String>, CameraError> {
        Ok(None)
    }

    /// Wi-Fi networks saved on and visible to the camera, plus the name of
    /// the camera's own network.
    async fn wifi_networks(&self) -> Result<WifiNetworks, CameraError> {
        Err(unsupported("networks"))
    }

    /// Saves a network on the camera without switching to it.
    async fn configure_wifi_network(
        &self,
        _ssid: &str,
        _security: WifiSecurity,
        _password: &str,
    ) -> Result<CommandAck, CameraError> {
        Err(unsupported("configureNetwork"))
    }

    /// Asks the camera to leave its own network and join a saved one. The
    /// camera becomes unreachable at its current address afterwards.
    async fn connect_wifi_network(&self, _ssid: &str) -> Result<CommandAck, CameraError> {
        Err(unsupported("connectNetwork"))
    }

    /// Removes a saved network from the camera.
    async fn remove_wifi_network(&self, _ssid: &str) -> Result<CommandAck, CameraError> {
        Err(unsupported("removeNetwork"))
    }

    /// Fetches a small resource (e.g. a thumbnail) into memory.
    /// Fails with [`CameraError::TooLarge`] above `max_bytes`.
    async fn fetch_resource(
        &self,
        url: &str,
        max_bytes: u64,
    ) -> Result<FetchedResource, CameraError>;

    /// Streams a camera resource to `destination`, returning the byte count.
    /// Implementations must not leave a partial file at `destination`.
    async fn download_to(
        &self,
        url: &str,
        destination: &Path,
        progress: ProgressFn<'_>,
    ) -> Result<u64, CameraError>;
}

fn unsupported(command: &str) -> CameraError {
    CameraError::UnsupportedCommand {
        command: command.to_string(),
        detail: "not implemented for this camera".to_string(),
    }
}
