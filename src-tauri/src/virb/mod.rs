//! Garmin VIRB 360 support over the camera's local HTTP API.
//!
//! The VIRB exposes a JSON-over-HTTP API: every command is a `POST` to
//! `http://<camera>/virb` with a body such as `{"command":"status"}`.
//! Responses are JSON objects carrying `"result": 1` on success.
//!
//! - [`commands`]: the commands we send and their payloads.
//! - [`models`]: tolerant parsing of responses into [`crate::camera`] models.
//! - [`errors`]: mapping of transport/protocol failures to typed errors.
//! - [`client`]: [`GarminVirb360Client`], the `CameraClient` implementation.
//! - [`mock`]: [`MockVirb360Client`], a simulated camera for development.

pub mod client;
pub mod commands;
pub mod errors;
pub mod mock;
pub mod models;

pub use client::{GarminVirb360Client, VirbClientConfig};
pub use mock::MockVirb360Client;
