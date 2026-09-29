//! HTTP client for the Garmin VIRB 360.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::CONTENT_TYPE;
use reqwest::redirect::Policy;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::camera::address::{normalize_address, resolve_camera_url};
use crate::camera::{
    CameraClient, CameraError, CameraFeature, CameraKind, CameraStatus, CommandAck, DeviceInfo,
    FeatureList, FetchedResource, MediaItem, ProgressFn,
};

use super::commands::VirbCommand;
use super::errors::{self, snippet};
use super::models;

/// Maximum number of response characters written to the debug log.
const LOG_SNIPPET_CHARS: usize = 2000;

#[derive(Debug, Clone)]
pub struct VirbClientConfig {
    /// Time allowed to open a TCP connection to the camera.
    pub connect_timeout: Duration,
    /// Total time allowed for an API command or small resource fetch.
    pub command_timeout: Duration,
    /// `mediaList` is much slower: a full card returns hundreds of KB
    /// (about 4 s for ~1000 files on firmware 4.20).
    pub media_list_timeout: Duration,
    /// Maximum silence while streaming a download (no total limit, since
    /// 360 videos can be several gigabytes).
    pub transfer_read_timeout: Duration,
}

impl Default for VirbClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(3),
            command_timeout: Duration::from_secs(10),
            media_list_timeout: Duration::from_secs(60),
            transfer_read_timeout: Duration::from_secs(30),
        }
    }
}

pub struct GarminVirb360Client {
    base: Url,
    endpoint: Url,
    config: VirbClientConfig,
    http: reqwest::Client,
    transfer_http: reqwest::Client,
}

impl GarminVirb360Client {
    pub fn new(address: &str) -> Result<Self, CameraError> {
        Self::with_config(address, VirbClientConfig::default())
    }

    pub fn with_config(address: &str, config: VirbClientConfig) -> Result<Self, CameraError> {
        let base = normalize_address(address)?;
        let endpoint = base.join("virb").map_err(|e| CameraError::InvalidAddress {
            address: address.to_string(),
            reason: e.to_string(),
        })?;
        let build_error = |e: reqwest::Error| CameraError::Unreachable {
            address: base.to_string(),
            detail: format!("could not initialise HTTP client: {e}"),
        };
        // No proxies (the camera is on the local network) and no redirects
        // (requests must never be sent to another host).
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(config.connect_timeout)
            .timeout(config.command_timeout)
            .build()
            .map_err(build_error)?;
        let transfer_http = reqwest::Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(config.connect_timeout)
            .read_timeout(config.transfer_read_timeout)
            .build()
            .map_err(build_error)?;
        Ok(Self {
            base,
            endpoint,
            config,
            http,
            transfer_http,
        })
    }

    /// Sends a command and returns the validated JSON response.
    async fn execute(&self, command: &VirbCommand) -> Result<Value, CameraError> {
        let name = command.name();
        let timeout = match command {
            VirbCommand::MediaList => self.config.media_list_timeout,
            _ => self.config.command_timeout,
        };
        let started = Instant::now();
        log::debug!("VIRB -> {name} ({})", self.endpoint);

        let response = self
            .http
            .post(self.endpoint.clone())
            .json(&command.payload())
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| self.transport_error(&e, timeout))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| self.transport_error(&e, timeout))?;

        log::debug!(
            "VIRB <- {name}: HTTP {} in {} ms, {} bytes: {}",
            status.as_u16(),
            started.elapsed().as_millis(),
            body.len(),
            snippet(&body, LOG_SNIPPET_CHARS)
        );

        errors::check_http_status(name, status, &body)?;
        let value = errors::parse_json(name, &body)?;
        errors::check_result(name, &value)?;
        Ok(value)
    }

    async fn acknowledge(&self, command: VirbCommand) -> Result<CommandAck, CameraError> {
        let response = self.execute(&command).await?;
        Ok(models::command_ack(command.name(), response))
    }

    fn transport_error(&self, err: &reqwest::Error, timeout: Duration) -> CameraError {
        errors::from_reqwest(err, self.base.as_str(), timeout)
    }

    fn check_resource_status(url: &Url, status: reqwest::StatusCode) -> Result<(), CameraError> {
        if status.is_success() {
            Ok(())
        } else {
            Err(CameraError::Http {
                status: status.as_u16(),
                body: format!("GET {url}"),
            })
        }
    }
}

#[async_trait]
impl CameraClient for GarminVirb360Client {
    fn kind(&self) -> CameraKind {
        CameraKind::GarminVirb360
    }

    fn address(&self) -> String {
        self.base.to_string()
    }

    async fn device_info(&self) -> Result<DeviceInfo, CameraError> {
        models::parse_device_info(&self.execute(&VirbCommand::DeviceInfo).await?)
    }

    async fn status(&self) -> Result<CameraStatus, CameraError> {
        models::parse_status(&self.execute(&VirbCommand::Status).await?)
    }

