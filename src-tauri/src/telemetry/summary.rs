//! Telemetry of one video: the part of the FIT track recorded while the
//! video was filmed, aligned with its timeline, plus summary figures.

use serde::Serialize;

use super::{CameraEvent, TelemetrySample, TelemetryTrack};

/// `camera_event_type` of a video start.
const VIDEO_START: i64 = 0;
/// A video start event this close to the media date is the same video.
const START_MATCH_MS: i64 = 10_000;
/// Points sent to the UI; longer tracks are thinned out evenly.
pub const MAX_POINTS: usize = 5_000;
/// GPS noise below this is not counted as climbing.
const ELEVATION_NOISE_M: f64 = 2.0;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackSummary {
    pub duration_secs: f64,
    pub distance_m: f64,
    pub max_speed_mps: Option<f64>,
    pub avg_speed_mps: Option<f64>,
    pub min_altitude_m: Option<f64>,
    pub max_altitude_m: Option<f64>,
    pub elevation_gain_m: f64,
    pub sample_count: usize,
    pub has_position: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoTelemetry {
    /// UTC milliseconds of the video's first frame: sample times minus this
    /// are positions in the video.
    pub video_start_ms: i64,
    /// True when the start comes from a camera event in the FIT file rather
    /// than from the (second-precise) media date.
    pub start_from_camera_event: bool,
    pub samples: Vec<TelemetrySample>,
    pub summary: TrackSummary,
    pub camera_events: Vec<CameraEvent>,
}

/// Great-circle distance in metres.
pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_008.8;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = p2 - p1;
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * a.sqrt().asin()
}

pub fn summarize(samples: &[TelemetrySample]) -> TrackSummary {
    let mut summary = TrackSummary {
        sample_count: samples.len(),
        ..TrackSummary::default()
    };
    if let (Some(first), Some(last)) = (samples.first(), samples.last()) {
        summary.duration_secs = (last.timestamp_ms - first.timestamp_ms) as f64 / 1000.0;
    }
    let mut previous: Option<(f64, f64)> = None;
    let mut climb_base: Option<f64> = None;
    for sample in samples {
        if let (Some(lat), Some(lon)) = (sample.latitude, sample.longitude) {
            summary.has_position = true;
            if let Some((plat, plon)) = previous {
                summary.distance_m += haversine_m(plat, plon, lat, lon);
            }
            previous = Some((lat, lon));
        }
        if let Some(speed) = sample.speed_mps {
            summary.max_speed_mps =
                Some(summary.max_speed_mps.map_or(speed, |m: f64| m.max(speed)));
        }
        if let Some(alt) = sample.altitude_m {
            summary.min_altitude_m = Some(summary.min_altitude_m.map_or(alt, |m: f64| m.min(alt)));
            summary.max_altitude_m = Some(summary.max_altitude_m.map_or(alt, |m: f64| m.max(alt)));
            // Count climbs only once they exceed the noise threshold.
            match climb_base {
                None => climb_base = Some(alt),
                Some(base) if alt - base >= ELEVATION_NOISE_M => {
                    summary.elevation_gain_m += alt - base;
                    climb_base = Some(alt);
                }
                Some(base) if alt < base => climb_base = Some(alt),
                _ => {}
            }
        }
    }
    if summary.duration_secs > 0.0 && summary.has_position {
        summary.avg_speed_mps = Some(summary.distance_m / summary.duration_secs);
    }
    summary
}

/// Keeps at most `max` samples, evenly spread, always keeping the last.
fn thin(samples: Vec<TelemetrySample>, max: usize) -> Vec<TelemetrySample> {
    if samples.len() <= max || max < 2 {
        return samples;
    }
    let step = (samples.len() - 1) as f64 / (max - 1) as f64;
    (0..max)
        .map(|i| samples[((i as f64 * step).round() as usize).min(samples.len() - 1)].clone())
        .collect()
}

