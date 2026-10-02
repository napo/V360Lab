//! Checking for a newer V360Lab release on GitHub.
//!
//! The check reads the latest release of the repository (the GitHub API
//! returns the release marked "latest": app releases, not the models
//! release). Installing is done by the UI: on desktop with the signed
//! Tauri updater, on Android by downloading the APK, which Android then
//! asks the user to install.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/napo/V360Lab/releases/latest";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    /// The latest release is newer than this app.
    pub available: bool,
    /// Release notes (Markdown), as published.
    pub notes: String,
    pub page_url: String,
    /// Direct link to the Android APK of the latest release.
    pub apk_url: Option<String>,
    /// `android` or `desktop`: how the UI installs the update.
    pub platform: &'static str,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// `v1.2.3` / `1.2.3` -> (1, 2, 3); pre-release suffixes are ignored.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let core = text
        .trim()
        .trim_start_matches('v')
        .split(['-', '+'])
        .next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    Some((
        parts.next()??,
        parts.next().flatten().unwrap_or(0),
        parts.next().flatten().unwrap_or(0),
    ))
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    matches!((parse_version(latest), parse_version(current)), (Some(l), Some(c)) if l > c)
}

fn failure(detail: impl ToString) -> AppError {
    AppError::UpdateCheck {
        detail: detail.to_string(),
    }
}

/// Asks GitHub for the latest release (`api_url` is a parameter for tests).
pub async fn check(api_url: &str, current_version: &str) -> Result<UpdateInfo, AppError> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("V360Lab/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(failure)?;
    let response = client
        .get(api_url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(failure)?;
    if !response.status().is_success() {
        return Err(failure(format!(
            "GitHub answered HTTP {}",
            response.status()
        )));
    }
    let release: Release = response.json().await.map_err(failure)?;
    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    Ok(UpdateInfo {
        available: is_newer(&latest_version, current_version),
        current_version: current_version.to_string(),
        latest_version,
        notes: release.body.unwrap_or_default(),
        page_url: release.html_url,
        apk_url: release
            .assets
            .into_iter()
            .find(|a| a.name.ends_with(".apk"))
            .map(|a| a.browser_download_url),
        platform: if cfg!(target_os = "android") {
            "android"
        } else {
            "desktop"
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.5.2", "0.5.1"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.5.1", "0.5.1"));
        assert!(!is_newer("0.5.0", "0.5.1"));
        assert!(!is_newer("0.6.0-beta", "0.6.0"));
        assert!(!is_newer("models-v1", "0.5.1"));
    }

    #[tokio::test]
    async fn reads_the_latest_release() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tag_name": "v0.6.0",
                "body": "## What's new\n- Things",
                "html_url": "https://github.com/napo/V360Lab/releases/tag/v0.6.0",
                "assets": [
                    { "name": "V360Lab_0.6.0_x64-setup.exe", "browser_download_url": "https://x/setup.exe" },
                    { "name": "V360Lab_0.6.0_android_universal.apk", "browser_download_url": "https://x/app.apk" }
                ]
            })))
            .mount(&server)
            .await;
        let info = check(&format!("{}/latest", server.uri()), "0.5.1")
            .await
            .unwrap();
        assert!(info.available);
        assert_eq!(info.latest_version, "0.6.0");
        assert_eq!(info.apk_url.as_deref(), Some("https://x/app.apk"));
        assert!(info.notes.contains("Things"));

        let same = check(&format!("{}/latest", server.uri()), "0.6.0")
            .await
            .unwrap();
        assert!(!same.available);
    }

    #[tokio::test]
    async fn reports_an_unreachable_server() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        let err = check(&format!("{}/latest", server.uri()), "0.5.1")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::UpdateCheck { .. }));
    }
}
