//! HTTP-level tests for the VIRB client against a mocked camera.
//! No physical VIRB 360 is required.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::{json, Value};
use v360lab_lib::camera::{CameraClient, CameraError, MediaType, RecordingState};
use v360lab_lib::virb::{GarminVirb360Client, VirbClientConfig};
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn client_for(server: &MockServer) -> GarminVirb360Client {
    let config = VirbClientConfig {
        connect_timeout: Duration::from_secs(2),
        command_timeout: Duration::from_millis(500),
        transfer_read_timeout: Duration::from_millis(500),
        media_list_timeout: Duration::from_millis(1500),
    };
    GarminVirb360Client::with_config(&server.uri(), config).unwrap()
}

async fn mount_command(server: &MockServer, command: &str, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/virb"))
        .and(body_json(json!({ "command": command })))
        .respond_with(response)
        .mount(server)
        .await;
}

#[tokio::test]
async fn sends_commands_and_parses_responses() {
    let server = MockServer::start().await;
    mount_command(
        &server,
        "deviceInfo",
        ResponseTemplate::new(200).set_body_json(fixture("device_info.json")),
    )
    .await;
    mount_command(
        &server,
        "status",
        ResponseTemplate::new(200).set_body_json(fixture("status_recording.json")),
    )
    .await;
    mount_command(
        &server,
        "features",
        ResponseTemplate::new(200).set_body_json(fixture("features.json")),
    )
    .await;
    mount_command(
        &server,
        "mediaList",
        ResponseTemplate::new(200).set_body_json(fixture("media_list.json")),
    )
    .await;
    for command in ["startRecording", "stopRecording", "snapPicture"] {
        mount_command(
            &server,
            command,
            ResponseTemplate::new(200).set_body_json(json!({ "result": 1 })),
        )
        .await;
    }
    let client = client_for(&server);

    let info = client.device_info().await.unwrap();
    assert_eq!(info.model.as_deref(), Some("VIRB 360"));

    let status = client.status().await.unwrap();
    assert_eq!(status.recording_state, RecordingState::Recording);

    let features = client.features().await.unwrap();
    assert_eq!(features.features.len(), 4);

    let media = client.media_list().await.unwrap();
    assert_eq!(media.len(), 3);
    assert_eq!(media[0].media_type, MediaType::Video);

    assert_eq!(
        client.start_recording().await.unwrap().command,
        "startRecording"
    );
    assert_eq!(
        client.stop_recording().await.unwrap().command,
        "stopRecording"
    );
    assert_eq!(client.snap_picture().await.unwrap().command, "snapPicture");
}

#[tokio::test]
async fn malformed_json_is_reported() {
    let server = MockServer::start().await;
    mount_command(
        &server,
        "status",
        ResponseTemplate::new(200).set_body_string("{\"result\": 1, \"state\": "),
    )
    .await;
    let err = client_for(&server).status().await.unwrap_err();
    assert!(
        matches!(err, CameraError::MalformedResponse { ref command, .. } if command == "status"),
        "{err:?}"
    );
}

#[tokio::test]
async fn html_error_page_is_malformed_response() {
    let server = MockServer::start().await;
    mount_command(
        &server,
        "deviceInfo",
        ResponseTemplate::new(200).set_body_string("<html>captive portal</html>"),
    )
    .await;
    let err = client_for(&server).device_info().await.unwrap_err();
    assert_eq!(err.kind(), "malformedResponse");
}

#[tokio::test]
async fn result_zero_is_command_failed() {
    let server = MockServer::start().await;
    mount_command(
        &server,
        "startRecording",
        ResponseTemplate::new(200).set_body_json(json!({ "result": 0 })),
    )
    .await;
    let err = client_for(&server).start_recording().await.unwrap_err();
    assert!(matches!(err, CameraError::CommandFailed { .. }), "{err:?}");
}

#[tokio::test]
async fn http_errors_are_typed() {
    let server = MockServer::start().await;
    mount_command(
        &server,
        "status",
        ResponseTemplate::new(500).set_body_string("internal error"),
    )
    .await;
    mount_command(&server, "features", ResponseTemplate::new(404)).await;
    let client = client_for(&server);
    assert!(matches!(
        client.status().await.unwrap_err(),
        CameraError::Http { status: 500, .. }
    ));
    assert!(matches!(
        client.features().await.unwrap_err(),
        CameraError::UnsupportedCommand { .. }
    ));
}

#[tokio::test]
async fn slow_camera_times_out() {
    let server = MockServer::start().await;
    mount_command(
        &server,
        "status",
        ResponseTemplate::new(200)
            .set_body_json(fixture("status_idle.json"))
            .set_delay(Duration::from_secs(2)),
    )
    .await;
    let err = client_for(&server).status().await.unwrap_err();
    assert!(matches!(err, CameraError::Timeout { .. }), "{err:?}");
}

#[tokio::test]
async fn media_list_gets_a_longer_timeout() {
    let server = MockServer::start().await;
    mount_command(
        &server,
        "mediaList",
        ResponseTemplate::new(200)
            .set_body_json(fixture("media_list.json"))
            .set_delay(Duration::from_millis(800)),
    )
    .await;
    // Slower than the command timeout, faster than the media list timeout.
    assert_eq!(client_for(&server).media_list().await.unwrap().len(), 3);
}

