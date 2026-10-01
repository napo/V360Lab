//! FIT record decoding: just what is needed to rebuild a GPS track and
//! align it with a VIRB video.
//!
//! FIT files are a sequence of *definition* messages (field layout of a
//! local message type) and *data* messages (values laid out as defined).
//! Messages used here (FIT profile numbers):
//!
//! | global | name                    | fields used |
//! |--------|-------------------------|-------------|
//! | 20     | `record`                | timestamp, position, altitude, speed, heart rate |
//! | 160    | `gps_metadata` (VIRB)   | timestamp(+ms), position, altitude, speed, heading, UTC timestamp |
//! | 161    | `camera_event` (VIRB)   | timestamp(+ms), event type, camera file UUID |
//! | 162    | `timestamp_correlation` | UTC timestamp vs. system timestamp |
//!
//! VIRB timestamps (field 253) count from the camera's own clock; a
//! `timestamp_correlation` message maps them to UTC, and `gps_metadata`
//! also carries a UTC timestamp of its own.

use std::collections::HashMap;

use super::fit::{read_header, FitError};
use super::{CameraEvent, TelemetrySample, TelemetryTrack};

/// Seconds between the Unix epoch and the FIT epoch (1989-12-31 00:00 UTC).
pub const FIT_EPOCH_OFFSET: i64 = 631_065_600;
const SEMICIRCLES_TO_DEGREES: f64 = 180.0 / 2_147_483_648.0;

const MSG_RECORD: u16 = 20;
const MSG_GPS_METADATA: u16 = 160;
const MSG_CAMERA_EVENT: u16 = 161;
const MSG_TIMESTAMP_CORRELATION: u16 = 162;
const FIELD_TIMESTAMP: u8 = 253;

#[derive(Debug, Clone)]
struct FieldDef {
    number: u8,
    size: usize,
    base_type: u8,
}

#[derive(Debug, Clone)]
struct Definition {
    global: u16,
    big_endian: bool,
    fields: Vec<FieldDef>,
    /// Bytes of developer fields, skipped.
    developer_bytes: usize,
}

/// A decoded field value; arrays keep their first element.
#[derive(Debug, Clone, PartialEq)]
enum FieldValue {
    Int(i64),
    Float(f64),
    Text(String),
}

impl FieldValue {
    fn int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            Self::Float(v) => Some(*v as i64),
            Self::Text(_) => None,
        }
    }

    fn float(&self) -> Option<f64> {
        match self {
            Self::Int(v) => Some(*v as f64),
            Self::Float(v) => Some(*v),
            Self::Text(_) => None,
        }
    }
}

type Fields = HashMap<u8, FieldValue>;

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], FitError> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.data.len());
        let end = end.ok_or(FitError::Truncated)?;
        let bytes = &self.data[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }

    fn byte(&mut self) -> Result<u8, FitError> {
        Ok(self.take(1)?[0])
    }
}

fn read_uint(bytes: &[u8], big_endian: bool) -> u64 {
    let mut value = 0u64;
    let mut push = |b: u8| value = (value << 8) | u64::from(b);
    if big_endian {
        bytes.iter().copied().for_each(&mut push);
    } else {
        bytes.iter().rev().copied().for_each(&mut push);
    }
    value
}

