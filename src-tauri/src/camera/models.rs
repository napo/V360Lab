//! Camera-agnostic domain models shared with the frontend.
//!
//! Every model keeps the original JSON object in `raw` so that fields we do
//! not understand yet (e.g. from newer firmware) remain inspectable in the UI
//! and are preserved in `metadata.json` next to downloaded media.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CameraKind {
    // kebab-case alone would produce "garmin-virb360".
    #[serde(rename = "garmin-virb-360")]
    GarminVirb360,
    Mock,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub device_id: Option<String>,
    pub part_number: Option<String>,
    pub device_type: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordingState {
    Idle,
    Recording,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraStatus {
    pub recording_state: RecordingState,
    /// Camera mode as reported (e.g. `video`, `photo`).
    pub mode: Option<String>,
    /// Battery level in percent.
    pub battery_level: Option<f64>,
    pub battery_charging_state: Option<String>,
    pub storage_total_bytes: Option<u64>,
    pub storage_available_bytes: Option<u64>,
    /// Elapsed time of the current recording, in seconds.
    pub recording_time_secs: Option<f64>,
    pub recording_time_remaining_secs: Option<f64>,
    pub gps_latitude: Option<f64>,
    pub gps_longitude: Option<f64>,
    /// A Bluetooth headset or microphone is connected to the camera.
    pub bluetooth_headset: Option<bool>,
    /// A Bluetooth sensor (heart rate, …) is connected.
    pub bluetooth_sensor: Option<bool>,
    /// An ANT+ sensor is connected.
    pub ant_sensor: Option<bool>,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraFeature {
    /// Stable identifier reported by the camera (e.g. `videoMode`).
    pub key: String,
    pub label: Option<String>,
    pub value: Option<Value>,
    pub options: Vec<Value>,
    pub option_summaries: Vec<String>,
    pub enabled: Option<bool>,
    pub feature_type: Option<Value>,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureList {
    pub features: Vec<CameraFeature>,
    /// Full camera response, for the raw JSON debug view.
    pub raw: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Video,
    Photo,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub id: String,
    pub name: String,
    pub media_type: MediaType,
    /// Media type string exactly as reported by the camera.
    pub media_type_raw: Option<String>,
    /// Capture time as a Unix timestamp (seconds).
    pub timestamp: Option<i64>,
    /// Capture time as RFC 3339 (UTC).
    pub date_time: Option<String>,
    pub duration_secs: Option<f64>,
    pub file_size_bytes: Option<u64>,
    pub lens_mode: Option<String>,
    pub url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub low_res_url: Option<String>,
    pub fit_url: Option<String>,
    pub has_fit: bool,
    /// Marked as favourite on the camera (`fav`).
    pub favorite: Option<bool>,
    pub raw: Value,
}

/// A sensor paired with the camera (heart rate, cadence, …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorInfo {
    pub name: String,
    /// How it is connected, e.g. `ANT` or `LOCAL`.
    pub sensor_type: Option<String>,
    /// Currently found (connected) by the camera.
    pub found: Option<bool>,
    pub raw: Value,
}

/// Security of a Wi-Fi network, as the VIRB names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WifiSecurity {
    #[serde(rename = "WPA2")]
    Wpa2,
    #[serde(rename = "WPA")]
    Wpa,
    #[serde(rename = "WEP")]
    Wep,
    Open,
}

impl WifiSecurity {
    /// Value sent to the camera.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wpa2 => "WPA2",
            Self::Wpa => "WPA",
            Self::Wep => "WEP",
            Self::Open => "Open",
        }
    }

    /// Case-insensitive match of a value reported by the camera.
    pub fn parse(value: &str) -> Option<Self> {
        [Self::Wpa2, Self::Wpa, Self::Wep, Self::Open]
            .into_iter()
            .find(|s| s.as_str().eq_ignore_ascii_case(value.trim()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiNetwork {
    pub ssid: String,
    /// `None` when the camera reports an unknown security value
    /// (kept in `security_raw`).
    pub security: Option<WifiSecurity>,
    pub security_raw: Option<String>,
}

/// Wi-Fi networks as seen by the camera.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiNetworks {
    /// Name of the network the camera creates itself.
    pub access_point_ssid: Option<String>,
    /// Networks saved on the camera, which it joins automatically.
    pub configured: Vec<WifiNetwork>,
    /// Networks the camera can see now.
    pub scanned: Vec<WifiNetwork>,
}

/// Result of a control command (start/stop recording, snap picture).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandAck {
    pub command: String,
    pub raw: Value,
}

/// Bytes `start..=end` of a resource; `end: None` means "to the end".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end: Option<u64>,
}

/// Part of a resource, for media players that seek with Range requests.
#[derive(Debug, Clone)]
pub struct RangedResource {
    pub bytes: Vec<u8>,
    /// Offset of `bytes` in the whole resource.
    pub start: u64,
    /// Size of the whole resource, when known.
    pub total: Option<u64>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FetchedResource {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_kind_names_match_the_frontend() {
        assert_eq!(
            serde_json::to_value(CameraKind::GarminVirb360).unwrap(),
            "garmin-virb-360"
        );
        assert_eq!(serde_json::to_value(CameraKind::Mock).unwrap(), "mock");
    }
}
