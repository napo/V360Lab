//! Simulated VIRB 360 for UI development without the physical camera.
//!
//! Responses are built in the VIRB JSON format and go through the same
//! parsers as real responses, so the mock also exercises [`super::models`].

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;

use crate::camera::{
    CameraClient, CameraError, CameraKind, CameraStatus, CommandAck, DeviceInfo, FeatureList,
    FetchedResource, MediaItem, ProgressFn,
};
use crate::telemetry::fit;

use super::client::partial_path;
use super::models;

const MOCK_ADDRESS: &str = "mock://virb360";
/// Card capacity in bytes.
const TOTAL_SPACE: u64 = 128_010_158_080;
/// Size of the placeholder files written for mock media downloads.
const MOCK_FILE_BYTES: usize = 512 * 1024;

pub struct MockVirb360Client {
    state: Mutex<MockState>,
    latency: Duration,
}

struct MockState {
    recording_started: Option<Instant>,
    next_video: u32,
    next_photo: u32,
    /// Media entries in raw VIRB format.
    media: Vec<Value>,
    /// Feature list in raw VIRB format (modified by `update_feature`).
    features: Value,
}

impl Default for MockVirb360Client {
    fn default() -> Self {
        Self::new()
    }
}

impl MockVirb360Client {
    pub fn new() -> Self {
        Self::with_latency(Duration::from_millis(150))
    }

    pub fn with_latency(latency: Duration) -> Self {
        let media = vec![
            video_entry(42, 1_720_000_000, 125.5, 1_048_576_000, true),
            photo_entry(1, 1_720_000_300),
            video_entry(43, 1_720_003_600, 612.0, 5_133_828_096, true),
            video_entry(44, 1_720_090_000, 18.2, 152_043_520, false),
            photo_entry(2, 1_720_090_120),
            photo_entry(3, 1_720_090_150),
        ];
        Self {
            state: Mutex::new(MockState {
                recording_started: None,
                next_video: 45,
                next_photo: 4,
                media,
                features: mock_features(),
            }),
            latency,
        }
    }

    async fn simulate_latency(&self) {
        tokio::time::sleep(self.latency).await;
    }

    fn with_state<T>(&self, f: impl FnOnce(&mut MockState) -> T) -> T {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut state)
    }

    fn failed(command: &str) -> CameraError {
        CameraError::CommandFailed {
            command: command.to_string(),
            response: json!({ "result": 0 }).to_string(),
        }
    }
}

#[async_trait]
impl CameraClient for MockVirb360Client {
    fn kind(&self) -> CameraKind {
        CameraKind::Mock
    }

    fn address(&self) -> String {
        MOCK_ADDRESS.to_string()
    }

    async fn device_info(&self) -> Result<DeviceInfo, CameraError> {
        self.simulate_latency().await;
        models::parse_device_info(&json!({
            "deviceInfo": [{
                "deviceId": "3999900001",
                "firmware": "4.30",
                "model": "VIRB 360 (mock)",
                "partNumber": "006-B2811-00",
                "type": "camera",
                "mockDevice": true
            }],
            "result": 1
        }))
    }

    async fn status(&self) -> Result<CameraStatus, CameraError> {
        self.simulate_latency().await;
        let (recording_secs, used): (Option<u64>, u64) = self.with_state(|s| {
            let used = s.media.iter().filter_map(|m| m["fileSize"].as_u64()).sum();
            (s.recording_started.map(|t| t.elapsed().as_secs()), used)
        });
        let available = TOTAL_SPACE.saturating_sub(used + 12_000_000_000);
        models::parse_status(&json!({
            "apiMax": "2.20",
            "apiMin": "1.00",
            // The VIRB reports storage in KiB.
            "availableSpace": available / 1024,
            "batteryChargingState": "discharging",
            "batteryLevel": 87,
            "gpsLatitude": 46.0664,
            "gpsLongitude": 11.1257,
            "gpsSatelliteFix": 1,
            "recordingTime": recording_secs.unwrap_or(0),
            "recordingTimeRemaining": available / 45_000_000,
            "state": if recording_secs.is_some() { "recording" } else { "idle" },
            "totalSpace": TOTAL_SPACE / 1024,
            "wifiSignalStrength": -51,
            "result": 1
        }))
    }

    async fn features(&self) -> Result<FeatureList, CameraError> {
        self.simulate_latency().await;
        models::parse_features(&self.with_state(|s| s.features.clone()))
    }

    async fn update_feature(&self, key: &str, value: &str) -> Result<FeatureList, CameraError> {
        self.simulate_latency().await;
        let features = self.with_state(|s| {
            let entry = s.features["features"]
                .as_array_mut()
                .and_then(|list| list.iter_mut().find(|f| f["feature"] == key))
                .ok_or_else(|| Self::failed("updateFeature"))?;
            if !mock_value_allowed(entry, value) {
                return Err(Self::failed("updateFeature"));
            }
            entry["value"] = Value::from(value);
            Ok(s.features.clone())
        })?;
        let list = models::parse_features(&features)?;
        models::check_feature_value(&list, key, value)?;
        Ok(list)
    }

