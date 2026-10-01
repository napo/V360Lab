//! Telemetry support (FIT files recorded alongside VIRB media).
//!
//! V360Lab detects FIT files associated with media (`fitURL` in the media
//! list), downloads them next to the media, checks their header
//! ([`fit::read_header`]) and decodes the GPS track and camera events
//! ([`decode::decode`]) to show them in sync with the video ([`summary`]).
//!
//! The pipeline (roadmap phase 2) is:
//!
//! ```text
//! FIT file -> TelemetryTrack (time, GNSS, altitude, speed, IMU, camera events)
//!          -> alignment with the video timeline
//!          -> GPS track / map view / GeoJSON or GeoParquet export
//! ```
//!
//! Keep this module independent from the camera and UI layers so it can be
//! reused by batch tools later.

pub mod decode;
pub mod export;
pub mod fit;
pub mod summary;

use std::path::Path;

use serde::Serialize;

/// One decoded telemetry sample.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySample {
    /// UTC time in milliseconds since the Unix epoch.
    pub timestamp_ms: i64,
    /// Degrees (WGS84).
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude_m: Option<f64>,
    pub speed_mps: Option<f64>,
    /// Direction of travel, degrees clockwise from north.
    pub heading_deg: Option<f64>,
    pub heart_rate: Option<u16>,
}

/// A VIRB camera event (`camera_event` message): video start, split, end…
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraEvent {
    /// UTC time in milliseconds since the Unix epoch.
    pub timestamp_ms: i64,
    /// FIT `camera_event_type`: 0 video start, 1 split, 2 end, 4 photo, …
    pub event_type: i64,
    pub file_uuid: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryTrack {
    /// Sorted by time.
    pub samples: Vec<TelemetrySample>,
    pub camera_events: Vec<CameraEvent>,
}

/// Interface for a future FIT (or other telemetry format) decoder.
pub trait TelemetryParser {
    fn parse(&self, path: &Path) -> Result<TelemetryTrack, fit::FitError>;
}
