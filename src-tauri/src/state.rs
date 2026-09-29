//! Shared backend state managed by Tauri.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::camera::CameraClient;
use crate::error::AppError;
use crate::settings::SettingsStore;

pub struct AppState {
    pub settings: SettingsStore,
    /// Used when no download directory is configured.
    pub default_download_root: PathBuf,
    camera: RwLock<Option<Arc<dyn CameraClient>>>,
}

impl AppState {
    pub fn new(settings: SettingsStore, default_download_root: PathBuf) -> Self {
        Self {
            settings,
            default_download_root,
            camera: RwLock::new(None),
        }
    }

    /// The connected camera. The lock is released immediately, so slow
    /// camera requests never block other commands.
    pub async fn camera(&self) -> Result<Arc<dyn CameraClient>, AppError> {
        self.camera.read().await.clone().ok_or(AppError::NotConnected)
    }

    pub async fn set_camera(&self, camera: Option<Arc<dyn CameraClient>>) {
        *self.camera.write().await = camera;
    }

    pub fn download_root(&self) -> PathBuf {
        self.settings
            .get()
            .download_directory
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_download_root.clone())
    }
}
