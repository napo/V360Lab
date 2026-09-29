//! Application-level errors returned by Tauri commands.
//!
//! Errors reach the frontend as `{ kind, message, detail }`: `message` is
//! readable text for the user, `detail` holds technical information that the
//! UI shows only in debug mode.

use std::path::PathBuf;

use serde::{Serialize, Serializer};
use thiserror::Error;

use crate::camera::CameraError;

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

    #[error("\"{name}\" has no {what} available on the camera")]
    MissingResource { name: String, what: &'static str },

    #[error("Could not access {}", path.display())]
    Filesystem {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid settings: {0}")]
    Settings(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Camera(e) => e.kind(),
            Self::NotConnected => "notConnected",
            Self::NotACamera { .. } => "notACamera",
            Self::MissingResource { .. } => "missingResource",
            Self::Filesystem { .. } => "filesystem",
            Self::Settings(_) => "settings",
        }
    }

    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Camera(e) => e.detail(),
            Self::NotACamera { source, .. } => {
                Some(format!("{source}{}", source.detail().map(|d| format!(": {d}")).unwrap_or_default()))
            }
            Self::Filesystem { source, .. } => Some(source.to_string()),
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
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorPayload {
            kind: self.kind(),
            message: self.to_string(),
            detail: self.detail(),
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
    }
}
