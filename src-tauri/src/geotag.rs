//! Geotagging of extracted video frames: EXIF GPS and, for 360° frames,
//! the XMP "GPano" (Photo Sphere) tags that Mapillary, Panoramax, Google
//! and photogrammetry tools use to recognise equirectangular panoramas.
//!
//! JPEG layout written: `SOI, APP1 Exif, APP1 XMP, <original segments>`
//! (an existing JFIF APP0 or Exif APP1 right after SOI is dropped).

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GeotagError {
    #[error("not a JPEG image")]
    NotJpeg,
    #[error("the JPEG image has no frame header")]
    NoFrameHeader,
}

/// Where and when a frame was taken.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameLocation {
    /// UTC milliseconds since the Unix epoch.
    pub unix_ms: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: Option<f64>,
    /// Direction of travel (degrees clockwise from true north), used as
    /// the image direction and the panorama heading.
    pub heading_deg: Option<f64>,
    pub speed_mps: Option<f64>,
}

// TIFF field types.
const BYTE: u16 = 1;
const ASCII: u16 = 2;
const LONG: u16 = 4;
const RATIONAL: u16 = 5;

struct Entry {
    tag: u16,
    kind: u16,
    count: u32,
    data: Vec<u8>,
}

fn ascii(tag: u16, text: &str) -> Entry {
    let mut data = text.as_bytes().to_vec();
    data.push(0);
    Entry {
        tag,
        kind: ASCII,
        count: data.len() as u32,
        data,
    }
}

fn rationals(tag: u16, values: &[(u32, u32)]) -> Entry {
    let data = values
        .iter()
        .flat_map(|(n, d)| n.to_le_bytes().into_iter().chain(d.to_le_bytes()))
        .collect();
    Entry {
        tag,
        kind: RATIONAL,
        count: values.len() as u32,
        data,
    }
}

fn bytes(tag: u16, values: &[u8]) -> Entry {
    Entry {
        tag,
        kind: BYTE,
        count: values.len() as u32,
        data: values.to_vec(),
    }
}

fn long(tag: u16, value: u32) -> Entry {
    Entry {
        tag,
        kind: LONG,
        count: 1,
        data: value.to_le_bytes().to_vec(),
    }
}

/// Serialises an IFD placed at `offset` in the TIFF stream; returns its
/// bytes (entries, next-IFD 0, then the values that do not fit in 4 bytes).
fn ifd(mut entries: Vec<Entry>, offset: u32) -> Vec<u8> {
    entries.sort_by_key(|e| e.tag);
    let table = 2 + entries.len() as u32 * 12 + 4;
    let mut out = (entries.len() as u16).to_le_bytes().to_vec();
    let mut extra = Vec::new();
    for entry in &entries {
        out.extend_from_slice(&entry.tag.to_le_bytes());
        out.extend_from_slice(&entry.kind.to_le_bytes());
        out.extend_from_slice(&entry.count.to_le_bytes());
        if entry.data.len() <= 4 {
            let mut inline = entry.data.clone();
            inline.resize(4, 0);
            out.extend_from_slice(&inline);
        } else {
            out.extend_from_slice(&(offset + table + extra.len() as u32).to_le_bytes());
            extra.extend_from_slice(&entry.data);
            if extra.len() % 2 == 1 {
                extra.push(0); // values start on word boundaries
            }
        }
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&extra);
    out
}

/// Degrees -> degrees/minutes/seconds rationals (seconds to 1/10000).
fn dms(value: f64) -> [(u32, u32); 3] {
    let value = value.abs();
    let degrees = value.trunc();
    let minutes = ((value - degrees) * 60.0).trunc();
    let seconds = (value - degrees - minutes / 60.0) * 3600.0;
    [
        (degrees as u32, 1),
        (minutes as u32, 1),
        ((seconds * 10_000.0).round() as u32, 10_000),
    ]
}

