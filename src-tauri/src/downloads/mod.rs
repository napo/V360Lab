//! Local storage of media downloaded from the camera.
//!
//! Layout (dates in UTC, from the camera-reported capture time):
//!
//! ```text
//! <download root>/
//!   2024-07-03/
//!     V0010042/
//!       V0010042.MP4          original media file
//!       2024-07-03-09-46-40.fit  FIT telemetry, when available
//!       thumbnail.jpg         when available
//!       metadata.json         camera metadata + download record
//! ```

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};

use crate::camera::{CameraClient, MediaItem};
use crate::error::{AppError, Resource};
use crate::telemetry::fit;

/// Upper bound for thumbnails saved next to media.
const MAX_THUMBNAIL_BYTES: u64 = 5 * 1024 * 1024;
/// Minimum number of bytes between two progress notifications.
const PROGRESS_STEP_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Media,
    Telemetry,
    Thumbnail,
    Metadata,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedFile {
    pub kind: FileKind,
    pub path: String,
    pub bytes: u64,
    /// True when an identical file already existed and was kept.
    pub skipped: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadReport {
    pub item_id: String,
    pub directory: String,
    pub files: Vec<DownloadedFile>,
    /// Non-fatal problems (e.g. thumbnail unavailable, FIT header invalid).
    pub warnings: Vec<DownloadWarning>,
}

/// Non-fatal download problem. `code` is stable and translated by the UI;
/// `detail` is technical English text.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadWarning {
    pub code: &'static str,
    pub detail: String,
}

impl DownloadWarning {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub item_id: String,
    pub kind: FileKind,
    pub file_name: String,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
}

pub type ProgressSink<'a> = &'a (dyn Fn(DownloadProgress) + Send + Sync);

#[derive(Debug, Clone, Copy)]
pub struct DownloadOptions {
    pub include_fit: bool,
    pub include_thumbnail: bool,
}

/// `<root>/<YYYY-MM-DD>/<recording name>` for a media item.
pub fn item_directory(root: &Path, item: &MediaItem) -> PathBuf {
    let date = item
        .timestamp
        .and_then(|t| DateTime::<Utc>::from_timestamp(t, 0))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "undated".to_string());
    let name = sanitize_file_name(&item.name);
    let stem = match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_string(),
        _ => name,
    };
    root.join(date).join(stem)
}

/// Makes a camera-provided name safe to use as a single path component.
pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_start_matches('.');
    let limited: String = trimmed.chars().take(120).collect();
    if limited.is_empty() || limited.chars().all(|c| c == '_') {
        "unnamed".to_string()
    } else {
        limited
    }
}

/// Downloads the media file, and optionally FIT and thumbnail, then writes
/// `metadata.json`. Only the media file is mandatory; problems with the
/// optional files are reported as warnings.
pub async fn download_media(
    client: &dyn CameraClient,
    root: &Path,
    item: &MediaItem,
    options: DownloadOptions,
    progress: ProgressSink<'_>,
) -> Result<DownloadReport, AppError> {
    let url = item
        .url
        .as_deref()
        .ok_or_else(|| AppError::MissingResource {
            name: item.name.clone(),
            resource: Resource::DownloadUrl,
        })?;
    let directory = prepare_directory(root, item).await?;
    let mut report = new_report(item, &directory);

    let media_path = directory.join(sanitize_file_name(&item.name));
    let media = download_file(
        client,
        url,
        &media_path,
        item,
        FileKind::Media,
        item.file_size_bytes,
        progress,
    )
    .await?;
    report.files.push(media);

    if options.include_fit {
        if item.fit_url.is_some() {
            match download_fit_into(client, &directory, item, progress).await {
                Ok((file, warning)) => {
                    report.files.push(file);
                    report.warnings.extend(warning);
                }
                Err(e) => report
                    .warnings
                    .push(DownloadWarning::new("fitFailed", e.to_string())),
            }
        } else {
            log::debug!("{} has no associated FIT file", item.name);
        }
    }

    if options.include_thumbnail {
        if let Some(thumbnail_url) = &item.thumbnail_url {
            match save_thumbnail(client, thumbnail_url, &directory).await {
                Ok(file) => report.files.push(file),
                Err(e) => report
                    .warnings
                    .push(DownloadWarning::new("thumbnailFailed", e.to_string())),
            }
        }
    }

    report
        .files
        .push(write_metadata(client, item, &directory, &report).await?);
    Ok(report)
}

