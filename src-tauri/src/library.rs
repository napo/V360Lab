//! Operations on the camera's media library that span several requests.

use serde::Serialize;

use crate::camera::{CameraClient, CameraError, MediaItem};
use crate::error::{AppError, Resource};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFailure {
    pub item_id: String,
    pub name: String,
    pub error: AppError,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReport {
    /// Ids of items confirmed as deleted (or assumed, see `verified`).
    pub deleted: Vec<String>,
    pub failed: Vec<DeleteFailure>,
    /// False when the media list could not be re-read to confirm deletion.
    pub verified: bool,
}

/// Deletes media items on the camera, then re-reads the media list to
/// confirm: firmware 4.20 acknowledges `deleteFile` even for files that do
/// not exist, so the acknowledgement alone proves nothing.
pub async fn delete_media(client: &dyn CameraClient, items: &[MediaItem]) -> DeleteReport {
    let mut requested = Vec::new();
    let mut failed = Vec::new();

    for item in items {
        let result = match item.url.as_deref() {
            Some(url) => client.delete_file(url).await.map_err(AppError::from),
            None => Err(AppError::MissingResource {
                name: item.name.clone(),
                resource: Resource::DownloadUrl,
            }),
        };
        match result {
            Ok(_) => {
                log::info!("Deleted {} on the camera", item.name);
                requested.push(item);
            }
            Err(error) => failed.push(failure(item, error)),
        }
    }

    let (deleted, verified) = match client.media_list().await {
        Ok(remaining) => {
            let mut deleted = Vec::new();
            for item in requested {
                if remaining.iter().any(|m| m.id == item.id) {
                    let error = CameraError::CommandFailed {
                        command: "deleteFile".into(),
                        response: "the file is still listed after deletion".into(),
                    };
                    failed.push(failure(item, error.into()));
                } else {
                    deleted.push(item.id.clone());
                }
            }
            (deleted, true)
        }
        Err(e) => {
            log::warn!("Could not verify deletion: {e}");
            (requested.iter().map(|i| i.id.clone()).collect(), false)
        }
    };

    DeleteReport {
        deleted,
        failed,
        verified,
    }
}

fn failure(item: &MediaItem, error: AppError) -> DeleteFailure {
    DeleteFailure {
        item_id: item.id.clone(),
        name: item.name.clone(),
        error,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::virb::MockVirb360Client;

    #[tokio::test]
    async fn deletes_and_verifies() {
        let camera = MockVirb360Client::with_latency(Duration::ZERO);
        let media = camera.media_list().await.unwrap();

        // An item whose URL does not match what the camera lists: the camera
        // acknowledges, but the file stays, which must be reported.
        let mut stale = media[2].clone();
        stale.url = Some("mock://virb360/DCIM/100_VIRB/GONE.MP4".into());
        let mut no_url = media[3].clone();
        no_url.url = None;

        let report = delete_media(
            &camera,
            &[media[0].clone(), media[1].clone(), stale, no_url],
        )
        .await;

        assert!(report.verified);
        assert_eq!(
            report.deleted,
            vec![media[0].id.clone(), media[1].id.clone()]
        );
        assert_eq!(report.failed.len(), 2);
        let kinds: Vec<&str> = report.failed.iter().map(|f| f.error.kind()).collect();
        assert!(kinds.contains(&"commandFailed"));
        assert!(kinds.contains(&"missingResource"));
        assert_eq!(camera.media_list().await.unwrap().len(), 4);
    }
}
