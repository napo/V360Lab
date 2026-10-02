//! Detection models V360Lab can download for the user.
//!
//! The files are published as assets of a dedicated release of the V360Lab
//! repository (`models-v1`). Their size and SHA-256 are fixed here: a
//! download is accepted only if it matches, so a changed or corrupted file
//! is never used. This is the only time V360Lab contacts the internet, and
//! only when the user asks for it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub file_name: &'static str,
    pub url: &'static str,
    pub size_bytes: u64,
    pub sha256: &'static str,
    /// Number of object classes it recognises.
    pub classes: u32,
    pub license: &'static str,
}

/// Models offered for download.
pub const CATALOG: &[ModelInfo] = &[ModelInfo {
    id: "yolo11n-coco",
    name: "YOLO11n (COCO)",
    file_name: "yolo11n.onnx",
    url: "https://github.com/napo/V360Lab/releases/download/models-v1/yolo11n.onnx",
    size_bytes: 10_741_196,
    sha256: "4e59bcb1a82604a26303a55ce00fa5277a4822614dd9794aba26d021c4735533",
    classes: 80,
    license: "AGPL-3.0 (Ultralytics)",
}];

/// Space left free on the device after a download.
const SPARE_BYTES: u64 = 50 * 1024 * 1024;

pub fn find(id: &str) -> Option<&'static ModelInfo> {
    CATALOG.iter().find(|m| m.id == id)
}

pub fn installed_path(models_dir: &Path, model: &ModelInfo) -> PathBuf {
    models_dir.join(model.file_name)
}

/// What the user needs to know before downloading.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCheck {
    pub size_bytes: u64,
    /// Size announced by the server; `None` when it did not say.
    pub remote_size_bytes: Option<u64>,
    /// Free space where the model is saved; `None` when unknown.
    pub free_bytes: Option<u64>,
    pub enough_space: bool,
}

fn failure(reason: &'static str, detail: impl ToString) -> AppError {
    AppError::ModelDownload {
        reason,
        detail: detail.to_string(),
    }
}

