//! Tolerant parsing of VIRB responses into camera-agnostic models.
//!
//! The field names below follow Garmin's VIRB network API as far as it is
//! known. Firmware versions differ, so every field is optional, numbers may
//! arrive as strings (and vice versa), and aliases are accepted where
//! variants have been seen or are plausible. The original JSON is always kept
//! in the model's `raw` field.

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::camera::{
    CameraError, CameraFeature, CameraStatus, CommandAck, DeviceInfo, FeatureList, MediaItem,
    MediaType, RecordingState,
};

use super::errors::snippet;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VirbDeviceInfo {
    #[serde(default, deserialize_with = "flex::opt_string")]
    model: Option<String>,
    #[serde(
        default,
        alias = "firmwareVersion",
        deserialize_with = "flex::opt_string"
    )]
    firmware: Option<String>,
    #[serde(
        default,
        alias = "deviceID",
        alias = "unitId",
        alias = "serialNumber",
        deserialize_with = "flex::opt_string"
    )]
    device_id: Option<String>,
    #[serde(default, deserialize_with = "flex::opt_string")]
    part_number: Option<String>,
    #[serde(default, rename = "type", deserialize_with = "flex::opt_string")]
    device_type: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VirbStatus {
    #[serde(default, deserialize_with = "flex::opt_string")]
    state: Option<String>,
    #[serde(default, alias = "isRecording", deserialize_with = "flex::opt_bool")]
    recording: Option<bool>,
    #[serde(default, deserialize_with = "flex::opt_string")]
    mode: Option<String>,
    #[serde(default, alias = "battery", deserialize_with = "flex::opt_f64")]
    battery_level: Option<f64>,
    #[serde(default, deserialize_with = "flex::opt_string")]
    battery_charging_state: Option<String>,
    #[serde(default, deserialize_with = "flex::opt_u64")]
    total_space: Option<u64>,
    #[serde(default, alias = "freeSpace", deserialize_with = "flex::opt_u64")]
    available_space: Option<u64>,
    #[serde(default, deserialize_with = "flex::opt_f64")]
    recording_time: Option<f64>,
    #[serde(default, deserialize_with = "flex::opt_f64")]
    recording_time_remaining: Option<f64>,
    #[serde(default, deserialize_with = "flex::opt_f64")]
    gps_latitude: Option<f64>,
    #[serde(default, deserialize_with = "flex::opt_f64")]
    gps_longitude: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VirbFeature {
    #[serde(
        default,
        alias = "name",
        alias = "key",
        deserialize_with = "flex::opt_string"
    )]
    feature: Option<String>,
    #[serde(default, alias = "label", deserialize_with = "flex::opt_string")]
    description: Option<String>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    options: Option<Value>,
    #[serde(default, alias = "optionsSummary", alias = "optionSummaries")]
    option_summary: Option<Value>,
    #[serde(default, deserialize_with = "flex::opt_bool")]
    enabled: Option<bool>,
    #[serde(default, rename = "type")]
    feature_type: Option<Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VirbMediaItem {
    #[serde(
        default,
        alias = "fileName",
        alias = "filename",
        deserialize_with = "flex::opt_string"
    )]
    name: Option<String>,
    #[serde(default, rename = "type", deserialize_with = "flex::opt_string")]
    media_type: Option<String>,
    #[serde(default, alias = "timestamp", alias = "dateTime")]
    date: Option<Value>,
    #[serde(default, deserialize_with = "flex::opt_f64")]
    duration: Option<f64>,
    #[serde(default, alias = "size", deserialize_with = "flex::opt_u64")]
    file_size: Option<u64>,
    #[serde(default, deserialize_with = "flex::opt_string")]
    lens_mode: Option<String>,
    #[serde(
        default,
        alias = "URL",
        alias = "path",
        deserialize_with = "flex::opt_string"
    )]
    url: Option<String>,
    #[serde(
        default,
        alias = "thumbURL",
        alias = "thumbnailUrl",
        alias = "thumbnailURL",
        deserialize_with = "flex::opt_string"
    )]
    thumb_url: Option<String>,
    #[serde(
        default,
        alias = "lowResVideoURL",
        alias = "lowResVideoUrl",
        alias = "lowResUrl",
        alias = "lowResURL",
        deserialize_with = "flex::opt_string"
    )]
    low_res_video_path: Option<String>,
    #[serde(
        default,
        rename = "fitURL",
        alias = "fitUrl",
        alias = "fitPath",
        deserialize_with = "flex::opt_string"
    )]
    fit_url: Option<String>,
}

