//! Live preview against a real camera. Run with:
//! V360LAB_CAMERA=192.168.1.200 PREVIEW_OUT=/tmp/p.h264 cargo test --test real_preview -- --ignored --nocapture

use std::sync::{Arc, Mutex};
use std::time::Duration;

use v360lab_lib::camera::CameraClient;
use v360lab_lib::preview::{PreviewManager, MSG_CONFIG, MSG_FRAME};
use v360lab_lib::virb::GarminVirb360Client;

#[tokio::test]
#[ignore = "needs a real camera"]
async fn records_three_seconds_of_preview() {
    let address = std::env::var("V360LAB_CAMERA").expect("V360LAB_CAMERA");
    let out = std::env::var("PREVIEW_OUT").expect("PREVIEW_OUT");
    let camera = GarminVirb360Client::new(&address).unwrap();
    let url = camera.live_preview_url().await.unwrap().expect("preview URL");
    println!("preview URL: {url}");

    let messages = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let sink_messages = messages.clone();
    let manager = PreviewManager::default();
    manager
        .start(
            &url,
            Box::new(move |m| {
                sink_messages.lock().unwrap().push(m);
                true
            }),
            Box::new(|| {}),
        )
        .await
        .unwrap();
    let secs = std::env::var("PREVIEW_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
    tokio::time::sleep(Duration::from_secs(secs)).await;
    manager.stop().await;

    let messages = messages.lock().unwrap();
    let mut stream = Vec::new();
    let (mut frames, mut keyframes) = (0, 0);
    for m in messages.iter() {
        match m[0] {
            MSG_CONFIG => println!("config: {}", String::from_utf8_lossy(&m[1..])),
            MSG_FRAME => {
                frames += 1;
                keyframes += m[1] as usize;
                stream.extend_from_slice(&m[6..]);
            }
            _ => println!("ended: {}", String::from_utf8_lossy(&m[1..])),
        }
    }
    println!("{frames} frames, {keyframes} keyframes, {} bytes", stream.len());
    std::fs::write(out, stream).unwrap();
    assert!(frames > 20);
}