/// Decodes one field; `None` for FIT "invalid" values and unknown types.
fn decode_field(bytes: &[u8], base_type: u8, big_endian: bool) -> Option<FieldValue> {
    let kind = base_type & 0x1F;
    if kind == 7 {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let text = String::from_utf8_lossy(&bytes[..end]).trim().to_string();
        return (!text.is_empty()).then_some(FieldValue::Text(text));
    }
    // (element size, signed, invalid raw value)
    let (size, signed, invalid): (usize, bool, u64) = match kind {
        0 | 2 | 13 => (1, false, 0xFF),
        1 => (1, true, 0x7F),
        10 => (1, false, 0),
        3 => (2, true, 0x7FFF),
        4 => (2, false, 0xFFFF),
        11 => (2, false, 0),
        5 => (4, true, 0x7FFF_FFFF),
        6 => (4, false, 0xFFFF_FFFF),
        12 => (4, false, 0),
        14 => (8, true, 0x7FFF_FFFF_FFFF_FFFF),
        15 => (8, false, u64::MAX),
        16 => (8, false, 0),
        8 => {
            let raw = read_uint(bytes.get(..4)?, big_endian) as u32;
            let value = f32::from_bits(raw);
            return (raw != 0xFFFF_FFFF && value.is_finite())
                .then_some(FieldValue::Float(value.into()));
        }
        9 => {
            let raw = read_uint(bytes.get(..8)?, big_endian);
            let value = f64::from_bits(raw);
            return (raw != u64::MAX && value.is_finite()).then_some(FieldValue::Float(value));
        }
        _ => return None,
    };
    let raw = read_uint(bytes.get(..size)?, big_endian);
    if raw == invalid {
        return None;
    }
    let value = if signed {
        let shift = 64 - 8 * size as u32;
        ((raw << shift) as i64) >> shift
    } else {
        raw as i64
    };
    Some(FieldValue::Int(value))
}

/// Decodes the GPS track and camera events of a FIT file.
pub fn decode(bytes: &[u8]) -> Result<TelemetryTrack, FitError> {
    let header = read_header(bytes)?;
    let start = usize::from(header.header_size);
    let end = start
        .checked_add(header.data_size as usize)
        .filter(|&end| end <= bytes.len())
        // Some writers leave data_size at 0 or short: read up to the CRC.
        .unwrap_or(bytes.len().saturating_sub(2))
        .max(start);
    let mut reader = Reader {
        data: &bytes[..end],
        pos: start,
    };

    let mut definitions: HashMap<u8, Definition> = HashMap::new();
    let mut last_timestamp: Option<u32> = None;
    let mut collector = Collector::default();

    while reader.pos < reader.data.len() {
        let header = reader.byte()?;
        if header & 0x80 != 0 {
            // Compressed timestamp header: a data message 5-bit time offset.
            let local = (header >> 5) & 0x03;
            let offset = u32::from(header & 0x1F);
            let timestamp = last_timestamp.map(|last| {
                let mut t = (last & !0x1F) | offset;
                if offset < (last & 0x1F) {
                    t += 0x20;
                }
                t
            });
            let definition = definitions
                .get(&local)
                .ok_or(FitError::UnknownMessage(local))?
                .clone();
            let mut fields = read_data(&mut reader, &definition)?;
            if let Some(t) = timestamp {
                fields
                    .entry(FIELD_TIMESTAMP)
                    .or_insert(FieldValue::Int(i64::from(t)));
                last_timestamp = Some(t);
            }
            collector.add(definition.global, &fields);
        } else if header & 0x40 != 0 {
            let local = header & 0x0F;
            let has_developer_fields = header & 0x20 != 0;
            reader.take(1)?; // reserved
            let big_endian = reader.byte()? == 1;
            let global = read_uint(reader.take(2)?, big_endian) as u16;
            let count = reader.byte()?;
            let mut fields = Vec::with_capacity(count.into());
            for _ in 0..count {
                let spec = reader.take(3)?;
                fields.push(FieldDef {
                    number: spec[0],
                    size: spec[1].into(),
                    base_type: spec[2],
                });
            }
            let mut developer_bytes = 0;
            if has_developer_fields {
                let count = reader.byte()?;
                for _ in 0..count {
                    developer_bytes += usize::from(reader.take(3)?[1]);
                }
            }
            definitions.insert(
                local,
                Definition {
                    global,
                    big_endian,
                    fields,
                    developer_bytes,
                },
            );
        } else {
            let local = header & 0x0F;
            let definition = definitions
                .get(&local)
                .ok_or(FitError::UnknownMessage(local))?
                .clone();
            let fields = read_data(&mut reader, &definition)?;
            if let Some(t) = fields.get(&FIELD_TIMESTAMP).and_then(FieldValue::int) {
                last_timestamp = Some(t as u32);
            }
            collector.add(definition.global, &fields);
        }
    }
    Ok(collector.finish())
}

