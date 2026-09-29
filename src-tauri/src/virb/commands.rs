//! VIRB HTTP API commands.

use serde_json::{json, Value};

/// Commands V360Lab sends to the camera.
///
/// Other VIRB commands (e.g. `livePreview`, `locate`) are not used yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirbCommand {
    DeviceInfo,
    Status,
    Features,
    StartRecording,
    StopRecording,
    SnapPicture,
    /// Ends a still capture sequence (photo time-lapse, burst).
    StopStillRecording,
    MediaList,
    /// Firmware 4.20 answers with the complete, updated feature list.
    UpdateFeature {
        feature: String,
        value: String,
    },
    /// `file` is the media URL exactly as reported by `mediaList`.
    /// Firmware 4.20 answers `"result": 1` even for files that do not exist.
    DeleteFile {
        file: String,
    },
}

impl VirbCommand {
    /// Command name as expected by the camera.
    pub fn name(&self) -> &'static str {
        match self {
            Self::DeviceInfo => "deviceInfo",
            Self::Status => "status",
            Self::Features => "features",
            Self::StartRecording => "startRecording",
            Self::StopRecording => "stopRecording",
            Self::SnapPicture => "snapPicture",
            Self::StopStillRecording => "stopStillRecording",
            Self::MediaList => "mediaList",
            Self::UpdateFeature { .. } => "updateFeature",
            Self::DeleteFile { .. } => "deleteFile",
        }
    }

    /// JSON body sent to `POST /virb`.
    pub fn payload(&self) -> Value {
        match self {
            Self::UpdateFeature { feature, value } => json!({
                "command": self.name(),
                "feature": feature,
                "value": value,
            }),
            Self::DeleteFile { file } => json!({ "command": self.name(), "file": file }),
            _ => json!({ "command": self.name() }),
        }
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

    #[test]
    fn payloads_with_arguments() {
        let update = VirbCommand::UpdateFeature {
            feature: "shootingMode".into(),
            value: "photoShootingMode".into(),
        };
        assert_eq!(
            update.payload(),
            json!({ "command": "updateFeature", "feature": "shootingMode", "value": "photoShootingMode" })
        );
        let delete = VirbCommand::DeleteFile {
            file: "http://192.168.0.1:80/DCIM/100_VIRB/V0010001.MP4".into(),
        };
        assert_eq!(
            delete.payload()["file"],
            "http://192.168.0.1:80/DCIM/100_VIRB/V0010001.MP4"
        );
    }
}
