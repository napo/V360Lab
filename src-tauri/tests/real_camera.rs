//! Opt-in checks against a physical VIRB 360. They are `#[ignore]`d so the
//! normal test suite never needs hardware. They only read from the camera
//! (no recording, no deletion) and download into a temporary directory.
//!
//! ```text
//! V360LAB_CAMERA=192.168.0.1 cargo test --test real_camera -- --ignored --nocapture --test-threads=1
//! ```

use std::path::PathBuf;

use v360lab_lib::camera::{CameraClient, MediaItem, MediaType};
use v360lab_lib::downloads::{download_media, DownloadOptions, FileKind};
use v360lab_lib::virb::GarminVirb360Client;

fn camera() -> Option<GarminVirb360Client> {
    match std::env::var("V360LAB_CAMERA") {
        Ok(address) => Some(GarminVirb360Client::new(&address).expect("valid V360LAB_CAMERA")),
        Err(_) => {
            eprintln!("V360LAB_CAMERA not set: skipping hardware test");
            None
        }
    }
}

fn output_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "v360lab-hwtest-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S")
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn smallest(media: &[MediaItem], filter: impl Fn(&MediaItem) -> bool) -> Option<&MediaItem> {
    media
        .iter()
        .filter(|m| filter(m) && m.url.is_some())
        .min_by_key(|m| m.file_size_bytes.unwrap_or(u64::MAX))
}

#[tokio::test]
#[ignore = "requires a VIRB 360 (set V360LAB_CAMERA)"]
async fn read_only_commands() {
    let Some(camera) = camera() else { return };

    let info = camera.device_info().await.expect("deviceInfo");
    println!("device: {info:?}");
    assert!(info.model.is_some());

    let status = camera.status().await.expect("status");
    println!(
        "status: {:?}, battery {:?}%, storage {:?}/{:?} bytes",
        status.recording_state,
        status.battery_level,
        status.storage_available_bytes,
        status.storage_total_bytes
    );

    let features = camera.features().await.expect("features");
    println!("features: {}", features.features.len());

    let commands = camera.supported_commands().await.expect("commandList");
    println!("supported commands: {commands:?}");

    let media = camera.media_list().await.expect("mediaList");
    let videos = media
        .iter()
        .filter(|m| m.media_type == MediaType::Video)
        .count();
    let with_fit = media.iter().filter(|m| m.has_fit).count();
    println!(
        "media: {} items, {videos} videos, {with_fit} with FIT",
        media.len()
    );
}

#[tokio::test]
#[ignore = "requires a VIRB 360 (set V360LAB_CAMERA)"]
async fn downloads_smallest_video_with_fit_and_smallest_photo() {
    let Some(camera) = camera() else { return };
    let media = camera.media_list().await.expect("mediaList");
    let root = output_dir();
    let options = DownloadOptions {
        include_fit: true,
        include_thumbnail: true,
    };

    let candidates = [
        smallest(&media, |m| m.media_type == MediaType::Video && m.has_fit),
        smallest(&media, |m| m.media_type == MediaType::Photo),
    ];
    for item in candidates.into_iter().flatten() {
        println!(
            "downloading {} ({:?} bytes)",
            item.name, item.file_size_bytes
        );
        let report = download_media(&camera, &root, item, options, &|_| {})
            .await
            .expect("download");
        println!("  -> {} {:?}", report.directory, report.warnings);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);

        let media_file = report
            .files
            .iter()
            .find(|f| f.kind == FileKind::Media)
            .unwrap();
        if let Some(expected) = item.file_size_bytes {
            assert_eq!(media_file.bytes, expected, "size differs from mediaList");
        }
        if item.has_fit {
            assert!(report.files.iter().any(|f| f.kind == FileKind::Telemetry));
        }
        assert!(report.files.iter().any(|f| f.kind == FileKind::Metadata));
    }
    println!("files kept in {}", root.display());
}

/// Scans the local network without any known address.
/// `V360LAB_SCAN=1 cargo test --test real_camera scan -- --ignored --nocapture`
#[tokio::test]
#[ignore = "scans the local network (set V360LAB_SCAN=1)"]
async fn scan_finds_camera_on_local_network() {
    if std::env::var("V360LAB_SCAN").is_err() {
        eprintln!("V360LAB_SCAN not set: skipping network scan");
        return;
    }
    let started = std::time::Instant::now();
    let found = v360lab_lib::discovery::discover(&[], &|step| {
        if step.code != "scanProgress" || step.progress.is_some_and(|p| p.done == p.total) {
            println!(
                "{:>6} ms  {} {:?}",
                started.elapsed().as_millis(),
                step.code,
                step.params
            );
        }
    })
    .await;
    println!("found: {found:?} in {} ms", started.elapsed().as_millis());
    assert!(!found.is_empty(), "no camera found");
}

/// Read-only: prints the Wi-Fi networks, so the raw response format can be
/// checked (run with `RUST_LOG=v360lab_lib=debug` to see the JSON).
#[tokio::test]
#[ignore = "requires a VIRB 360 (set V360LAB_CAMERA)"]
async fn reads_wifi_networks() {
    let Some(camera) = camera() else { return };
    let wifi = camera.wifi_networks().await.expect("networks");
    println!("camera network: {:?}", wifi.access_point_ssid);
    println!("configured: {:?}", wifi.configured);
    println!("scanned: {:?}", wifi.scanned);
}
