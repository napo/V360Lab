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
    pub raw: Value,
}

/// Result of a control command (start/stop recording, snap picture).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandAck {
    pub command: String,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct FetchedResource {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}