fn malformed(command: &str, detail: impl Into<String>, response: &Value) -> CameraError {
    CameraError::MalformedResponse {
        command: command.to_string(),
        detail: detail.into(),
        snippet: snippet(&response.to_string(), 512),
    }
}

/// Parses `deviceInfo`. Known shape: `{"deviceInfo":[{...}],"result":1}`;
/// a single object or top-level fields are accepted as well.
pub fn parse_device_info(response: &Value) -> Result<DeviceInfo, CameraError> {
    const CMD: &str = "deviceInfo";
    let entry = match response.get("deviceInfo") {
        Some(Value::Array(items)) => items
            .first()
            .cloned()
            .ok_or_else(|| malformed(CMD, "empty \"deviceInfo\" array", response))?,
        Some(object @ Value::Object(_)) => object.clone(),
        Some(_) => return Err(malformed(CMD, "unexpected \"deviceInfo\" type", response)),
        None => response.clone(),
    };
    if !entry.is_object() {
        return Err(malformed(CMD, "device entry is not an object", response));
    }
    let parsed: VirbDeviceInfo = serde_json::from_value(entry.clone())
        .map_err(|e| malformed(CMD, e.to_string(), response))?;
    Ok(DeviceInfo {
        model: parsed.model,
        firmware: firmware_version(&entry["firmware"]).or(parsed.firmware),
        device_id: parsed.device_id,
        part_number: parsed.part_number,
        device_type: parsed.device_type,
        raw: entry,
    })
}

/// Garmin reports firmware as an integer scaled by 100 (`420` = 4.20).
fn firmware_version(value: &Value) -> Option<String> {
    let scaled = value.as_u64().filter(|n| *n >= 100)?;
    Some(format!("{}.{:02}", scaled / 100, scaled % 100))
}

/// Parses `status`. Fields are expected at the top level of the response.
/// Storage values are reported in KiB (a 128 GB card reports ~125 000 000)
/// and converted to bytes here.
pub fn parse_status(response: &Value) -> Result<CameraStatus, CameraError> {
    const CMD: &str = "status";
    let entry = match response.get("status") {
        Some(object @ Value::Object(_)) => object,
        _ => response,
    };
    let parsed: VirbStatus = serde_json::from_value(entry.clone())
        .map_err(|e| malformed(CMD, e.to_string(), response))?;
    Ok(CameraStatus {
        recording_state: recording_state(parsed.state.as_deref(), parsed.recording),
        mode: parsed.mode,
        battery_level: parsed.battery_level,
        battery_charging_state: parsed.battery_charging_state,
        storage_total_bytes: parsed.total_space.map(kib_to_bytes),
        storage_available_bytes: parsed.available_space.map(kib_to_bytes),
        recording_time_secs: parsed.recording_time,
        recording_time_remaining_secs: parsed.recording_time_remaining,
        gps_latitude: parsed.gps_latitude,
        gps_longitude: parsed.gps_longitude,
        raw: entry.clone(),
    })
}

fn kib_to_bytes(kib: u64) -> u64 {
    kib.saturating_mul(1024)
}

