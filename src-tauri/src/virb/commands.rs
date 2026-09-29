//! VIRB HTTP API commands.

use serde_json::{json, Value};

/// Commands V360Lab currently sends to the camera.
///
/// The VIRB API has more commands (e.g. `updateFeature`, `deleteFile`,
/// `livePreview`); they are intentionally not exposed yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirbCommand {
    DeviceInfo,
    Status,
    Features,
    StartRecording,
    StopRecording,
    SnapPicture,
    MediaList,
}

impl VirbCommand {
    /// Command name as expected by the camera.
    pub fn name(self) -> &'static str {
        match self {
            Self::DeviceInfo => "deviceInfo",
            Self::Status => "status",
            Self::Features => "features",
            Self::StartRecording => "startRecording",
            Self::StopRecording => "stopRecording",
            Self::SnapPicture => "snapPicture",
            Self::MediaList => "mediaList",
        }
    }

    /// JSON body sent to `POST /virb`.
    pub fn payload(self) -> Value {
        json!({ "command": self.name() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_contains_command_name() {
        assert_eq!(
            VirbCommand::MediaList.payload(),
            json!({ "command": "mediaList" })
        );
        assert_eq!(VirbCommand::SnapPicture.name(), "snapPicture");
    }
}