fn client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .user_agent(concat!("V360Lab/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| failure("network", e))
}

fn free_space(dir: &Path) -> Option<u64> {
    // The folder may not exist yet: measure the nearest existing parent.
    dir.ancestors()
        .find(|p| p.exists())
        .and_then(|p| fs4::available_space(p).ok())
}

/// Checks that the file is reachable, has the expected size, and fits.
pub async fn check(model: &ModelInfo, models_dir: &Path) -> Result<DownloadCheck, AppError> {
    let response = client()?
        .head(model.url)
        .send()
        .await
        .map_err(|e| failure("network", e))?;
    if !response.status().is_success() {
        return Err(failure(
            "network",
            format!("HTTP {} for {}", response.status(), model.url),
        ));
    }
    let remote_size_bytes = response.content_length().filter(|n| *n > 0);
    if remote_size_bytes.is_some_and(|n| n != model.size_bytes) {
        return Err(failure(
            "sizeMismatch",
            format!(
                "expected {} bytes, the server announces {remote_size_bytes:?}",
                model.size_bytes
            ),
        ));
    }
    let free_bytes = free_space(models_dir);
    Ok(DownloadCheck {
        size_bytes: model.size_bytes,
        remote_size_bytes,
        free_bytes,
        enough_space: free_bytes.is_none_or(|free| free >= model.size_bytes + SPARE_BYTES),
    })
}

/// Downloads `model` into `models_dir`, checking size and SHA-256, and
/// returns its path. `progress(received, total)` follows the transfer.
pub async fn download(
    model: &ModelInfo,
    models_dir: &Path,
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> Result<PathBuf, AppError> {
    if free_space(models_dir).is_some_and(|free| free < model.size_bytes + SPARE_BYTES) {
        return Err(failure(
            "noSpace",
            format!("{} bytes needed", model.size_bytes),
        ));
    }
    tokio::fs::create_dir_all(models_dir)
        .await
        .map_err(AppError::fs(models_dir))?;
    let path = installed_path(models_dir, model);
    let partial = path.with_extension("onnx.part");

    let result = async {
        let response = client()?
            .get(model.url)
            .send()
            .await
            .map_err(|e| failure("network", e))?;
        if !response.status().is_success() {
            return Err(failure(
                "network",
                format!("HTTP {} for {}", response.status(), model.url),
            ));
        }
        let mut file = tokio::fs::File::create(&partial)
            .await
            .map_err(AppError::fs(&partial))?;
        let mut hasher = Sha256::new();
        let mut received = 0u64;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| failure("network", e))?;
            received += chunk.len() as u64;
            if received > model.size_bytes {
                return Err(failure(
                    "sizeMismatch",
                    format!("more than {} bytes", model.size_bytes),
                ));
            }
            hasher.update(&chunk);
            file.write_all(&chunk)
                .await
                .map_err(AppError::fs(&partial))?;
            progress(received, model.size_bytes);
        }
        file.flush().await.map_err(AppError::fs(&partial))?;
        if received != model.size_bytes {
            return Err(failure(
                "sizeMismatch",
                format!("received {received} of {} bytes", model.size_bytes),
            ));
        }
        let digest: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if digest != model.sha256 {
            return Err(failure(
                "checksum",
                format!("SHA-256 {digest}, expected {}", model.sha256),
            ));
        }
        tokio::fs::rename(&partial, &path)
            .await
            .map_err(AppError::fs(&path))?;
        Ok(path.clone())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&partial).await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn leak(text: String) -> &'static str {
        Box::leak(text.into_boxed_str())
    }

    fn test_model(url: String, body: &[u8]) -> ModelInfo {
        let sha256: String = Sha256::digest(body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        ModelInfo {
            id: "test",
            name: "Test",
            file_name: "test.onnx",
            url: leak(url),
            size_bytes: body.len() as u64,
            sha256: leak(sha256),
            classes: 1,
            license: "test",
        }
    }

    #[test]
    fn catalog_entries_are_complete() {
        for model in CATALOG {
            assert!(model.url.starts_with("https://"));
            assert_eq!(model.sha256.len(), 64);
            assert!(model.size_bytes > 0);
            assert!(model.file_name.ends_with(".onnx"));
        }
        assert!(find("yolo11n-coco").is_some());
    }

    #[tokio::test]
    async fn downloads_and_verifies_a_model() {
        let server = MockServer::start().await;
        let body = b"fake onnx model".to_vec();
        Mock::given(method("GET"))
            .and(path("/m.onnx"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .mount(&server)
            .await;
        Mock::given(method("HEAD"))
            .and(path("/m.onnx"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("content-length", body.len().to_string()),
            )
            .mount(&server)
            .await;
        let model = test_model(format!("{}/m.onnx", server.uri()), &body);
        let dir = tempfile::tempdir().unwrap();

        let check = check(&model, dir.path()).await.unwrap();
        assert_eq!(check.size_bytes, body.len() as u64);
        assert!(check.enough_space);

        let seen = std::sync::Mutex::new(Vec::new());
        let path = download(&model, &dir.path().join("models"), &|received, _| {
            seen.lock().unwrap().push(received)
        })
        .await
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), body);
        assert_eq!(seen.lock().unwrap().last(), Some(&(body.len() as u64)));
    }

    #[tokio::test]
    async fn rejects_a_file_with_the_wrong_checksum() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/m.onnx"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"tampered model!".to_vec()))
            .mount(&server)
            .await;
        // Same length as the tampered body, different content.
        let model = test_model(format!("{}/m.onnx", server.uri()), b"genuine model!!");
        let dir = tempfile::tempdir().unwrap();
        let err = download(&model, dir.path(), &|_, _| {}).await.unwrap_err();
        assert!(
            matches!(
                err,
                AppError::ModelDownload {
                    reason: "checksum",
                    ..
                }
            ),
            "{err:?}"
        );
        assert!(!dir.path().join("test.onnx").exists());
        assert!(!dir.path().join("test.onnx.part").exists());
    }
}