/// The telemetry of a video recorded at `media_timestamp_s` (Unix seconds,
/// the media list's `date`) lasting `duration_secs`.
pub fn for_video(
    track: TelemetryTrack,
    media_timestamp_s: Option<i64>,
    duration_secs: Option<f64>,
    max_points: usize,
) -> VideoTelemetry {
    let media_ms = media_timestamp_s.map(|t| t * 1000);
    let event_start = media_ms.and_then(|media| {
        track
            .camera_events
            .iter()
            .filter(|e| e.event_type == VIDEO_START)
            .map(|e| e.timestamp_ms)
            .filter(|t| (t - media).abs() <= START_MATCH_MS)
            .min_by_key(|t| (t - media).abs())
    });
    let video_start_ms = event_start
        .or(media_ms)
        .or_else(|| track.samples.first().map(|s| s.timestamp_ms))
        .unwrap_or(0);

    // The FIT file can span several videos: keep this one's part, unless
    // nothing falls in it (e.g. a wrong clock), then show everything.
    let window: Vec<TelemetrySample> = match duration_secs {
        Some(duration) if duration > 0.0 => {
            let end = video_start_ms + (duration * 1000.0).ceil() as i64;
            track
                .samples
                .iter()
                .filter(|s| s.timestamp_ms >= video_start_ms && s.timestamp_ms <= end)
                .cloned()
                .collect()
        }
        _ => Vec::new(),
    };
    let samples = if window.is_empty() {
        track.samples
    } else {
        window
    };
    VideoTelemetry {
        video_start_ms,
        start_from_camera_event: event_start.is_some(),
        summary: summarize(&samples),
        samples: thin(samples, max_points),
        camera_events: track.camera_events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(t_ms: i64, lat: f64, lon: f64, alt: f64, speed: f64) -> TelemetrySample {
        TelemetrySample {
            timestamp_ms: t_ms,
            latitude: Some(lat),
            longitude: Some(lon),
            altitude_m: Some(alt),
            speed_mps: Some(speed),
            heading_deg: None,
            heart_rate: None,
        }
    }

    #[test]
    fn measures_distance_and_climb() {
        // 0.001° of latitude is about 111 m.
        let samples = [
            sample(0, 46.0, 11.0, 200.0, 1.0),
            sample(10_000, 46.001, 11.0, 201.0, 3.0),
            sample(20_000, 46.002, 11.0, 210.0, 2.0),
        ];
        let summary = summarize(&samples);
        assert!(
            (summary.distance_m - 222.4).abs() < 1.0,
            "{}",
            summary.distance_m
        );
        assert_eq!(summary.duration_secs, 20.0);
        assert_eq!(summary.max_speed_mps, Some(3.0));
        assert_eq!(summary.elevation_gain_m, 10.0);
        assert_eq!(summary.min_altitude_m, Some(200.0));
    }

    #[test]
    fn aligns_with_the_video_start_event() {
        let track = TelemetryTrack {
            samples: (0..100)
                .map(|i| sample(1_000_000 + i * 1000, 46.0, 11.0, 200.0, 1.0))
                .collect(),
            camera_events: vec![CameraEvent {
                timestamp_ms: 1_010_400,
                event_type: VIDEO_START,
                file_uuid: None,
            }],
        };
        let video = for_video(track, Some(1_010), Some(20.0), MAX_POINTS);
        assert_eq!(video.video_start_ms, 1_010_400);
        assert!(video.start_from_camera_event);
        assert_eq!(video.samples.first().unwrap().timestamp_ms, 1_011_000);
        assert_eq!(video.samples.last().unwrap().timestamp_ms, 1_030_000);
    }

    #[test]
    fn falls_back_to_the_whole_track_and_thins_it() {
        let track = TelemetryTrack {
            samples: (0..1000)
                .map(|i| sample(i * 100, 46.0, 11.0, 200.0, 1.0))
                .collect(),
            camera_events: Vec::new(),
        };
        let video = for_video(track, Some(999_999), Some(10.0), 100);
        assert!(!video.start_from_camera_event);
        assert_eq!(video.samples.len(), 100);
        assert_eq!(video.samples.last().unwrap().timestamp_ms, 99_900);
        assert_eq!(video.summary.sample_count, 1000);
    }
}
