//! Typed errors for camera communication.

use std::path::PathBuf;

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

    /// Technical details for logs and the debug view.
    pub fn detail(&self) -> Option<String> {
        match self {
            Self::InvalidAddress { .. } | Self::TooLarge { .. } => None,
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