/// Downloads only the FIT file associated with a media item.
pub async fn download_fit(
    client: &dyn CameraClient,
    root: &Path,
    item: &MediaItem,
    progress: ProgressSink<'_>,
) -> Result<DownloadReport, AppError> {
    if item.fit_url.is_none() {
        return Err(AppError::MissingResource {
            name: item.name.clone(),
            resource: Resource::FitFile,
        });
    }
    let directory = prepare_directory(root, item).await?;
    let mut report = new_report(item, &directory);
    let (file, warning) = download_fit_into(client, &directory, item, progress).await?;
    report.files.push(file);
    report.warnings.extend(warning);
    if !directory.join("metadata.json").exists() {
        report
            .files
            .push(write_metadata(client, item, &directory, &report).await?);
    }
    Ok(report)
}

async fn prepare_directory(root: &Path, item: &MediaItem) -> Result<PathBuf, AppError> {
    let directory = item_directory(root, item);
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(AppError::fs(&directory))?;
    Ok(directory)
}

fn new_report(item: &MediaItem, directory: &Path) -> DownloadReport {
    DownloadReport {
        item_id: item.id.clone(),
        directory: directory.display().to_string(),
        files: Vec::new(),
        warnings: Vec::new(),
    }
}

async fn download_fit_into(
    client: &dyn CameraClient,
    directory: &Path,
    item: &MediaItem,
    progress: ProgressSink<'_>,
) -> Result<(DownloadedFile, Option<DownloadWarning>), AppError> {
    let url = item
        .fit_url
        .as_deref()
        .ok_or_else(|| AppError::MissingResource {
            name: item.name.clone(),
            resource: Resource::FitFile,
        })?;
    let path = directory.join(fit_file_name(url));
    let file = download_file(
        client,
        url,
        &path,
        item,
        FileKind::Telemetry,
        None,
        progress,
    )
    .await?;
    let warning = match fit::inspect_file(&path).await {
        Ok(header) => {
            log::debug!("FIT header for {}: {header:?}", item.name);
            None
        }
        Err(e) => Some(DownloadWarning::new(
            "fitInvalid",
            format!("{}: {e}", path.display()),
        )),
    };
    Ok((file, warning))
}