fn recording_state(state: Option<&str>, recording: Option<bool>) -> RecordingState {
    if let Some(state) = state.map(str::to_ascii_lowercase) {
        if state.contains("record") {
            return RecordingState::Recording;
        }
        if ["idle", "ready", "standby", "stopped"].contains(&state.as_str()) {
            return RecordingState::Idle;
        }
    }
    match recording {
        Some(true) => RecordingState::Recording,
        Some(false) => RecordingState::Idle,
        None => RecordingState::Unknown,
    }
}

/// Parses `features`. Known shape: `{"features":[{"feature":..,"value":..,
/// "options":[..],"optionSummary":[..],"enabled":1,"type":1}],"result":1}`.
pub fn parse_features(response: &Value) -> Result<FeatureList, CameraError> {
    const CMD: &str = "features";
    let items = match response.get("features") {
        Some(Value::Array(items)) => items,
        Some(_) => return Err(malformed(CMD, "\"features\" is not an array", response)),
        None => return Err(malformed(CMD, "missing \"features\" array", response)),
    };
    let features = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            if !item.is_object() {
                log::warn!("Skipping non-object feature entry at index {index}");
                return None;
            }
            let parsed: VirbFeature = serde_json::from_value(item.clone()).unwrap_or_default();
            Some(CameraFeature {
                key: parsed.feature.unwrap_or_else(|| format!("feature-{index}")),
                label: parsed.description,
                value: parsed.value.filter(|v| !v.is_null()),
                options: value_list(parsed.options),
                option_summaries: value_list(parsed.option_summary)
                    .into_iter()
                    .map(|v| match v {
                        Value::String(s) => s,
                        other => other.to_string(),
                    })
                    .collect(),
                enabled: parsed.enabled,
                feature_type: parsed.feature_type.filter(|v| !v.is_null()),
                raw: item.clone(),
            })
        })
        .collect();
    Ok(FeatureList {
        features,
        raw: response.clone(),
    })
}

fn value_list(value: Option<Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items,
        Some(Value::Null) | None => Vec::new(),
        Some(single) => vec![single],
    }
}

/// Parses `mediaList`. Known shape: `{"media":[{...}],"result":1}`.
/// A response without a `media` key is treated as an empty list.
pub fn parse_media_list(response: &Value) -> Result<Vec<MediaItem>, CameraError> {
    const CMD: &str = "mediaList";
    let items = match response.get("media").or_else(|| response.get("mediaList")) {
        Some(Value::Array(items)) => items,
        Some(Value::Null) | None => return Ok(Vec::new()),
        Some(_) => return Err(malformed(CMD, "\"media\" is not an array", response)),
    };
    Ok(items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let parsed = parse_media_item(item, index);
            if parsed.is_none() {
                log::warn!("Skipping unparseable media entry at index {index}");
            }
            parsed
        })
        .collect())
}

/// Parses one media entry; returns `None` for entries that are not objects.
pub fn parse_media_item(item: &Value, index: usize) -> Option<MediaItem> {
    if !item.is_object() {
        return None;
    }
    // The lenient field deserializers cannot fail; only conflicting aliases
    // (e.g. both "date" and "timestamp") can. Keep going with `raw` intact.
    let parsed: VirbMediaItem = serde_json::from_value(item.clone()).unwrap_or_else(|e| {
        log::warn!("Media entry {index} has conflicting fields: {e}");
        VirbMediaItem::default()
    });
    let name = parsed
        .name
        .clone()
        .or_else(|| parsed.url.as_deref().and_then(last_path_segment))
        .unwrap_or_else(|| format!("media-{index}"));
    let id = parsed.url.clone().unwrap_or_else(|| name.clone());
    let timestamp = parsed.date.as_ref().and_then(parse_timestamp);
    let media_type = media_type(parsed.media_type.as_deref(), &name);

    Some(MediaItem {
        id,
        media_type,
        media_type_raw: parsed.media_type,
        timestamp,
        date_time: timestamp
            .and_then(|t| DateTime::<Utc>::from_timestamp(t, 0))
            .map(|d| d.to_rfc3339()),
        duration_secs: parsed.duration,
        file_size_bytes: parsed.file_size,
        lens_mode: parsed.lens_mode,
        url: parsed.url,
        thumbnail_url: parsed.thumb_url,
        low_res_url: parsed.low_res_video_path,
        has_fit: parsed.fit_url.is_some(),
        fit_url: parsed.fit_url,
        name,
        raw: item.clone(),
    })
}

