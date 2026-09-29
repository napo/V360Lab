//! Typed errors for camera communication.

use std::path::PathBuf;

use serde_json::{Map, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CameraError {
    #[error("Invalid camera address \"{address}\": {reason}")]
    InvalidAddress { address: String, reason: String },

    #[error("Camera at {address} is unreachable")]
    Unreachable { address: String, detail: String },

    #[error("Camera did not respond within {timeout_secs} s")]
    Timeout { timeout_secs: u64, detail: String },

    #[error("Camera returned HTTP status {status}")]
    Http { status: u16, body: String },

    #[error("Camera returned a malformed response to \"{command}\"")]
    MalformedResponse {
        command: String,
        detail: String,
        snippet: String,
    },

    #[error("Command \"{command}\" is not supported by this camera or firmware")]
    UnsupportedCommand { command: String, detail: String },

    #[error("Camera rejected command \"{command}\"")]
    CommandFailed { command: String, response: String },

    #[error("Invalid media URL reported by the camera")]
    InvalidUrl { url: String, reason: String },

    #[error("Resource is too large ({size} bytes, limit {limit} bytes)")]
    TooLarge { size: u64, limit: u64 },

    #[error("Transfer from the camera was interrupted")]
    Transfer { url: String, detail: String },

    #[error("Could not write {}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl CameraError {
    /// Stable machine-readable identifier used by the frontend.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidAddress { .. } => "invalidAddress",
            Self::Unreachable { .. } => "unreachable",
            Self::Timeout { .. } => "timeout",
            Self::Http { .. } => "http",
            Self::MalformedResponse { .. } => "malformedResponse",
            Self::UnsupportedCommand { .. } => "unsupportedCommand",
            Self::CommandFailed { .. } => "commandFailed",
            Self::InvalidUrl { .. } => "invalidUrl",
            Self::TooLarge { .. } => "tooLarge",
            Self::Transfer { .. } => "transfer",
            Self::Io { .. } => "filesystem",
        }
    }

    /// Values the UI interpolates into its translated message.
    pub fn params(&self) -> Map<String, Value> {
        let pairs: Vec<(&str, Value)> = match self {
            Self::InvalidAddress { address, .. } | Self::Unreachable { address, .. } => {
                vec![("address", address.as_str().into())]
            }
            Self::Timeout { timeout_secs, .. } => vec![("timeoutSecs", (*timeout_secs).into())],
            Self::Http { status, .. } => vec![("status", (*status).into())],
            Self::MalformedResponse { command, .. }
            | Self::UnsupportedCommand { command, .. }
            | Self::CommandFailed { command, .. } => vec![("command", command.as_str().into())],
            Self::InvalidUrl { url, .. } | Self::Transfer { url, .. } => {
                vec![("url", url.as_str().into())]
            }
            Self::TooLarge { size, limit } => {
                vec![("size", (*size).into()), ("limit", (*limit).into())]
            }
            Self::Io { path, .. } => vec![("path", path.display().to_string().into())],
        };
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    /// Technical details for logs and the debug view.
    pub fn detail(&self) -> Option<String> {
        match self {
            Self::InvalidAddress { reason, .. } => Some(reason.clone()),
            Self::TooLarge { .. } => None,
            Self::Unreachable { detail, .. }
            | Self::Timeout { detail, .. }
            | Self::UnsupportedCommand { detail, .. } => Some(detail.clone()),
            Self::Http { body, .. } => Some(body.clone()),
            Self::MalformedResponse {
                detail, snippet, ..
            } => Some(format!("{detail}\nResponse: {snippet}")),
            Self::CommandFailed { response, .. } => Some(response.clone()),
            Self::InvalidUrl { url, reason } => Some(format!("{reason}: {url}")),
            Self::Transfer { url, detail } => Some(format!("{url}: {detail}")),
            Self::Io { source, .. } => Some(source.to_string()),
        }
    }
}