    async fn start_recording(&self) -> Result<CommandAck, CameraError> {
        self.simulate_latency().await;
        let command = "startRecording";
        self.with_state(|s| {
            if s.recording_started.is_some() {
                return Err(Self::failed(command));
            }
            s.recording_started = Some(Instant::now());
            Ok(())
        })?;
        Ok(models::command_ack(command, json!({ "result": 1 })))
    }

    async fn stop_recording(&self) -> Result<CommandAck, CameraError> {
        self.simulate_latency().await;
        let command = "stopRecording";
        self.with_state(|s| {
            let started = s
                .recording_started
                .take()
                .ok_or_else(|| Self::failed(command))?;
            let duration = started.elapsed().as_secs_f64().max(1.0);
            let index = s.next_video;
            s.next_video += 1;
            let size = (duration * 45_000_000.0) as u64;
            s.media.push(video_entry(
                index,
                Utc::now().timestamp(),
                duration,
                size,
                true,
            ));
            Ok(())
        })?;
        Ok(models::command_ack(command, json!({ "result": 1 })))
    }

    async fn snap_picture(&self) -> Result<CommandAck, CameraError> {
        self.simulate_latency().await;
        let entry = self.with_state(|s| {
            let index = s.next_photo;
            s.next_photo += 1;
            let entry = photo_entry(index, Utc::now().timestamp());
            s.media.push(entry.clone());
            entry
        });
        Ok(models::command_ack(
            "snapPicture",
            json!({ "result": 1, "media": entry }),
        ))
    }

    async fn stop_still_recording(&self) -> Result<CommandAck, CameraError> {
        self.simulate_latency().await;
        Ok(models::command_ack(
            "stopStillRecording",
            json!({ "result": 1 }),
        ))
    }

    async fn delete_files(&self, media_urls: &[String]) -> Result<CommandAck, CameraError> {
        self.simulate_latency().await;
        // Like firmware 4.20: success even when a file does not exist.
        self.with_state(|s| {
            s.media
                .retain(|m| !media_urls.iter().any(|url| m["url"] == url.as_str()))
        });
        Ok(models::command_ack("deleteFile", json!({ "result": 1 })))
    }

    async fn media_list(&self) -> Result<Vec<MediaItem>, CameraError> {
        self.simulate_latency().await;
        let media = self.with_state(|s| s.media.clone());
        models::parse_media_list(&json!({ "media": media, "result": 1 }))
    }

    async fn fetch_resource(
        &self,
        url: &str,
        max_bytes: u64,
    ) -> Result<FetchedResource, CameraError> {
        let name = url.rsplit('/').next().unwrap_or(url);
        let svg = thumbnail_svg(name);
        if svg.len() as u64 > max_bytes {
            return Err(CameraError::TooLarge {
                size: svg.len() as u64,
                limit: max_bytes,
            });
        }
        Ok(FetchedResource {
            bytes: svg.into_bytes(),
            content_type: Some("image/svg+xml".to_string()),
        })
    }

    async fn download_to(
        &self,
        url: &str,
        destination: &Path,
        progress: ProgressFn<'_>,
    ) -> Result<u64, CameraError> {
        let content = if url.to_ascii_lowercase().ends_with(".fit") {
            fit::empty_fit_file()
        } else {
            let line = format!("V360Lab mock media placeholder for {url}\n");
            line.as_bytes()
                .iter()
                .copied()
                .cycle()
                .take(MOCK_FILE_BYTES)
                .collect()
        };
        let total = content.len() as u64;
        let partial = partial_path(destination);
        let io_error = |source| CameraError::Io {
            path: partial.clone(),
            source,
        };

        let mut file = tokio::fs::File::create(&partial).await.map_err(io_error)?;
        let mut written = 0u64;
        // Write in chunks with small pauses so progress reporting is visible.
        for chunk in content.chunks(32 * 1024) {
            file.write_all(chunk).await.map_err(io_error)?;
            written += chunk.len() as u64;
            progress(written, Some(total));
            tokio::time::sleep(self.latency / 4).await;
        }
        file.flush().await.map_err(io_error)?;
        drop(file);
        tokio::fs::rename(&partial, destination)
            .await
            .map_err(|source| CameraError::Io {
                path: destination.to_path_buf(),
                source,
            })?;
        Ok(written)
    }
}

fn video_entry(index: u32, date: i64, duration: f64, size: u64, has_fit: bool) -> Value {
    let name = format!("V00{index:05}.MP4");
    let base = format!("{MOCK_ADDRESS}/DCIM/100_VIRB");
    json!({
        "date": date,
        "duration": duration,
        "fileSize": size,
        "fitURL": if has_fit { format!("{MOCK_ADDRESS}/GMetrix/V00{index:05}.fit") } else { String::new() },
        "lensMode": "360",
        "lowResVideoPath": format!("{base}/V00{index:05}.GLV"),
        "name": name,
        "thumbUrl": format!("{base}/V00{index:05}.THM"),
        "type": "video",
        "url": format!("{base}/{name}")
    })
}

