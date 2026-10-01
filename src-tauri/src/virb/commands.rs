//! VIRB HTTP API commands.

use serde_json::{json, Value};

use crate::camera::WifiSecurity;

/// Commands V360Lab sends to the camera.
///
/// `docs/virb-http-api.md` lists the commands not used yet.
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
    /// `files` are media URLs exactly as reported by `mediaList`.
    /// Firmware 4.20 only deletes with a `files` array: a single `file`
    /// string (or any other key) is acknowledged with `"result": 1` but
    /// ignored. Deleting a video also removes its `.GLV` and `.THM`, but
    /// not its FIT file.
    DeleteFile {
        files: Vec<String>,
    },
    /// Starts the live preview and returns its RTSP URL. Without
    /// `streamType: "rtp"` firmware 4.20 answers `"result": 0`.
    LivePreview,
    /// Wi-Fi management. All operations share the `networks` command and
    /// are selected by `subCommand` (see [`NetworkCommand`]).
    Networks(NetworkCommand),
    /// Commands the firmware supports: `{"commandList": [{"command": …}]}`.
    CommandList,
    /// Makes the camera signal its position (sound and lights) until `found`.
    Locate,
    Found,
    /// Asks for a keyframe on the live preview stream, so that the image
    /// recovers at once after a lost packet.
    EnableIdr,
    /// Paired sensors: `{"sensors": [{"name", "found", …}]}`.
    Sensors,
    Standby,
    /// Media folders: `{"mediaDirs": [...]}`.
    MediaDirList,
    /// Garmin's app sends `favorite` as the string `"true"` / `"false"`.
    SetFavorite {
        file: String,
        favorite: bool,
    },
}

/// `networks` sub-commands, as sent by Garmin's VIRB app (recovered from
/// its native library; see `docs/virb-http-api.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkCommand {
    /// Name of the network the camera creates itself.
    GetApSsid,
    /// Networks saved on the camera.
    GetConfiguredNetworks,
    /// Networks the camera can see.
    GetScannedNetworks,
    /// Saves a network the camera joins as a client (`type: "station"`).
    Configure {
        ssid: String,
        security: WifiSecurity,
        password: String,
    },
    /// Makes the camera leave its own network and join a saved one.
    Connect { ssid: String },
    /// Removes a saved network.
    Remove { ssid: String },
}

impl NetworkCommand {
    pub fn sub_command(&self) -> &'static str {
        match self {
            Self::GetApSsid => "getApSSID",
            Self::GetConfiguredNetworks => "getConfiguredNetworks",
            Self::GetScannedNetworks => "getScannedNetworks",
            Self::Configure { .. } => "configureNetwork",
            Self::Connect { .. } => "connectNetwork",
            Self::Remove { .. } => "removeNetwork",
        }
    }

    fn args(&self) -> Option<Value> {
        match self {
            Self::Configure {
                ssid,
                security,
                password,
            } => Some(json!({
                "type": "station",
                "securityType": security.as_str(),
                "ssid": ssid,
                "password": password,
            })),
            Self::Connect { ssid } | Self::Remove { ssid } => Some(json!({ "ssid": ssid })),
            _ => None,
        }
    }
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
            Self::LivePreview => "livePreview",
            Self::Networks(_) => "networks",
            Self::CommandList => "commandList",
            Self::Locate => "locate",
            Self::Found => "found",
            Self::EnableIdr => "enableIDR",
            Self::Sensors => "sensors",
            Self::Standby => "standby",
            Self::MediaDirList => "mediaDirList",
            Self::SetFavorite { .. } => "setFavorite",
        }
    }

    /// Name used in logs and error messages: the sub-command for `networks`.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Networks(network) => network.sub_command(),
            _ => self.name(),
        }
    }

    /// Secret carried by the payload, to be kept out of logs.
    pub fn secret(&self) -> Option<&str> {
        match self {
            Self::Networks(NetworkCommand::Configure { password, .. }) if !password.is_empty() => {
                Some(password)
            }
            _ => None,
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
            Self::DeleteFile { files } => json!({ "command": self.name(), "files": files }),
            Self::LivePreview => json!({ "command": self.name(), "streamType": "rtp" }),
            Self::SetFavorite { file, favorite } => json!({
                "command": self.name(),
                "file": file,
                "favorite": if *favorite { "true" } else { "false" },
            }),
            Self::Networks(network) => {
                let mut payload =
                    json!({ "command": self.name(), "subCommand": network.sub_command() });
                if let Some(args) = network.args() {
                    payload["args"] = args;
                }
                payload
            }
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
            files: vec!["http://192.168.0.1:80/DCIM/100_VIRB/V0010001.MP4".into()],
        };
        assert_eq!(
            delete.payload()["files"][0],
            "http://192.168.0.1:80/DCIM/100_VIRB/V0010001.MP4"
        );
    }

    #[test]
    fn network_payloads_match_the_official_app() {
        assert_eq!(
            VirbCommand::Networks(NetworkCommand::GetScannedNetworks).payload(),
            json!({ "command": "networks", "subCommand": "getScannedNetworks" })
        );
        let configure = VirbCommand::Networks(NetworkCommand::Configure {
            ssid: "Home".into(),
            security: WifiSecurity::Wpa2,
            password: "secret123".into(),
        });
        assert_eq!(
            configure.payload(),
            json!({
                "command": "networks",
                "subCommand": "configureNetwork",
                "args": { "type": "station", "securityType": "WPA2", "ssid": "Home", "password": "secret123" }
            })
        );
        assert_eq!(configure.label(), "configureNetwork");
        assert_eq!(configure.secret(), Some("secret123"));
        assert_eq!(
            VirbCommand::Networks(NetworkCommand::Connect {
                ssid: "Home".into()
            })
            .payload(),
            json!({ "command": "networks", "subCommand": "connectNetwork", "args": { "ssid": "Home" } })
        );
    }
}
