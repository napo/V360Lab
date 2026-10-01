//! GPS track export: GPX 1.1 (GPS tools, Strava, OpenStreetMap editors)
//! and GeoJSON (QGIS, web maps, scripts).

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};

use super::summary::VideoTelemetry;
use super::TelemetrySample;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackFormat {
    Gpx,
    Geojson,
}

impl TrackFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Gpx => "gpx",
            Self::Geojson => "geojson",
        }
    }
}

fn iso_time(unix_ms: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(unix_ms)
        .map(|t| t.to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn positioned(samples: &[TelemetrySample]) -> impl Iterator<Item = (&TelemetrySample, f64, f64)> {
    samples
        .iter()
        .filter_map(|s| Some((s, s.latitude?, s.longitude?)))
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// GPX 1.1 track; `name` is usually the video file name.
pub fn to_gpx(telemetry: &VideoTelemetry, name: &str) -> String {
    let name = xml_escape(name);
    let mut gpx = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <gpx version=\"1.1\" creator=\"V360Lab\" xmlns=\"http://www.topografix.com/GPX/1/1\">\n",
    );
    gpx.push_str(&format!("  <trk>\n    <name>{name}</name>\n    <trkseg>\n"));
    for (sample, lat, lon) in positioned(&telemetry.samples) {
        gpx.push_str(&format!("      <trkpt lat=\"{lat:.7}\" lon=\"{lon:.7}\">"));
        if let Some(ele) = sample.altitude_m {
            gpx.push_str(&format!("<ele>{ele:.1}</ele>"));
        }
        if let Some(time) = iso_time(sample.timestamp_ms) {
            gpx.push_str(&format!("<time>{time}</time>"));
        }
        gpx.push_str("</trkpt>\n");
    }
    gpx.push_str("    </trkseg>\n  </trk>\n</gpx>\n");
    gpx
}

/// GeoJSON FeatureCollection with one LineString. Per-point values are in
/// arrays parallel to the coordinates: `times` (UTC), `videoSeconds`
/// (position in the video), `speedsMps`.
pub fn to_geojson(telemetry: &VideoTelemetry, name: &str) -> Value {
    let points: Vec<_> = positioned(&telemetry.samples).collect();
    let coordinates: Vec<Value> = points
        .iter()
        .map(|(s, lat, lon)| match s.altitude_m {
            Some(ele) => json!([lon, lat, ele]),
            None => json!([lon, lat]),
        })
        .collect();
    let times: Vec<Value> = points
        .iter()
        .map(|(s, ..)| json!(iso_time(s.timestamp_ms)))
        .collect();
    let video_seconds: Vec<Value> = points
        .iter()
        .map(|(s, ..)| json!((s.timestamp_ms - telemetry.video_start_ms) as f64 / 1000.0))
        .collect();
    let speeds: Vec<Value> = points.iter().map(|(s, ..)| json!(s.speed_mps)).collect();
    let summary = &telemetry.summary;
    json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "geometry": { "type": "LineString", "coordinates": coordinates },
            "properties": {
                "name": name,
                "videoStartTime": iso_time(telemetry.video_start_ms),
                "videoStartFromCameraEvent": telemetry.start_from_camera_event,
                "distanceM": summary.distance_m,
                "durationSecs": summary.duration_secs,
                "maxSpeedMps": summary.max_speed_mps,
                "elevationGainM": summary.elevation_gain_m,
                "times": times,
                "videoSeconds": video_seconds,
                "speedsMps": speeds,
                "creator": "V360Lab",
            }
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::summary::summarize;

    fn telemetry() -> VideoTelemetry {
        let samples = vec![
            TelemetrySample {
                timestamp_ms: 1_613_758_902_000,
                latitude: Some(46.07),
                longitude: Some(11.12),
                altitude_m: Some(194.0),
                speed_mps: Some(4.0),
                heading_deg: None,
                heart_rate: None,
            },
            TelemetrySample {
                timestamp_ms: 1_613_758_903_000,
                latitude: None,
                longitude: None,
                altitude_m: Some(195.0),
                speed_mps: None,
                heading_deg: None,
                heart_rate: None,
            },
            TelemetrySample {
                timestamp_ms: 1_613_758_904_500,
                latitude: Some(46.071),
                longitude: Some(11.121),
                altitude_m: None,
                speed_mps: Some(5.0),
                heading_deg: None,
                heart_rate: None,
            },
        ];
        VideoTelemetry {
            video_start_ms: 1_613_758_901_000,
            start_from_camera_event: true,
            summary: summarize(&samples),
            samples,
            camera_events: Vec::new(),
        }
    }

    #[test]
    fn writes_gpx_track_points() {
        let gpx = to_gpx(&telemetry(), "V0010001 <test>.MP4");
        assert!(gpx.contains("<name>V0010001 &lt;test&gt;.MP4</name>"));
        assert_eq!(gpx.matches("<trkpt ").count(), 2);
        assert!(gpx.contains(
            "<trkpt lat=\"46.0700000\" lon=\"11.1200000\"><ele>194.0</ele><time>2021-02-19T18:21:42.000Z</time></trkpt>"
        ));
        assert!(gpx.contains("<trkpt lat=\"46.0710000\" lon=\"11.1210000\"><time>2021-02-19T18:21:44.500Z</time></trkpt>"));
    }

    #[test]
    fn writes_geojson_with_video_times() {
        let geojson = to_geojson(&telemetry(), "V0010001.MP4");
        let feature = &geojson["features"][0];
        assert_eq!(
            feature["geometry"]["coordinates"][0],
            json!([11.12, 46.07, 194.0])
        );
        assert_eq!(
            feature["geometry"]["coordinates"][1],
            json!([11.121, 46.071])
        );
        assert_eq!(feature["properties"]["videoSeconds"], json!([1.0, 3.5]));
        assert_eq!(
            feature["properties"]["times"][1],
            "2021-02-19T18:21:44.500Z"
        );
        assert_eq!(feature["properties"]["videoStartFromCameraEvent"], true);
    }
}
