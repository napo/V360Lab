//! Minimal FIT file inspection (header only, no record decoding).
//!
//! FIT header layout (little endian):
//!
//! | offset | size | field            |
//! |--------|------|------------------|
//! | 0      | 1    | header size (12 or 14) |
//! | 1      | 1    | protocol version |
//! | 2      | 2    | profile version  |
//! | 4      | 4    | data size        |
//! | 8      | 4    | ".FIT"           |
//! | 12     | 2    | header CRC (14-byte headers only) |

use std::path::Path;

use serde::Serialize;
use thiserror::Error;
use tokio::io::AsyncReadExt;

#[derive(Debug, Error)]
pub enum FitError {
    #[error("File is too short to be a FIT file")]
    TooShort,
    #[error("Missing \".FIT\" signature")]
    BadSignature,
    #[error("Unsupported FIT header size {0}")]
    BadHeaderSize(u8),
    #[error("Could not read FIT file: {0}")]
    Io(#[from] std::io::Error),
    #[error("FIT decoding is not implemented yet")]
    NotImplemented,
    #[error("The FIT file ends in the middle of a message")]
    Truncated,
    #[error("FIT data message uses undefined local type {0}")]
    UnknownMessage(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitHeader {
    pub header_size: u8,
    pub protocol_version: u8,
    pub profile_version: u16,
    pub data_size: u32,
}

pub fn read_header(bytes: &[u8]) -> Result<FitHeader, FitError> {
    if bytes.len() < 12 {
        return Err(FitError::TooShort);
    }
    let header_size = bytes[0];
    if header_size != 12 && header_size != 14 {
        return Err(FitError::BadHeaderSize(header_size));
    }
    if &bytes[8..12] != b".FIT" {
        return Err(FitError::BadSignature);
    }
    Ok(FitHeader {
        header_size,
        protocol_version: bytes[1],
        profile_version: u16::from_le_bytes([bytes[2], bytes[3]]),
        data_size: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
    })
}

/// Reads and validates the header of a FIT file on disk.
pub async fn inspect_file(path: &Path) -> Result<FitHeader, FitError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut header = [0u8; 14];
    let mut filled = 0;
    while filled < header.len() {
        let read = file.read(&mut header[filled..]).await?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    read_header(&header[..filled])
}

/// A valid, empty FIT file (14-byte header, no records, CRCs set).
/// Used by the mock camera.
pub fn empty_fit_file() -> Vec<u8> {
    let mut bytes = vec![14u8, 0x20];
    bytes.extend_from_slice(&2132u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(b".FIT");
    let header_crc = crc16(&bytes);
    bytes.extend_from_slice(&header_crc.to_le_bytes());
    let file_crc = crc16(&bytes);
    bytes.extend_from_slice(&file_crc.to_le_bytes());
    bytes
}

/// One GPS fix for [`gps_track_file`].
#[derive(Debug, Clone, Copy)]
pub struct GpsFix {
    pub unix_ms: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub speed_mps: f64,
}

/// A FIT file shaped like a VIRB's: a `camera_event` video start at
/// `video_start_ms` and one `gps_metadata` message per fix. Used by the mock
/// camera so that the telemetry view can be tried without hardware.
pub fn gps_track_file(video_start_ms: i64, fixes: &[GpsFix]) -> Vec<u8> {
    const FIT_EPOCH_MS: i64 = 631_065_600_000;
    let fit_time = |unix_ms: i64| {
        (
            ((unix_ms - FIT_EPOCH_MS) / 1000) as u32,
            (unix_ms.rem_euclid(1000)) as u16,
        )
    };
    let semicircles = |degrees: f64| ((degrees * 2_147_483_648.0 / 180.0) as i32).to_le_bytes();
    let mut data = Vec::new();
    // camera_event (161): timestamp, timestamp_ms, camera_event_type.
    data.extend_from_slice(&[0x40, 0, 0, 161, 0, 3, 253, 4, 0x86, 0, 2, 0x84, 1, 1, 0x00]);
    let (seconds, ms) = fit_time(video_start_ms);
    data.push(0);
    data.extend_from_slice(&seconds.to_le_bytes());
    data.extend_from_slice(&ms.to_le_bytes());
    data.push(0); // video start
                  // gps_metadata (160): timestamp, ms, lat, lon, altitude, speed, UTC timestamp.
    data.extend_from_slice(&[
        0x41, 0, 0, 160, 0, 7, 253, 4, 0x86, 0, 2, 0x84, 1, 4, 0x85, 2, 4, 0x85, 3, 4, 0x86, 4, 4,
        0x86, 6, 4, 0x86,
    ]);
    for fix in fixes {
        let (seconds, ms) = fit_time(fix.unix_ms);
        data.push(1);
        data.extend_from_slice(&seconds.to_le_bytes());
        data.extend_from_slice(&ms.to_le_bytes());
        data.extend_from_slice(&semicircles(fix.latitude));
        data.extend_from_slice(&semicircles(fix.longitude));
        data.extend_from_slice(&(((fix.altitude_m + 500.0) * 5.0).round() as u32).to_le_bytes());
        data.extend_from_slice(&((fix.speed_mps * 1000.0).round() as u32).to_le_bytes());
        data.extend_from_slice(&seconds.to_le_bytes());
    }
    let mut bytes = vec![14u8, 0x20];
    bytes.extend_from_slice(&2132u16.to_le_bytes());
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b".FIT");
    let header_crc = crc16(&bytes);
    bytes.extend_from_slice(&header_crc.to_le_bytes());
    bytes.extend_from_slice(&data);
    let file_crc = crc16(&bytes);
    bytes.extend_from_slice(&file_crc.to_le_bytes());
    bytes
}

/// CRC-16 as specified by the FIT protocol.
pub fn crc16(bytes: &[u8]) -> u16 {
    const TABLE: [u16; 16] = [
        0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800,
        0xB401, 0x5000, 0x9C01, 0x8801, 0x4400,
    ];
    bytes.iter().fold(0u16, |mut crc, &byte| {
        for nibble in [byte & 0x0F, byte >> 4] {
            let tmp = TABLE[(crc & 0x0F) as usize];
            crc = ((crc >> 4) & 0x0FFF) ^ tmp ^ TABLE[nibble as usize];
        }
        crc
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_fit_file_has_valid_header_and_crc() {
        let bytes = empty_fit_file();
        let header = read_header(&bytes).unwrap();
        assert_eq!(header.header_size, 14);
        assert_eq!(header.data_size, 0);
        // The CRC over a block that includes its own CRC is zero.
        assert_eq!(crc16(&bytes), 0);
    }

    #[test]
    fn rejects_non_fit_content() {
        assert!(matches!(read_header(b"short"), Err(FitError::TooShort)));
        assert!(matches!(
            read_header(b"<html>not a fit file</html>"),
            Err(FitError::BadHeaderSize(_))
        ));
        let mut bytes = empty_fit_file();
        bytes[9] = b'X';
        assert!(matches!(read_header(&bytes), Err(FitError::BadSignature)));
    }

    #[test]
    fn synthetic_track_decodes() {
        let fixes: Vec<GpsFix> = (0..10)
            .map(|i| GpsFix {
                unix_ms: 1_720_000_000_000 + i * 1000,
                latitude: 46.07 + i as f64 * 1e-4,
                longitude: 11.12,
                altitude_m: 194.0,
                speed_mps: 4.5,
            })
            .collect();
        let track =
            crate::telemetry::decode::decode(&gps_track_file(1_720_000_000_000, &fixes)).unwrap();
        assert_eq!(track.samples.len(), 10);
        assert_eq!(track.samples[3].timestamp_ms, 1_720_000_003_000);
        assert!((track.samples[3].latitude.unwrap() - 46.0703).abs() < 1e-6);
        assert_eq!(track.samples[3].altitude_m, Some(194.0));
        assert_eq!(track.camera_events[0].timestamp_ms, 1_720_000_000_000);
    }
}