fn media_type(reported: Option<&str>, name: &str) -> MediaType {
    if let Some(kind) = reported.map(str::to_ascii_lowercase) {
        match kind.as_str() {
            "video" | "movie" | "mp4" => return MediaType::Video,
            "photo" | "image" | "picture" | "still" | "jpg" | "jpeg" => return MediaType::Photo,
            _ => {}
        }
    }
    let extension = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase());
    match extension.as_deref() {
        Some("mp4" | "mov" | "glv") => MediaType::Video,
        Some("jpg" | "jpeg" | "png" | "dng") => MediaType::Photo,
        _ => MediaType::Other,
    }
}

/// Accepts Unix seconds, Unix milliseconds, numeric strings and ISO 8601.
fn parse_timestamp(value: &Value) -> Option<i64> {
    let from_number = |n: f64| {
        let secs = if n > 1e12 { n / 1000.0 } else { n };
        (secs > 0.0).then_some(secs as i64)
    };
    match value {
        Value::Number(n) => n.as_f64().and_then(from_number),
        Value::String(s) => {
            let s = s.trim();
            if let Ok(n) = s.parse::<f64>() {
                return from_number(n);
            }
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Some(dt.timestamp());
            }
            [
                "%Y-%m-%dT%H:%M:%S",
                "%Y-%m-%d %H:%M:%S",
                "%Y:%m:%d %H:%M:%S",
            ]
            .iter()
            .find_map(|fmt| NaiveDateTime::parse_from_str(s, fmt).ok())
            .map(|dt| dt.and_utc().timestamp())
        }
        _ => None,
    }
}

fn last_path_segment(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    path.rsplit('/')
        .find(|segment| !segment.is_empty())
        .map(str::to_string)
}

pub fn command_ack(command: &str, response: Value) -> CommandAck {
    CommandAck {
        command: command.to_string(),
        raw: response,
    }
}

/// Lenient deserializers: never fail on unexpected types, return `None`.
mod flex {
    use serde::{Deserialize, Deserializer};
    use serde_json::Value;