fn read_data(reader: &mut Reader<'_>, definition: &Definition) -> Result<Fields, FitError> {
    let mut fields = Fields::new();
    for field in &definition.fields {
        let bytes = reader.take(field.size)?;
        if let Some(value) = decode_field(bytes, field.base_type, definition.big_endian) {
            fields.insert(field.number, value);
        }
    }
    reader.take(definition.developer_bytes)?;
    Ok(fields)
}

/// A sample before its time is converted to UTC.
struct RawSample {
    /// Camera clock (FIT seconds) plus milliseconds.
    system_ms: Option<i64>,
    /// UTC milliseconds since the Unix epoch, when the message has it.
    utc_ms: Option<i64>,
    sample: TelemetrySample,
}

#[derive(Default)]
struct Collector {
    samples: Vec<RawSample>,
    events: Vec<(i64, CameraEvent)>,
    /// UTC minus system time, in milliseconds.
    correlation_ms: Option<i64>,
}

fn get_f(fields: &Fields, n: u8) -> Option<f64> {
    fields.get(&n).and_then(FieldValue::float)
}

fn get_i(fields: &Fields, n: u8) -> Option<i64> {
    fields.get(&n).and_then(FieldValue::int)
}

fn fit_seconds_to_unix_ms(seconds: i64) -> i64 {
    (seconds + FIT_EPOCH_OFFSET) * 1000
}

