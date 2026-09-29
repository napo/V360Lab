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
}