    pub fn opt_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::String(s) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        })
    }

    pub fn opt_f64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
        .filter(|n: &f64| n.is_finite()))
    }

    pub fn opt_u64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::Number(n) => n
                .as_u64()
                .or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)),
            Value::String(s) => {
                let s = s.trim();
                s.parse().ok().or_else(|| {
                    s.parse::<f64>()
                        .ok()
                        .filter(|f| *f >= 0.0)
                        .map(|f| f as u64)
                })
            }
            _ => None,
        })
    }

    pub fn opt_bool<'de, D: Deserializer<'de>>(d: D) -> Result<Option<bool>, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::Bool(b) => Some(b),
            Value::Number(n) => n.as_f64().map(|f| f != 0.0),
            Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => Some(true),
                "false" | "0" | "off" | "no" => Some(false),
                _ => None,
            },
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn parses_device_info_fixture() {
        let info = parse_device_info(&fixture("device_info.json")).unwrap();
        assert_eq!(info.model.as_deref(), Some("VIRB 360"));
        assert_eq!(info.firmware.as_deref(), Some("4.30"));
        assert_eq!(info.device_id.as_deref(), Some("3957712345"));
        assert_eq!(info.part_number.as_deref(), Some("006-B2811-00"));
        // Unknown properties are preserved.
        assert_eq!(info.raw["wifiMacAddress"], "00:11:22:33:44:55");
    }

    #[test]
    fn parses_device_info_variants() {
        let info =
            parse_device_info(&json!({"deviceInfo": {"model": "VIRB 360", "firmware": 4.3}}))
                .unwrap();
        assert_eq!(info.firmware.as_deref(), Some("4.3"));
        let info = parse_device_info(&json!({"model": "VIRB 360", "result": 1})).unwrap();
        assert_eq!(info.model.as_deref(), Some("VIRB 360"));
    }

    #[test]
    fn rejects_malformed_device_info() {
        for bad in [
            json!({"deviceInfo": []}),
            json!({"deviceInfo": "VIRB"}),
            json!({"deviceInfo": [42]}),
        ] {
            assert!(matches!(
                parse_device_info(&bad),
                Err(CameraError::MalformedResponse { .. })
            ));
        }
    }

    #[test]
    fn parses_status_fixtures() {
        let idle = parse_status(&fixture("status_idle.json")).unwrap();
        assert_eq!(idle.recording_state, RecordingState::Idle);
        assert_eq!(idle.battery_level, Some(82.0));
        assert_eq!(idle.mode.as_deref(), Some("video"));
        assert_eq!(idle.storage_total_bytes, Some(128_010_158_080));
        assert_eq!(idle.storage_available_bytes, Some(81_920_000_000));

        let recording = parse_status(&fixture("status_recording.json")).unwrap();
        assert_eq!(recording.recording_state, RecordingState::Recording);
        assert_eq!(recording.recording_time_secs, Some(42.0));
        // Battery level sent as a string is still understood.
        assert_eq!(recording.battery_level, Some(79.0));
    }

    #[test]
    fn status_tolerates_missing_and_odd_fields() {
        let status =
            parse_status(&json!({"result": 1, "batteryLevel": "n/a", "totalSpace": -5})).unwrap();
        assert_eq!(status.recording_state, RecordingState::Unknown);
        assert_eq!(status.battery_level, None);
        assert_eq!(status.storage_total_bytes, None);

        let status = parse_status(&json!({"recording": 1})).unwrap();
        assert_eq!(status.recording_state, RecordingState::Recording);
    }

    #[test]
    fn parses_features_fixture() {
        let list = parse_features(&fixture("features.json")).unwrap();
        assert_eq!(list.features.len(), 4);
        let video = list.features.iter().find(|f| f.key == "videoMode").unwrap();
        assert_eq!(video.value, Some(json!("5.7K 30fps")));
        assert_eq!(video.options.len(), 3);
        assert_eq!(video.option_summaries.len(), 3);
        assert_eq!(video.enabled, Some(true));
        let gps = list.features.iter().find(|f| f.key == "gps").unwrap();
        assert_eq!(gps.enabled, Some(true));
        assert_eq!(gps.value, Some(json!("on")));
    }

    #[test]
    fn rejects_malformed_features() {
        assert!(parse_features(&json!({"result": 1})).is_err());
        assert!(parse_features(&json!({"features": {"a": 1}})).is_err());
        // Non-object entries are skipped rather than failing the whole list.
        let list = parse_features(&json!({"features": [1, {"feature": "x"}]})).unwrap();
        assert_eq!(list.features.len(), 1);
    }

    #[test]
    fn parses_media_list_fixture() {
        let items = parse_media_list(&fixture("media_list.json")).unwrap();
        assert_eq!(items.len(), 3);

        let video = &items[0];
        assert_eq!(video.name, "V0010042.MP4");
        assert_eq!(video.media_type, MediaType::Video);
        assert_eq!(video.timestamp, Some(1_720_000_000));
        assert_eq!(
            video.date_time.as_deref(),
            Some("2024-07-03T09:46:40+00:00")
        );
        assert_eq!(video.duration_secs, Some(125.5));
        assert_eq!(video.file_size_bytes, Some(1_048_576_000));
        assert_eq!(video.lens_mode.as_deref(), Some("360"));
        assert!(video.has_fit);
        assert_eq!(
            video.fit_url.as_deref(),
            Some("http://192.168.0.1/GMetrix/2024-07-03-09-46-40.fit")
        );
        assert!(video.low_res_url.is_some());

        let photo = &items[1];
        assert_eq!(photo.media_type, MediaType::Photo);
        assert!(!photo.has_fit, "empty fitURL means no FIT file");
        assert_eq!(photo.duration_secs, None);

        // Entry without a name: derived from URL; string sizes/dates accepted.
        let unnamed = &items[2];
        assert_eq!(unnamed.name, "V0010043.MP4");
        assert_eq!(unnamed.file_size_bytes, Some(2048));
        assert_eq!(unnamed.timestamp, Some(1_720_003_600));
        assert_eq!(unnamed.raw["futureFirmwareField"], json!({"nested": true}));
    }

    #[test]
    fn parses_real_firmware_420_responses() {
        let info = parse_device_info(&fixture("real_fw420/device_info.json")).unwrap();
        assert_eq!(info.model.as_deref(), Some("VIRB 360"));
        assert_eq!(info.firmware.as_deref(), Some("4.20"));
        assert_eq!(info.device_id.as_deref(), Some("3300000001"));
        assert_eq!(info.raw["firmware"], 420);

        let status = parse_status(&fixture("real_fw420/status_recording.json")).unwrap();
        assert_eq!(status.recording_state, RecordingState::Recording);
        assert_eq!(status.recording_time_secs, Some(105.0));
        assert_eq!(status.battery_level, Some(75.0));
        assert_eq!(status.battery_charging_state.as_deref(), Some("0"));
        // ~119 GiB card: KiB converted to bytes.
        assert_eq!(status.storage_total_bytes, Some(125_009_920 * 1024));
        assert_eq!(status.mode, None, "firmware 4.20 reports no mode in status");

        let features = parse_features(&fixture("real_fw420/features.json")).unwrap();
        assert_eq!(features.features.len(), 22);
        let bare = &features.features[0];
        assert_eq!(bare.key, "previewWhileRecording");
        assert_eq!((bare.value.clone(), bare.enabled), (None, None));
        let mode = features
            .features
            .iter()
            .find(|f| f.key == "videoMode")
            .unwrap();
        assert_eq!(mode.options.len(), 3);

        let media = parse_media_list(&fixture("real_fw420/media_list.json")).unwrap();
        assert_eq!(media.len(), 3);
        assert_eq!(media[0].media_type, MediaType::Video);
        assert!(media[0].has_fit);
        assert_eq!(media[0].timestamp, Some(1_613_751_858));
        assert_eq!(media[1].media_type, MediaType::Photo);
        assert_eq!(media[1].lens_mode.as_deref(), Some("frontLensOnly"));
        assert!(media[1].thumbnail_url.as_deref().unwrap().ends_with(".BMP"));
        assert!(!media[1].has_fit);
    }

    #[test]
    fn media_list_edge_cases() {
        assert!(parse_media_list(&json!({"result": 1})).unwrap().is_empty());
        assert!(parse_media_list(&json!({"media": "nope"})).is_err());
        let items = parse_media_list(&json!({"media": [null, 3, {}]})).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "media-2");
        assert_eq!(items[0].media_type, MediaType::Other);
    }

    #[test]
    fn parses_timestamp_formats() {
        assert_eq!(
            parse_timestamp(&json!(1_720_000_000_000u64)),
            Some(1_720_000_000)
        );
        assert_eq!(
            parse_timestamp(&json!("2024-07-03T09:46:40Z")),
            Some(1_720_000_000)
        );
        assert_eq!(
            parse_timestamp(&json!("2024:07:03 09:46:40")),
            Some(1_720_000_000)
        );
        assert_eq!(parse_timestamp(&json!("yesterday")), None);
        assert_eq!(parse_timestamp(&json!(0)), None);
    }
}
