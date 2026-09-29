//! Step-by-step progress reporting for slow operations (discovery,
//! connection, deletion). Steps carry a stable `code` plus `params`; the UI
//! translates them, so no user-facing text is produced here.

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Success,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub code: &'static str,
    pub level: Level,
    pub params: Map<String, Value>,
    pub progress: Option<Progress>,
}

impl Step {
    pub fn info(code: &'static str) -> Self {
        Self {
            code,
            level: Level::Info,
            params: Map::new(),
            progress: None,
        }
    }

    pub fn success(code: &'static str) -> Self {
        Self {
            level: Level::Success,
            ..Self::info(code)
        }
    }

    pub fn warning(code: &'static str) -> Self {
        Self {
            level: Level::Warning,
            ..Self::info(code)
        }
    }

    pub fn param(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.params.insert(key.to_string(), value.into());
        self
    }

    pub fn progress(mut self, done: u64, total: u64) -> Self {
        self.progress = Some(Progress { done, total });
        self
    }
}

/// Receives the steps of one operation.
pub type Reporter<'a> = &'a (dyn Fn(Step) + Send + Sync);

/// Event payload sent to the UI: a step tagged with the id the UI chose.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub activity_id: String,
    #[serde(flatten)]
    pub step: Step,
}

pub const ACTIVITY_EVENT: &str = "activity";
