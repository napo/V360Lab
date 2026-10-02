//! Application-level errors returned by Tauri commands.
//!
//! Errors reach the frontend as `{ kind, message, detail, params }`. The UI
//! translates `kind` (plus `params`) into the user's language and falls back
//! to the English `message`; `detail` holds technical information that the
//! UI shows only in debug mode.

use std::fmt;
use std::path::PathBuf;

use serde::{Serialize, Serializer};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::camera::CameraError;
use crate::preview::PreviewFailure;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Camera(#[from] CameraError),

    #[error("No camera is connected")]
    NotConnected,

    /// A web server answered, but not like a VIRB (e.g. a router's page).
    #[error("The device at {address} does not respond like a Garmin VIRB camera")]
    NotACamera {
        address: String,
        #[source]
        source: CameraError,
    },

    #[error("\"{name}\" has no {resource} available on the camera")]
    MissingResource { name: String, resource: Resource },

    /// The camera acknowledged a deletion but still lists the file.
    #[error("The camera acknowledged the deletion, but \"{name}\" is still there")]
    NotDeleted { name: String },

    #[error("Could not access {}", path.display())]
    Filesystem {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The live preview could not start or stopped.
    #[error("Live preview failed: {detail}")]
    Preview {
        reason: PreviewFailure,
        detail: String,
    },

    /// `reason` is a stable code the UI translates.
    #[error("Invalid settings: {message}")]
    Settings {
        reason: &'static str,
        message: String,
    },

    /// Object detection could not run.
    #[error("{0}")]
    Detection(#[from] crate::detect::DetectError),

    /// A detection model could not be downloaded. `reason` is a stable code
    /// the UI translates (`network`, `sizeMismatch`, `checksum`, `noSpace`).
    #[error("Could not download the model: {detail}")]
    ModelDownload {
        reason: &'static str,
        detail: String,
    },

    /// A FIT file that could not be decoded.
    #[error("Could not read the telemetry: {detail}")]
    Telemetry { detail: String },

    /// Wi-Fi network details the camera would not accept.
    /// `reason` is a stable code the UI translates.
    #[error("Invalid Wi-Fi network: {message}")]
    Wifi {
        reason: &'static str,
        message: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    DownloadUrl,
    FitFile,
}

impl Resource {
    pub fn code(self) -> &'static str {
        match self {
            Self::DownloadUrl => "downloadUrl",
            Self::FitFile => "fitFile",
        }
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DownloadUrl => "download URL",
            Self::FitFile => "FIT telemetry file",
        })
    }
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Camera(e) => e.kind(),
            Self::NotConnected => "notConnected",
            Self::NotACamera { .. } => "notACamera",
            Self::MissingResource { .. } => "missingResource",
            Self::NotDeleted { .. } => "notDeleted",
            Self::Filesystem { .. } => "filesystem",
            Self::Settings { .. } => "settings",
            Self::Preview { .. } => "preview",
            Self::Wifi { .. } => "wifi",
            Self::Telemetry { .. } => "telemetry",
            Self::ModelDownload { .. } => "modelDownload",
            Self::Detection(e) => match e {
                crate::detect::DetectError::NoFrames => "detectionNoFrames",
                crate::detect::DetectError::Model(_) => "detectionModel",
                _ => "detection",
            },
        }
    }

    pub fn params(&self) -> Map<String, Value> {
        let mut params = Map::new();
        match self {
            Self::Camera(e) => return e.params(),
            Self::NotConnected | Self::Telemetry { .. } | Self::Detection(_) => {}
            Self::NotACamera { address, .. } => {
                params.insert("address".into(), address.as_str().into());
            }
            Self::MissingResource { name, resource } => {
                params.insert("name".into(), name.as_str().into());
                params.insert("resource".into(), resource.code().into());
            }
            Self::NotDeleted { name } => {
                params.insert("name".into(), name.as_str().into());
            }
            Self::Filesystem { path, .. } => {
                params.insert("path".into(), path.display().to_string().into());
            }
            Self::Settings { reason, .. }
            | Self::Wifi { reason, .. }
            | Self::ModelDownload { reason, .. } => {
                params.insert("reason".into(), (*reason).into());
            }
            Self::Preview { reason, .. } => {
                params.insert("reason".into(), reason.code().into());
            }
        }
        params
    }

    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Camera(e) => e.detail(),
            Self::NotACamera { source, .. } => Some(format!(
                "{source}{}",
                source
                    .detail()
                    .map(|d| format!(": {d}"))
                    .unwrap_or_default()
            )),
            Self::Filesystem { source, .. } => Some(source.to_string()),
            Self::Preview { detail, .. } | Self::Telemetry { detail } => Some(detail.clone()),
            Self::Detection(e) => Some(e.to_string()),
            Self::ModelDownload { detail, .. } => Some(detail.clone()),
            _ => None,
        }
    }

    /// Helper for `map_err`: wraps an I/O error with the path involved.
    pub fn fs(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> Self {
        let path = path.into();
        move |source| Self::Filesystem { path, source }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload<'a> {
    kind: &'a str,
    message: String,
    detail: Option<String>,
    params: Map<String, Value>,
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorPayload {
            kind: self.kind(),
            message: self.to_string(),
            detail: self.detail(),
            params: self.params(),
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_kind_message_and_detail() {
        let err = AppError::from(CameraError::Timeout {
            timeout_secs: 10,
            detail: "operation timed out".into(),
        });
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "timeout");
        assert_eq!(json["message"], "Camera did not respond within 10 s");
        assert_eq!(json["detail"], "operation timed out");
        assert_eq!(json["params"]["timeoutSecs"], 10);
    }

    #[test]
    fn missing_resource_params_use_codes() {
        let err = AppError::MissingResource {
            name: "V0010042.MP4".into(),
            resource: Resource::FitFile,
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "missingResource");
        assert_eq!(json["params"]["resource"], "fitFile");
        assert_eq!(json["params"]["name"], "V0010042.MP4");
    }
}