fn exif_segment(location: &FrameLocation, camera_model: &str) -> Vec<u8> {
    let time = DateTime::<Utc>::from_timestamp_millis(location.unix_ms).unwrap_or_default();
    let date_time = time.format("%Y:%m:%d %H:%M:%S").to_string();

    let mut gps = vec![
        bytes(0x0000, &[2, 3, 0, 0]),
        ascii(0x0001, if location.latitude >= 0.0 { "N" } else { "S" }),
        rationals(0x0002, &dms(location.latitude)),
        ascii(0x0003, if location.longitude >= 0.0 { "E" } else { "W" }),
        rationals(0x0004, &dms(location.longitude)),
        rationals(
            0x0007,
            &[
                (time.hour(), 1),
                (time.minute(), 1),
                (time.second() * 1000 + time.timestamp_subsec_millis(), 1000),
            ],
        ),
        ascii(
            0x001D,
            &format!("{:04}:{:02}:{:02}", time.year(), time.month(), time.day()),
        ),
    ];
    if let Some(altitude) = location.altitude_m {
        gps.push(bytes(0x0005, &[u8::from(altitude < 0.0)]));
        gps.push(rationals(
            0x0006,
            &[((altitude.abs() * 100.0).round() as u32, 100)],
        ));
    }
    if let Some(speed) = location.speed_mps {
        gps.push(ascii(0x000C, "K"));
        gps.push(rationals(
            0x000D,
            &[((speed * 3.6 * 100.0).round() as u32, 100)],
        ));
    }
    if let Some(heading) = location.heading_deg {
        let heading = heading.rem_euclid(360.0);
        gps.push(ascii(0x0010, "T"));
        gps.push(rationals(
            0x0011,
            &[((heading * 100.0).round() as u32, 100)],
        ));
        gps.push(ascii(0x000E, "T"));
        gps.push(rationals(
            0x000F,
            &[((heading * 100.0).round() as u32, 100)],
        ));
    }

    let exif = vec![
        ascii(0x9003, &date_time),
        ascii(0x9004, &date_time),
        ascii(0x9011, "+00:00"),
        ascii(0x9291, &format!("{:03}", time.timestamp_subsec_millis())),
    ];

    // TIFF header (8 bytes), IFD0, Exif IFD, GPS IFD.
    let ifd0_entries = |exif_at: u32, gps_at: u32| {
        vec![
            ascii(0x010F, "Garmin"),
            ascii(0x0110, camera_model),
            ascii(0x0131, "V360Lab"),
            long(0x8769, exif_at),
            long(0x8825, gps_at),
        ]
    };
    // Sizes do not depend on the pointer values: lay out once to measure.
    let ifd0_len = ifd(ifd0_entries(0, 0), 8).len() as u32;
    let exif_at = 8 + ifd0_len;
    let exif_ifd = ifd(exif, exif_at);
    let gps_at = exif_at + exif_ifd.len() as u32;
    let gps_ifd = ifd(gps, gps_at);

    let mut tiff = b"II*\0".to_vec();
    tiff.extend_from_slice(&8u32.to_le_bytes());
    tiff.extend_from_slice(&ifd(ifd0_entries(exif_at, gps_at), 8));
    tiff.extend_from_slice(&exif_ifd);
    tiff.extend_from_slice(&gps_ifd);

    let mut payload = b"Exif\0\0".to_vec();
    payload.extend_from_slice(&tiff);
    segment(0xE1, &payload)
}

fn xmp_segment(location: &FrameLocation, width: u32, height: u32) -> Vec<u8> {
    let heading = location
        .heading_deg
        .map(|h| {
            format!(
                "\n   GPano:PoseHeadingDegrees=\"{:.1}\"",
                h.rem_euclid(360.0)
            )
        })
        .unwrap_or_default();
    let xmp = format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
 <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
  <rdf:Description rdf:about=\"\" xmlns:GPano=\"http://ns.google.com/photos/1.0/panorama/\"\n\
   GPano:UsePanoramaViewer=\"True\"\n\
   GPano:ProjectionType=\"equirectangular\"\n\
   GPano:FullPanoWidthPixels=\"{width}\"\n\
   GPano:FullPanoHeightPixels=\"{height}\"\n\
   GPano:CroppedAreaImageWidthPixels=\"{width}\"\n\
   GPano:CroppedAreaImageHeightPixels=\"{height}\"\n\
   GPano:CroppedAreaLeftPixels=\"0\"\n\
   GPano:CroppedAreaTopPixels=\"0\"{heading}/>\n\
 </rdf:RDF>\n\
</x:xmpmeta>\n\
<?xpacket end=\"w\"?>"
    );
    let mut payload = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
    payload.extend_from_slice(xmp.as_bytes());
    segment(0xE1, &payload)
}

fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0xFF, marker];
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Width and height from the JPEG frame header (SOF0–SOF15, not DHT/JPG/DAC).
pub fn jpeg_size(jpeg: &[u8]) -> Result<(u32, u32), GeotagError> {
    if !jpeg.starts_with(&[0xFF, 0xD8]) {
        return Err(GeotagError::NotJpeg);
    }
    let mut pos = 2;
    while pos + 4 <= jpeg.len() {
        if jpeg[pos] != 0xFF {
            return Err(GeotagError::NoFrameHeader);
        }
        let marker = jpeg[pos + 1];
        let length = usize::from(u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]));
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let header = jpeg
                .get(pos + 5..pos + 9)
                .ok_or(GeotagError::NoFrameHeader)?;
            let height = u16::from_be_bytes([header[0], header[1]]);
            let width = u16::from_be_bytes([header[2], header[3]]);
            return Ok((width.into(), height.into()));
        }
        if marker == 0xDA {
            break; // image data: no frame header before it
        }
        pos += 2 + length;
    }
    Err(GeotagError::NoFrameHeader)
}