    async fn features(&self) -> Result<FeatureList, CameraError> {
        let list = models::parse_features(&self.execute(&VirbCommand::Features).await?)?;
        log::debug!(
            "VIRB features: {}",
            list.features
                .iter()
                .map(|f: &CameraFeature| f.key.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(list)
    }

    async fn start_recording(&self) -> Result<CommandAck, CameraError> {
        self.acknowledge(VirbCommand::StartRecording).await
    }

    async fn stop_recording(&self) -> Result<CommandAck, CameraError> {
        self.acknowledge(VirbCommand::StopRecording).await
    }

    async fn snap_picture(&self) -> Result<CommandAck, CameraError> {
        self.acknowledge(VirbCommand::SnapPicture).await
    }

    async fn stop_still_recording(&self) -> Result<CommandAck, CameraError> {
        self.acknowledge(VirbCommand::StopStillRecording).await
    }

    async fn update_feature(&self, key: &str, value: &str) -> Result<FeatureList, CameraError> {
        let response = self
            .execute(&VirbCommand::UpdateFeature {
                feature: key.to_string(),
                value: value.to_string(),
            })
            .await?;
        // Firmware 4.20 returns the updated list; others may only ack.
        let list = if response.get("features").is_some() {
            models::parse_features(&response)?
        } else {
            self.features().await?
        };
        models::check_feature_value(&list, key, value)?;
        Ok(list)
    }

    async fn delete_files(&self, media_urls: &[String]) -> Result<CommandAck, CameraError> {
        self.acknowledge(VirbCommand::DeleteFile {
            files: media_urls.to_vec(),
        })
        .await
    }

    async fn media_list(&self) -> Result<Vec<MediaItem>, CameraError> {
        models::parse_media_list(&self.execute(&VirbCommand::MediaList).await?)
    }

    async fn fetch_resource(
        &self,
        url: &str,
        max_bytes: u64,
    ) -> Result<FetchedResource, CameraError> {
        let url = resolve_camera_url(&self.base, url)?;
        log::debug!("VIRB GET {url} (max {max_bytes} bytes)");
        let timeout = self.config.command_timeout;
        let response = self
            .http
            .get(url.clone())
            .send()
            .await
            .map_err(|e| self.transport_error(&e, timeout))?;
        Self::check_resource_status(&url, response.status())?;
        if let Some(size) = response.content_length().filter(|size| *size > max_bytes) {
            return Err(CameraError::TooLarge {
                size,
                limit: max_bytes,
            });
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| self.transport_error(&e, timeout))?;
            bytes.extend_from_slice(&chunk);
            if bytes.len() as u64 > max_bytes {
                return Err(CameraError::TooLarge {
                    size: bytes.len() as u64,
                    limit: max_bytes,
                });
            }
        }
        Ok(FetchedResource {
            bytes,
            content_type,
        })
    }

    async fn download_to(
        &self,
        url: &str,
        destination: &Path,
        progress: ProgressFn<'_>,
    ) -> Result<u64, CameraError> {
        let url = resolve_camera_url(&self.base, url)?;
        log::info!("Downloading {url} -> {}", destination.display());
        let timeout = self.config.transfer_read_timeout;
        let response = self
            .transfer_http
            .get(url.clone())
            .send()
            .await
            .map_err(|e| self.transport_error(&e, timeout))?;
        Self::check_resource_status(&url, response.status())?;
        let total = response.content_length();

        let partial = partial_path(destination);
        let result = stream_to_file(response, &partial, total, progress).await;
        let received = match result {
            Ok(received) => received,
            Err(StreamError::Io(source)) => {
                let _ = tokio::fs::remove_file(&partial).await;
                return Err(CameraError::Io {
                    path: partial,
                    source,
                });
            }
            Err(StreamError::Network(e)) => {
                let _ = tokio::fs::remove_file(&partial).await;
                return Err(CameraError::Transfer {
                    url: url.to_string(),
                    detail: e.to_string(),
                });
            }
        };
        if let Some(expected) = total.filter(|expected| *expected != received) {
            let _ = tokio::fs::remove_file(&partial).await;
            return Err(CameraError::Transfer {
                url: url.to_string(),
                detail: format!("expected {expected} bytes, received {received}"),
            });
        }
        tokio::fs::rename(&partial, destination)
            .await
            .map_err(|source| CameraError::Io {
                path: destination.to_path_buf(),
                source,
            })?;
        Ok(received)
    }
}

enum StreamError {
    Io(std::io::Error),
    Network(reqwest::Error),
}

async fn stream_to_file(
    response: reqwest::Response,
    path: &Path,
    total: Option<u64>,
    progress: ProgressFn<'_>,
) -> Result<u64, StreamError> {
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(StreamError::Io)?;
    let mut received = 0u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(StreamError::Network)?;
        file.write_all(&chunk).await.map_err(StreamError::Io)?;
        received += chunk.len() as u64;
        progress(received, total);
    }
    file.flush().await.map_err(StreamError::Io)?;
    file.sync_all().await.map_err(StreamError::Io)?;
    Ok(received)
}

/// `video.mp4` -> `video.mp4.part`
pub(crate) fn partial_path(destination: &Path) -> PathBuf {
    let mut name = destination.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    destination.with_file_name(name)
}