#[tokio::test]
async fn unreachable_camera_is_reported() {
    // Bind and immediately drop a listener to get a closed local port.
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let client = GarminVirb360Client::new(&format!("127.0.0.1:{port}")).unwrap();
    let err = client.device_info().await.unwrap_err();
    assert!(matches!(err, CameraError::Unreachable { .. }), "{err:?}");
}

#[tokio::test]
async fn downloads_stream_to_disk_with_progress() {
    let server = MockServer::start().await;
    let payload = vec![7u8; 300_000];
    Mock::given(method("GET"))
        .and(path("/DCIM/100_VIRB/V0010042.MP4"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.clone()))
        .mount(&server)
        .await;
    let client = client_for(&server);
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("V0010042.MP4");
    let last = AtomicU64::new(0);

    // The camera reports its own IP; the client must re-anchor onto the
    // configured host (the mock server).
    let bytes = client
        .download_to(
            "http://192.168.0.1/DCIM/100_VIRB/V0010042.MP4",
            &destination,
            &|received, _total| last.store(received, Ordering::Relaxed),
        )
        .await
        .unwrap();

    assert_eq!(bytes, payload.len() as u64);
    assert_eq!(last.load(Ordering::Relaxed), payload.len() as u64);
    assert_eq!(std::fs::read(&destination).unwrap(), payload);
    assert!(!dir.path().join("V0010042.MP4.part").exists());
}

#[tokio::test]
async fn failed_download_leaves_no_partial_file() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("missing.MP4");
    let err = client_for(&server)
        .download_to("/DCIM/missing.MP4", &destination, &|_, _| {})
        .await
        .unwrap_err();
    assert!(matches!(err, CameraError::Http { status: 404, .. }));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn fetch_resource_enforces_size_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/thumb.jpg"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(vec![1u8; 4096])
                .insert_header("content-type", "image/jpeg"),
        )
        .mount(&server)
        .await;
    let client = client_for(&server);
    let small = client.fetch_resource("/thumb.jpg", 10_000).await.unwrap();
    assert_eq!(small.bytes.len(), 4096);
    assert_eq!(small.content_type.as_deref(), Some("image/jpeg"));
    assert!(matches!(
        client.fetch_resource("/thumb.jpg", 1000).await.unwrap_err(),
        CameraError::TooLarge { .. }
    ));
}

#[test]
fn rejects_invalid_address() {
    assert!(matches!(
        GarminVirb360Client::new("ftp://camera").err(),
        Some(CameraError::InvalidAddress { .. })
    ));
}

#[tokio::test]
async fn update_feature_sends_value_and_verifies_it() {
    let server = MockServer::start().await;
    // Firmware 4.20 answers updateFeature with the full feature list.
    Mock::given(method("POST"))
        .and(path("/virb"))
        .and(body_json(
            json!({ "command": "updateFeature", "feature": "units", "value": "Metric" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("real_fw420/features.json")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/virb"))
        .and(body_json(
            json!({ "command": "updateFeature", "feature": "units", "value": "Statute" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("real_fw420/features.json")))
        .mount(&server)
        .await;
    let client = client_for(&server);

    let list = client.update_feature("units", "Metric").await.unwrap();
    assert_eq!(list.features.len(), 22);
    // The camera answered but kept the old value: reported as a failure.
    let err = client.update_feature("units", "Statute").await.unwrap_err();
    assert!(matches!(err, CameraError::CommandFailed { .. }), "{err:?}");
}

#[tokio::test]
async fn delete_and_stop_still_recording_send_expected_payloads() {
    let server = MockServer::start().await;
    let url = "http://192.168.0.1:80/DCIM/100_VIRB/V0010001.MP4";
    Mock::given(method("POST"))
        .and(path("/virb"))
        .and(body_json(
            json!({ "command": "deleteFile", "files": [url] }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "result": 1 })))
        .expect(1)
        .mount(&server)
        .await;
    mount_command(
        &server,
        "stopStillRecording",
        ResponseTemplate::new(200).set_body_json(json!({ "result": 1 })),
    )
    .await;
    let client = client_for(&server);
    // The URL is sent exactly as reported, not re-anchored.
    let files = [url.to_string()];
    assert_eq!(
        client.delete_files(&files).await.unwrap().command,
        "deleteFile"
    );
    assert_eq!(
        client.stop_still_recording().await.unwrap().command,
        "stopStillRecording"
    );
}

#[tokio::test]
async fn http_400_is_unsupported_command() {
    // Firmware 4.20 answers unknown commands with an nginx 400 page.
    let server = MockServer::start().await;
    mount_command(
        &server,
        "stopStillRecording",
        ResponseTemplate::new(400)
            .set_body_string("<html><head><title>400 Bad Request</title></head></html>"),
    )
    .await;
    let err = client_for(&server)
        .stop_still_recording()
        .await
        .unwrap_err();
    assert!(
        matches!(err, CameraError::UnsupportedCommand { .. }),
        "{err:?}"
    );
}