impl Collector {
    fn add(&mut self, global: u16, fields: &Fields) {
        let timestamp = get_i(fields, FIELD_TIMESTAMP);
        match global {
            MSG_RECORD => {
                let altitude = get_f(fields, 78)
                    .or_else(|| get_f(fields, 2))
                    .map(|a| a / 5.0 - 500.0);
                let speed = get_f(fields, 73)
                    .or_else(|| get_f(fields, 6))
                    .map(|s| s / 1000.0);
                self.push(
                    timestamp.map(|t| t * 1000),
                    None,
                    get_i(fields, 0),
                    get_i(fields, 1),
                    altitude,
                    speed,
                    None,
                    get_i(fields, 3),
                );
            }
            MSG_GPS_METADATA => {
                let ms = get_i(fields, 0).unwrap_or(0);
                let utc = get_i(fields, 6).map(|t| fit_seconds_to_unix_ms(t) + ms);
                self.push(
                    timestamp.map(|t| t * 1000 + ms),
                    utc,
                    get_i(fields, 1),
                    get_i(fields, 2),
                    get_f(fields, 3).map(|a| a / 5.0 - 500.0),
                    get_f(fields, 4).map(|s| s / 1000.0),
                    get_f(fields, 5).map(|h| h / 100.0),
                    None,
                );
            }
            MSG_CAMERA_EVENT => {
                if let Some(t) = timestamp {
                    let ms = get_i(fields, 0).unwrap_or(0);
                    self.events.push((
                        t * 1000 + ms,
                        CameraEvent {
                            timestamp_ms: 0,
                            event_type: get_i(fields, 1).unwrap_or(-1),
                            file_uuid: match fields.get(&2) {
                                Some(FieldValue::Text(uuid)) => Some(uuid.clone()),
                                _ => None,
                            },
                        },
                    ));
                }
            }
            MSG_TIMESTAMP_CORRELATION => {
                // timestamp (UTC) + fraction vs. system_timestamp + fraction.
                if let (Some(utc), Some(system)) = (timestamp, get_i(fields, 1)) {
                    let fraction = |n| get_f(fields, n).map_or(0.0, |f| f / 32768.0);
                    let utc_ms = (utc * 1000) as f64 + fraction(0) * 1000.0;
                    let system_ms = (system * 1000) as f64 + fraction(2) * 1000.0;
                    self.correlation_ms
                        .get_or_insert((utc_ms - system_ms).round() as i64);
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        system_ms: Option<i64>,
        utc_ms: Option<i64>,
        lat: Option<i64>,
        lon: Option<i64>,
        altitude_m: Option<f64>,
        speed_mps: Option<f64>,
        heading_deg: Option<f64>,
        heart_rate: Option<i64>,
    ) {
        let degrees = |v: Option<i64>| v.map(|v| v as f64 * SEMICIRCLES_TO_DEGREES);
        let (latitude, longitude) = match (degrees(lat), degrees(lon)) {
            // (0, 0) means "no fix" in practice.
            (Some(la), Some(lo)) if la != 0.0 || lo != 0.0 => (Some(la), Some(lo)),
            _ => (None, None),
        };
        if latitude.is_none() && altitude_m.is_none() && speed_mps.is_none() {
            return;
        }
        self.samples.push(RawSample {
            system_ms,
            utc_ms,
            sample: TelemetrySample {
                timestamp_ms: 0,
                latitude,
                longitude,
                altitude_m,
                speed_mps,
                heading_deg,
                heart_rate: heart_rate.and_then(|h| u16::try_from(h).ok()),
            },
        });
    }

    fn finish(self) -> TelemetryTrack {
        // Camera clock -> UTC: from the correlation message, or from the
        // first GPS sample that carries both.
        let offset = self.correlation_ms.or_else(|| {
            self.samples
                .iter()
                .find_map(|s| match (s.utc_ms, s.system_ms) {
                    (Some(utc), Some(system)) => Some(utc - fit_seconds_to_unix_ms(0) - system),
                    _ => None,
                })
        });
        let to_unix = |system_ms: i64| match offset {
            Some(offset) => fit_seconds_to_unix_ms(0) + system_ms + offset,
            // `record` timestamps are already UTC in standard FIT files.
            None => fit_seconds_to_unix_ms(0) + system_ms,
        };
        let mut samples: Vec<TelemetrySample> = self
            .samples
            .into_iter()
            .filter_map(|raw| {
                let timestamp_ms = raw.utc_ms.or(raw.system_ms.map(to_unix))?;
                Some(TelemetrySample {
                    timestamp_ms,
                    ..raw.sample
                })
            })
            .collect();
        samples.sort_by_key(|s| s.timestamp_ms);
        samples.dedup_by_key(|s| s.timestamp_ms);
        let events = self
            .events
            .into_iter()
            .map(|(system_ms, event)| CameraEvent {
                timestamp_ms: to_unix(system_ms),
                ..event
            })
            .collect();
        TelemetryTrack {
            samples,
            camera_events: events,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::telemetry::fit::crc16;

    /// Builds FIT files for tests.
    pub struct FitBuilder {
        data: Vec<u8>,
    }

    impl FitBuilder {
        pub fn new() -> Self {
            Self { data: Vec::new() }
        }

        /// Definition of `local` as `global` with `(field, size, base type)`.
        pub fn define(mut self, local: u8, global: u16, fields: &[(u8, u8, u8)]) -> Self {
            self.data.extend_from_slice(&[0x40 | local, 0, 0]);
            self.data.extend_from_slice(&global.to_le_bytes());
            self.data.push(fields.len() as u8);
            for &(number, size, base) in fields {
                self.data.extend_from_slice(&[number, size, base]);
            }
            self
        }

        pub fn data(mut self, local: u8, values: &[&[u8]]) -> Self {
            self.data.push(local);
            values.iter().for_each(|v| self.data.extend_from_slice(v));
            self
        }

        pub fn compressed(mut self, local: u8, offset: u8, values: &[&[u8]]) -> Self {
            self.data.push(0x80 | (local << 5) | (offset & 0x1F));
            values.iter().for_each(|v| self.data.extend_from_slice(v));
            self
        }

        pub fn build(self) -> Vec<u8> {
            let mut bytes = vec![14u8, 0x20];
            bytes.extend_from_slice(&2132u16.to_le_bytes());
            bytes.extend_from_slice(&(self.data.len() as u32).to_le_bytes());
            bytes.extend_from_slice(b".FIT");
            let crc = crc16(&bytes);
            bytes.extend_from_slice(&crc.to_le_bytes());
            bytes.extend_from_slice(&self.data);
            let crc = crc16(&bytes);
            bytes.extend_from_slice(&crc.to_le_bytes());
            bytes
        }
    }

    fn semicircles(degrees: f64) -> [u8; 4] {
        ((degrees / SEMICIRCLES_TO_DEGREES) as i32).to_le_bytes()
    }

    #[test]
    fn decodes_virb_gps_metadata_with_utc_time() {
        // 2021-02-19 18:21:42 UTC as FIT seconds.
        let utc = (1_613_758_902 - FIT_EPOCH_OFFSET) as u32;
        let bytes = FitBuilder::new()
            .define(
                0,
                MSG_GPS_METADATA,
                &[
                    (253, 4, 0x86),
                    (0, 2, 0x84),
                    (1, 4, 0x85),
                    (2, 4, 0x85),
                    (3, 4, 0x86),
                    (4, 4, 0x86),
                    (6, 4, 0x86),
                ],
            )
            .data(
                0,
                &[
                    &1000u32.to_le_bytes(),
                    &250u16.to_le_bytes(),
                    &semicircles(46.07),
                    &semicircles(11.12),
                    // (200 m + 500) * 5
                    &3500u32.to_le_bytes(),
                    &5000u32.to_le_bytes(),
                    &utc.to_le_bytes(),
                ],
            )
            .build();
        let track = decode(&bytes).unwrap();
        let sample = &track.samples[0];
        assert_eq!(sample.timestamp_ms, 1_613_758_902_250);
        assert!((sample.latitude.unwrap() - 46.07).abs() < 1e-6);
        assert!((sample.longitude.unwrap() - 11.12).abs() < 1e-6);
        assert!((sample.altitude_m.unwrap() - 200.0).abs() < 1e-9);
        assert_eq!(sample.speed_mps, Some(5.0));
    }

    #[test]
    fn maps_camera_clock_to_utc_with_correlation() {
        let utc = (1_613_758_902 - FIT_EPOCH_OFFSET) as u32;
        let bytes = FitBuilder::new()
            .define(
                0,
                MSG_TIMESTAMP_CORRELATION,
                &[(253, 4, 0x86), (1, 4, 0x86)],
            )
            .data(0, &[&utc.to_le_bytes(), &1000u32.to_le_bytes()])
            .define(
                1,
                MSG_CAMERA_EVENT,
                &[(253, 4, 0x86), (0, 2, 0x84), (1, 1, 0x00), (2, 8, 0x07)],
            )
            .data(
                1,
                &[
                    &1010u32.to_le_bytes(),
                    &500u16.to_le_bytes(),
                    &[0],
                    b"VIRBabc\0",
                ],
            )
            .define(2, MSG_RECORD, &[(253, 4, 0x86), (0, 4, 0x85), (1, 4, 0x85)])
            .data(
                2,
                &[
                    &1020u32.to_le_bytes(),
                    &semicircles(46.0),
                    &semicircles(11.0),
                ],
            )
            // Compressed timestamp messages use a definition without field
            // 253: 1020 (low 5 bits = 28) with offset 29 -> 1021.
            .define(3, MSG_RECORD, &[(0, 4, 0x85), (1, 4, 0x85)])
            .compressed(3, 29, &[&semicircles(46.1), &semicircles(11.1)])
            .build();
        let track = decode(&bytes).unwrap();
        assert_eq!(track.camera_events[0].timestamp_ms, 1_613_758_912_500);
        assert_eq!(track.camera_events[0].file_uuid.as_deref(), Some("VIRBabc"));
        assert_eq!(track.samples[0].timestamp_ms, 1_613_758_922_000);
        assert_eq!(track.samples[1].timestamp_ms, 1_613_758_923_000);
    }

    #[test]
    fn skips_invalid_values_and_rejects_truncated_files() {
        let bytes = FitBuilder::new()
            .define(
                0,
                MSG_RECORD,
                &[(253, 4, 0x86), (0, 4, 0x85), (1, 4, 0x85), (2, 2, 0x84)],
            )
            .data(
                0,
                &[
                    &5u32.to_le_bytes(),
                    &0x7FFF_FFFFu32.to_le_bytes(),
                    &0x7FFF_FFFFu32.to_le_bytes(),
                    &3000u16.to_le_bytes(),
                ],
            )
            .build();
        let track = decode(&bytes).unwrap();
        assert_eq!(track.samples[0].latitude, None);
        assert_eq!(track.samples[0].altitude_m, Some(100.0));

        let mut truncated = bytes.clone();
        truncated.truncate(bytes.len() - 6);
        truncated[4..8].copy_from_slice(&100u32.to_le_bytes());
        assert!(decode(&truncated).is_err());
    }
}