fn fit_file_name(url: &str) -> String {
    let segment = url
        .split(['?', '#'])
        .next()
        .and_then(|path| path.rsplit('/').find(|s| !s.is_empty()))
        .map(sanitize_file_name);
    match segment {
        Some(name) if name.to_ascii_lowercase().ends_with(".fit") => name,
        _ => "telemetry.fit".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
async fn download_file(
    client: &dyn CameraClient,
    url: &str,
    path: &Path,
    item: &MediaItem,
    kind: FileKind,
    expected_size: Option<u64>,
    progress: ProgressSink<'_>,
) -> Result<DownloadedFile, AppError> {
    // Keep an existing complete copy instead of downloading gigabytes again.
    if let (Some(expected), Ok(meta)) = (expected_size, tokio::fs::metadata(path).await) {
        if meta.is_file() && meta.len() == expected {
            log::info!("Keeping existing {}", path.display());
            return Ok(DownloadedFile {
                kind,
                path: path.display().to_string(),
                bytes: expected,
                skipped: true,
            });
        }
    }

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let last_reported = AtomicU64::new(0);
    let on_progress = |received: u64, total: Option<u64>| {
        let previous = last_reported.load(Ordering::Relaxed);
        let finished = total.is_some_and(|t| received >= t);
        if received - previous.min(received) >= PROGRESS_STEP_BYTES || finished {
            last_reported.store(received, Ordering::Relaxed);
            progress(DownloadProgress {
                item_id: item.id.clone(),
                kind,
                file_name: file_name.clone(),
                received_bytes: received,
                total_bytes: total,
            });
        }
    };
    let bytes = client.download_to(url, path, &on_progress).await?;
    Ok(DownloadedFile {
        kind,
        path: path.display().to_string(),
        bytes,
        skipped: false,
    })
}

async fn save_thumbnail(
    client: &dyn CameraClient,
    url: &str,
    directory: &Path,
) -> Result<DownloadedFile, AppError> {
    let resource = client.fetch_resource(url, MAX_THUMBNAIL_BYTES).await?;
    let extension = match resource
        .content_type
        .as_deref()
        .map(|t| t.split(';').next().unwrap_or(t).trim())
    {
        Some("image/png") => "png",
        Some("image/bmp") => "bmp",
        Some("image/svg+xml") => "svg",
        Some("image/webp") => "webp",
        _ => "jpg",
    };
    let path = directory.join(format!("thumbnail.{extension}"));
    tokio::fs::write(&path, &resource.bytes)
        .await
        .map_err(AppError::fs(&path))?;
    Ok(DownloadedFile {
        kind: FileKind::Thumbnail,
        path: path.display().to_string(),
        bytes: resource.bytes.len() as u64,
        skipped: false,
    })
}

/// Writes `metadata.json` with the untouched camera metadata (`cameraMetadata`)
/// plus V360Lab's normalized view and a record of the downloaded files.
async fn write_metadata(
    client: &dyn CameraClient,
    item: &MediaItem,
    directory: &Path,
    report: &DownloadReport,
) -> Result<DownloadedFile, AppError> {
    let mut normalized = serde_json::to_value(item).unwrap_or(Value::Null);
    if let Value::Object(map) = &mut normalized {
        map.remove("raw");
    }
    let files: Vec<Value> = report
        .files
        .iter()
        .map(|f| {
            json!({
                "kind": f.kind,
                "fileName": Path::new(&f.path).file_name().map(|n| n.to_string_lossy().into_owned()),
                "bytes": f.bytes,
            })
        })
        .collect();
    let document = json!({
        "schema": "v360lab.media-metadata/1",
        "generator": { "name": "V360Lab", "version": env!("CARGO_PKG_VERSION") },
        "downloadedAt": Utc::now().to_rfc3339(),
        "camera": { "kind": client.kind(), "address": client.address() },
        "cameraMetadata": item.raw,
        "normalized": normalized,
        "files": files,
        "warnings": report.warnings,
    });
    let path = directory.join("metadata.json");
    let text = serde_json::to_string_pretty(&document).unwrap_or_default();
    tokio::fs::write(&path, &text)
        .await
        .map_err(AppError::fs(&path))?;
    Ok(DownloadedFile {
        kind: FileKind::Metadata,
        path: path.display().to_string(),
        bytes: text.len() as u64,
        skipped: false,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use super::*;
    use crate::virb::MockVirb360Client;

    fn options() -> DownloadOptions {
        DownloadOptions {
            include_fit: true,
            include_thumbnail: true,
        }
    }

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_file_name("V0010042.MP4"), "V0010042.MP4");
        assert_eq!(sanitize_file_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize_file_name("..."), "unnamed");
        assert_eq!(sanitize_file_name("a b/c"), "a_b_c");
        assert_eq!(sanitize_file_name(""), "unnamed");
    }

    #[test]
    fn fit_names_are_sanitized() {
        assert_eq!(
            fit_file_name("http://x/GMetrix/2024-07-03.fit?x=1"),
            "2024-07-03.fit"
        );
        assert_eq!(fit_file_name("http://x/fit?id=4"), "telemetry.fit");
    }

    #[tokio::test]
    async fn downloads_media_fit_thumbnail_and_metadata() {
        let camera = MockVirb360Client::with_latency(Duration::ZERO);
        let item = camera.media_list().await.unwrap().remove(0);
        assert!(item.has_fit);
        let root = tempfile::tempdir().unwrap();
        let events = Mutex::new(Vec::new());

        let report = download_media(&camera, root.path(), &item, options(), &|p| {
            events.lock().unwrap().push(p)
        })
        .await
        .unwrap();

        let dir = root.path().join("2024-07-03").join("V0000042");
        assert_eq!(PathBuf::from(&report.directory), dir);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert!(dir.join("V0000042.MP4").is_file());
        assert!(dir.join("V0000042.fit").is_file());
        assert!(dir.join("thumbnail.svg").is_file());
        let kinds: Vec<FileKind> = report.files.iter().map(|f| f.kind).collect();
        assert_eq!(
            kinds,
            [
                FileKind::Media,
                FileKind::Telemetry,
                FileKind::Thumbnail,
                FileKind::Metadata
            ]
        );

        let metadata: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("metadata.json")).unwrap())
                .unwrap();
        assert_eq!(metadata["cameraMetadata"], item.raw);
        assert_eq!(metadata["camera"]["kind"], "mock");
        assert!(metadata["normalized"].get("raw").is_none());

        let events = events.into_inner().unwrap();
        assert!(events.iter().any(|e| e.kind == FileKind::Media));
    }

    #[tokio::test]
    async fn fit_only_download_and_missing_fit() {
        let camera = MockVirb360Client::with_latency(Duration::ZERO);
        let media = camera.media_list().await.unwrap();
        let root = tempfile::tempdir().unwrap();

        let with_fit = media.iter().find(|m| m.has_fit).unwrap();
        let report = download_fit(&camera, root.path(), with_fit, &|_| {})
            .await
            .unwrap();
        assert_eq!(report.files[0].kind, FileKind::Telemetry);
        assert!(report.files.iter().any(|f| f.kind == FileKind::Metadata));

        let without_fit = media.iter().find(|m| !m.has_fit).unwrap();
        let err = download_fit(&camera, root.path(), without_fit, &|_| {})
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "missingResource");
    }

    #[tokio::test]
    async fn missing_url_is_an_error() {
        let camera = MockVirb360Client::with_latency(Duration::ZERO);
        let mut item = camera.media_list().await.unwrap().remove(0);
        item.url = None;
        let root = tempfile::tempdir().unwrap();
        let err = download_media(&camera, root.path(), &item, options(), &|_| {})
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "missingResource");
    }
}
