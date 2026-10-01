//! `virb://` URLs: camera media for `<video>` and `<img>` elements.
//!
//! The UI never talks to the camera itself. To play a video or show a full
//! photo it uses `virb://localhost/<percent-encoded camera URL>` (Windows
//! and Android: `http://virb.localhost/…`, as built by Tauri's
//! `convertFileSrc(url, "virb")`). This handler forwards the request to the
//! connected camera, with Range support so that players can seek, and only
//! ever reaches the camera's own address (see
//! [`crate::camera::address::resolve_camera_url`]).

use percent_encoding::percent_decode_str;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Manager};

use crate::camera::ByteRange;
use crate::state::AppState;

pub const SCHEME: &str = "virb";

/// Largest answer to an open-ended Range request: players ask again for
/// the rest, and a smaller answer starts playback sooner.
const CHUNK_BYTES: u64 = 4 * 1024 * 1024;
/// Largest resource served whole (a request without Range, e.g. a photo).
const MAX_WHOLE_BYTES: u64 = 64 * 1024 * 1024;

pub async fn handle(app: &AppHandle, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Some(url) = camera_url(request.uri().path()) else {
        return text(StatusCode::BAD_REQUEST, "invalid camera URL");
    };
    let range = request
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_range);
    let state = app.state::<AppState>();
    let camera = match state.camera().await {
        Ok(camera) => camera,
        Err(e) => return text(StatusCode::SERVICE_UNAVAILABLE, &e.to_string()),
    };
    let max = if range.is_some() {
        CHUNK_BYTES
    } else {
        MAX_WHOLE_BYTES
    };
    let resource = match camera.fetch_range(&url, range, max).await {
        Ok(resource) => resource,
        Err(e) => {
            log::warn!("virb:// {url}: {e}");
            return text(StatusCode::BAD_GATEWAY, &e.to_string());
        }
    };

    let content_type = content_type(&url, resource.content_type.as_deref());
    let length = resource.bytes.len() as u64;
    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, length)
        // Lets WebGL use the frames of a 360° video or photo as a texture.
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
    if range.is_some() {
        let end = resource.start + length.saturating_sub(1);
        let total = resource.total.map_or("*".to_string(), |t| t.to_string());
        builder = builder.status(StatusCode::PARTIAL_CONTENT).header(
            header::CONTENT_RANGE,
            format!("bytes {}-{end}/{total}", resource.start),
        );
    }
    builder
        .body(resource.bytes)
        .unwrap_or_else(|_| text(StatusCode::INTERNAL_SERVER_ERROR, "invalid response"))
}

/// `/http%3A%2F%2F192.168.0.1%2FDCIM%2F…` -> `http://192.168.0.1/DCIM/…`
fn camera_url(path: &str) -> Option<String> {
    let encoded = path.strip_prefix('/')?;
    let url = percent_decode_str(encoded).decode_utf8().ok()?.into_owned();
    (url.starts_with("http://") || url.starts_with("https://")).then_some(url)
}

/// `bytes=100-` / `bytes=100-199`. Suffix and multiple ranges are not used
/// by media players for a first request, so they get the whole resource.
fn parse_range(value: &str) -> Option<ByteRange> {
    let spec = value.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (start, end) = spec.split_once('-')?;
    let start = start.trim().parse().ok()?;
    let end = match end.trim() {
        "" => None,
        end => Some(end.parse().ok()?),
    };
    match end {
        Some(end) if end < start => None,
        _ => Some(ByteRange { start, end }),
    }
}

/// The camera's server sends `application/octet-stream` for some files
/// (e.g. `.GLV`, its low-resolution MP4 copy of each video).
fn content_type(url: &str, reported: Option<&str>) -> String {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();
    let by_extension = match path.rsplit('.').next() {
        Some("mp4" | "glv" | "m4v") => Some("video/mp4"),
        Some("jpg" | "jpeg" | "thm") => Some("image/jpeg"),
        Some("bmp") => Some("image/bmp"),
        Some("png") => Some("image/png"),
        _ => None,
    };
    match (reported, by_extension) {
        (_, Some(known)) => known.to_string(),
        (Some(reported), None) => reported.to_string(),
        (None, None) => "application/octet-stream".to_string(),
    }
}

fn text(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(message.as_bytes().to_vec())
        .expect("static response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_camera_urls() {
        assert_eq!(
            camera_url("/http%3A%2F%2F192.168.0.1%3A80%2FDCIM%2F100_VIRB%2FV0010001.GLV")
                .as_deref(),
            Some("http://192.168.0.1:80/DCIM/100_VIRB/V0010001.GLV")
        );
        assert_eq!(camera_url("/file%3A%2F%2F%2Fetc%2Fpasswd"), None);
        assert_eq!(camera_url("no-slash"), None);
    }

    #[test]
    fn parses_ranges() {
        assert_eq!(
            parse_range("bytes=0-"),
            Some(ByteRange {
                start: 0,
                end: None
            })
        );
        assert_eq!(
            parse_range("bytes=100-199"),
            Some(ByteRange {
                start: 100,
                end: Some(199)
            })
        );
        assert_eq!(parse_range("bytes=-500"), None);
        assert_eq!(parse_range("bytes=0-1,5-6"), None);
        assert_eq!(parse_range("bytes=9-1"), None);
    }

    #[test]
    fn picks_content_types() {
        assert_eq!(
            content_type("http://c/V1.GLV", Some("application/octet-stream")),
            "video/mp4"
        );
        assert_eq!(content_type("http://c/V1.JPG?x=1", None), "image/jpeg");
        assert_eq!(
            content_type("http://c/file.fit", Some("application/x-fit")),
            "application/x-fit"
        );
    }
}
