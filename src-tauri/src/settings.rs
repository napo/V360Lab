//! User settings persisted as JSON in the application config directory.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Last camera address that connected successfully.
    pub last_camera_address: Option<String>,
    /// Root directory for downloads; `None` means the platform default.
    pub download_directory: Option<String>,
    /// Use the simulated camera instead of the network.
    pub mock_mode: bool,
    /// Show technical error details and raw JSON; verbose logging.
    pub debug_mode: bool,
    /// Interval of the background status refresh while connected.
    pub status_poll_interval_secs: u32,
    /// UI language (`en`, `it`); `None` follows the system language.
    pub language: Option<String>,
}

/// Languages the UI is translated into.
pub const SUPPORTED_LANGUAGES: [&str; 2] = ["en", "it"];

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_camera_address: None,
            download_directory: None,
            mock_mode: false,
            debug_mode: cfg!(debug_assertions),
            status_poll_interval_secs: 5,
            language: None,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), AppError> {
        if !(1..=300).contains(&self.status_poll_interval_secs) {
            return Err(AppError::Settings {
                reason: "pollInterval",
                message: "the status refresh interval must be between 1 and 300 seconds".into(),
            });
        }
        if let Some(dir) = &self.download_directory {
            if !Path::new(dir).is_absolute() {
                return Err(AppError::Settings {
                    reason: "downloadDirectoryRelative",
                    message: "the download directory must be an absolute path".into(),
                });
            }
        }
        if let Some(language) = &self.language {
            if !SUPPORTED_LANGUAGES.contains(&language.as_str()) {
                return Err(AppError::Settings {
                    reason: "unsupportedLanguage",
                    message: format!("unsupported language \"{language}\""),
                });
            }
        }
        Ok(())
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    /// Loads settings from `path`, falling back to defaults when the file is
    /// missing or unreadable (the problem is logged, never fatal).
    pub fn load(path: PathBuf) -> Self {
        let settings = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                log::warn!("Ignoring invalid settings file {}: {e}", path.display());
                Settings::default()
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => {
                log::warn!("Could not read settings file {}: {e}", path.display());
                Settings::default()
            }
        };
        Self {
            path,
            current: Mutex::new(settings),
        }
    }

    pub fn get(&self) -> Settings {
        self.lock().clone()
    }

    /// Applies `change`, validates and saves. Nothing changes on failure.
    pub fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<Settings, AppError> {
        let mut current = self.lock();
        let mut next = current.clone();
        change(&mut next);
        next.validate()?;
        self.save(&next)?;
        *current = next.clone();
        Ok(next)
    }

    fn save(&self, settings: &Settings) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(AppError::fs(parent))?;
        }
        let json = serde_json::to_string_pretty(settings).map_err(|e| AppError::Settings {
            reason: "serialization",
            message: e.to_string(),
        })?;
        let temp = self.path.with_extension("json.tmp");
        std::fs::write(&temp, json).map_err(AppError::fs(&temp))?;
        std::fs::rename(&temp, &self.path).map_err(AppError::fs(&self.path))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Settings> {
        self.current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/settings.json");
        let store = SettingsStore::load(path.clone());
        assert_eq!(store.get(), Settings::default());

        store
            .update(|s| s.last_camera_address = Some("192.168.1.50".into()))
            .unwrap();
        let reloaded = SettingsStore::load(path);
        assert_eq!(
            reloaded.get().last_camera_address.as_deref(),
            Some("192.168.1.50")
        );
    }

    #[test]
    fn rejects_invalid_values_without_saving() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::load(dir.path().join("settings.json"));
        assert!(store.update(|s| s.status_poll_interval_secs = 0).is_err());
        assert!(store
            .update(|s| s.download_directory = Some("relative/dir".into()))
            .is_err());
        assert!(store.update(|s| s.language = Some("xx".into())).is_err());
        assert!(store.update(|s| s.language = Some("it".into())).is_ok());
        store.update(|s| s.language = None).unwrap();
        assert_eq!(store.get(), Settings::default());
    }

    #[test]
    fn tolerates_corrupt_or_partial_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(SettingsStore::load(path.clone()).get(), Settings::default());

        std::fs::write(&path, r#"{"mockMode": true}"#).unwrap();
        let settings = SettingsStore::load(path).get();
        assert!(settings.mock_mode);
        assert_eq!(settings.status_poll_interval_secs, 5);
    }
}