fn photo_entry(index: u32, date: i64) -> Value {
    let name = format!("V02{index:05}.JPG");
    let base = format!("{MOCK_ADDRESS}/DCIM/100_VIRB");
    json!({
        "date": date,
        "fileSize": 7_340_032,
        "fitURL": "",
        "lensMode": "360",
        "name": name,
        "thumbUrl": format!("{base}/{name}?thumb=1"),
        "type": "photo",
        "url": format!("{base}/{name}")
    })
}

/// Feature list captured from a real VIRB 360 (firmware 4.20).
fn mock_features() -> Value {
    serde_json::from_str(include_str!("mock_features.json")).expect("valid mock features")
}

/// Mirrors the camera's constraints: choices must be one of the options,
/// toggles are "0"/"1", actions (type 0) have no value.
fn mock_value_allowed(feature: &Value, value: &str) -> bool {
    match feature["type"].as_i64() {
        Some(0) => false,
        Some(2) => value == "0" || value == "1",
        _ => match feature["options"].as_array() {
            Some(options) => options.iter().any(|o| o == value),
            None => !value.is_empty(),
        },
    }
}

fn thumbnail_svg(name: &str) -> String {
    let label: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.')
        .take(24)
        .collect();
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="320" height="160" viewBox="0 0 320 160"><defs><linearGradient id="g" x1="0" x2="1"><stop offset="0" stop-color="#1c3b4a"/><stop offset="1" stop-color="#3f6f5f"/></linearGradient></defs><rect width="320" height="160" fill="url(#g)"/><path d="M0 120 Q80 90 160 110 T320 100 V160 H0Z" fill="#23303a"/><text x="160" y="84" font-family="monospace" font-size="16" fill="#e8eef2" text-anchor="middle">{label}</text></svg>"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::MediaType;

    fn client() -> MockVirb360Client {
        MockVirb360Client::with_latency(Duration::ZERO)
    }

    #[tokio::test]
    async fn returns_realistic_data() {
        let mock = client();
        let info = mock.device_info().await.unwrap();
        assert_eq!(info.model.as_deref(), Some("VIRB 360 (mock)"));
        let features = mock.features().await.unwrap();
        assert_eq!(features.features.len(), 40);
        let media = mock.media_list().await.unwrap();
        assert_eq!(media.len(), 6);
        assert!(media.iter().any(|m| m.has_fit));
        assert!(media.iter().any(|m| m.media_type == MediaType::Photo));
    }

    #[tokio::test]
    async fn recording_cycle_adds_media() {
        let mock = client();
        mock.start_recording().await.unwrap();
        assert_eq!(
            mock.status().await.unwrap().recording_state,
            crate::camera::RecordingState::Recording
        );
        assert!(mock.start_recording().await.is_err());
        mock.stop_recording().await.unwrap();
        assert!(mock.stop_recording().await.is_err());
        mock.snap_picture().await.unwrap();
        assert_eq!(mock.media_list().await.unwrap().len(), 8);
    }

    #[tokio::test]
    async fn updates_features_like_the_camera() {
        let mock = client();
        let list = mock
            .update_feature("shootingMode", "photoShootingMode")
            .await
            .unwrap();
        let mode = list
            .features
            .iter()
            .find(|f| f.key == "shootingMode")
            .unwrap();
        assert_eq!(mode.value, Some(json!("photoShootingMode")));
        assert!(mock
            .update_feature("video360Format", "sideways")
            .await
            .is_err());
        assert!(mock.update_feature("gps", "2").await.is_err());
        assert!(mock.update_feature("locateCamera", "1").await.is_err());
        assert!(mock.update_feature("noSuchFeature", "1").await.is_err());
        assert!(mock.update_feature("gps", "0").await.is_ok());
    }

    #[tokio::test]
    async fn deletes_files() {
        let mock = client();
        let first = mock.media_list().await.unwrap().remove(0);
        mock.delete_files(&[first.url.clone().unwrap()])
            .await
            .unwrap();
        let media = mock.media_list().await.unwrap();
        assert_eq!(media.len(), 5);
        assert!(media.iter().all(|m| m.id != first.id));
        // Unknown files are "deleted" successfully, as on the real camera.
        let unknown = ["mock://virb360/nope.MP4".to_string()];
        assert!(mock.delete_files(&unknown).await.is_ok());
    }

    #[tokio::test]
    async fn downloads_placeholder_and_valid_fit() {
        let mock = client();
        let dir = tempfile::tempdir().unwrap();
        let fit_path = dir.path().join("track.fit");
        mock.download_to("mock://virb360/GMetrix/V0000042.fit", &fit_path, &|_, _| {})
            .await
            .unwrap();
        assert!(fit::inspect_file(&fit_path).await.is_ok());

        let video_path = dir.path().join("video.mp4");
        let bytes = mock
            .download_to(
                "mock://virb360/DCIM/100_VIRB/V0000042.MP4",
                &video_path,
                &|_, _| {},
            )
            .await
            .unwrap();
        assert_eq!(bytes, MOCK_FILE_BYTES as u64);
        assert!(!partial_path(&video_path).exists());
    }
}
