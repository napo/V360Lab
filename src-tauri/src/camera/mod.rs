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
    async fn media_list(&self) -> Result<Vec<MediaItem>, CameraError>;

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