/// Adds EXIF GPS (and GPano XMP when `spherical`) to a JPEG image.
pub fn tag_jpeg(
    jpeg: &[u8],
    location: &FrameLocation,
    camera_model: &str,
    spherical: bool,
) -> Result<Vec<u8>, GeotagError> {
    let (width, height) = jpeg_size(jpeg)?;
    let mut rest = &jpeg[2..];
    // Drop a leading JFIF (APP0) or Exif (APP1) segment: ours replaces it.
    while rest.len() >= 4 && rest[0] == 0xFF && matches!(rest[1], 0xE0 | 0xE1) {
        let length = usize::from(u16::from_be_bytes([rest[2], rest[3]]));
        let is_xmp =
            rest[1] == 0xE1 && rest.get(4..33) == Some(&b"http://ns.adobe.com/xap/1.0/\0"[..]);
        if is_xmp || rest.len() < 2 + length {
            break;
        }
        rest = &rest[2 + length..];
    }
    let mut out = vec![0xFF, 0xD8];
    out.extend_from_slice(&exif_segment(location, camera_model));
    if spherical {
        out.extend_from_slice(&xmp_segment(location, width, height));
    }
    out.extend_from_slice(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smallest JPEG-like stream the tagger needs: SOI, JFIF, SOF0, SOS, EOI.
    fn fake_jpeg(width: u16, height: u16) -> Vec<u8> {
        let mut jpeg = vec![0xFF, 0xD8];
        jpeg.extend_from_slice(&segment(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"));
        let mut sof = vec![8];
        sof.extend_from_slice(&height.to_be_bytes());
        sof.extend_from_slice(&width.to_be_bytes());
        sof.extend_from_slice(&[1, 1, 0x11, 0]);
        jpeg.extend_from_slice(&segment(0xC0, &sof));
        jpeg.extend_from_slice(&segment(0xDA, &[1, 1, 0, 0, 63, 0]));
        jpeg.extend_from_slice(&[0x12, 0x34, 0xFF, 0xD9]);
        jpeg
    }

    fn location() -> FrameLocation {
        FrameLocation {
            unix_ms: 1_613_758_902_250,
            latitude: 46.0679,
            longitude: -11.1211,
            altitude_m: Some(194.25),
            heading_deg: Some(270.0),
            speed_mps: Some(5.0),
        }
    }

    #[test]
    fn reads_the_frame_size() {
        assert_eq!(jpeg_size(&fake_jpeg(5760, 2880)), Ok((5760, 2880)));
        assert_eq!(jpeg_size(b"GIF89a"), Err(GeotagError::NotJpeg));
    }

    #[test]
    fn converts_to_degrees_minutes_seconds() {
        let [d, m, s] = dms(46.0679);
        assert_eq!((d, m), ((46, 1), (4, 1)));
        // 0.0679° = 4' 4.44"
        assert_eq!(s, (44_400, 10_000));
    }

    #[test]
    fn writes_exif_and_gpano_before_the_image() {
        let tagged = tag_jpeg(&fake_jpeg(2048, 1024), &location(), "VIRB 360", true).unwrap();
        assert_eq!(&tagged[..2], &[0xFF, 0xD8]);
        assert_eq!(&tagged[2..4], &[0xFF, 0xE1]);
        assert_eq!(&tagged[6..12], b"Exif\0\0");
        let text = String::from_utf8_lossy(&tagged);
        assert!(text.contains("GPano:ProjectionType=\"equirectangular\""));
        assert!(text.contains("GPano:FullPanoWidthPixels=\"2048\""));
        assert!(text.contains("GPano:PoseHeadingDegrees=\"270.0\""));
        assert!(text.contains("2021:02:19 18:21:42"));
        assert!(!text.contains("JFIF"), "the JFIF segment is replaced");
        assert!(tagged.ends_with(&[0x12, 0x34, 0xFF, 0xD9]));
        assert_eq!(jpeg_size(&tagged), Ok((2048, 1024)));
    }

    #[test]
    fn flat_frames_get_no_panorama_tags() {
        let tagged = tag_jpeg(&fake_jpeg(1920, 1080), &location(), "VIRB 360", false).unwrap();
        assert!(!String::from_utf8_lossy(&tagged).contains("GPano"));
    }

    /// Writes a tagged frame for manual inspection with exiftool:
    /// `cargo test geotag::tests::write_sample -- --ignored`, then
    /// `exiftool -G -a /tmp/v360lab-geotag-sample.jpg`.
    #[test]
    #[ignore = "writes a file for manual inspection"]
    fn write_sample() {
        let tagged = tag_jpeg(&fake_jpeg(2048, 1024), &location(), "VIRB 360", true).unwrap();
        std::fs::write(
            std::env::temp_dir().join("v360lab-geotag-sample.jpg"),
            tagged,
        )
        .unwrap();
    }
}
