//! Telemetry support (FIT files recorded alongside VIRB media).
//!
//! # Current state (MVP)
//!
//! V360Lab only *detects* FIT files associated with media (`fitURL` in the
//! media list), *downloads* them next to the media, and sanity-checks the FIT
//! file header ([`fit::read_header`]). Records are not decoded yet.
//!
//! # TODO(telemetry): FIT decoding
//!
//! A decoder can be plugged in behind [`TelemetryParser`]. Candidates:
//!
//! - the official Garmin FIT SDK (<https://developer.garmin.com/fit/>),
//!   through its C implementation or a generated binding; check the SDK
//!   license terms before bundling it;
//! - a native Rust parser such as the `fitparser` crate.
//!
//! The planned pipeline (roadmap phase 2) is:
//!
//! ```text
//! FIT file -> TelemetryTrack (time, GNSS, altitude, speed, IMU, camera events)
//!          -> alignment with the video timeline
//!          -> GPS track / map view / GeoJSON or GeoParquet export
//! ```
//!
//! Keep this module independent from the camera and UI layers so it can be
//! reused by batch tools later.

pub mod fit;

use std::path::Path;

use serde::Serialize;

/// One decoded telemetry sample. Deliberately minimal until a decoder exists.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySample {
    /// UTC time in milliseconds since the Unix epoch.
    pub timestamp_ms: i64,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude_m: Option<f64>,
    pub speed_mps: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryTrack {
    pub samples: Vec<TelemetrySample>,
}

/// Interface for a future FIT (or other telemetry format) decoder.
pub trait TelemetryParser {
    fn parse(&self, path: &Path) -> Result<TelemetryTrack, fit::FitError>;
}
