//! Detection over the frames extracted from a video (`frames/` folder with
//! its `frames.geojson` index, see `commands::write_frames_index`).
//!
//! 360° frames (2:1) are split into perspective views; every detection is
//! turned into a direction relative to the camera's front (yaw, clockwise)
//! and, when the frame has a direction of travel, into an absolute bearing
//! (azimuth). This assumes the camera's front lens faces the direction of
//! travel, as when it is mounted on a helmet, a bike or a car.

use std::path::{Path, PathBuf};

use image::RgbImage;
use serde::Serialize;
use serde_json::{json, Value};

use super::views::{
    angular_distance, direction_angles, horizontal_views, pixel_direction, render_view, View,
};
use super::{BoxDetection, DetectError, Detector};

/// A frame listed in `frames.geojson`.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameInfo {
    pub path: PathBuf,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: Option<f64>,
    pub heading_deg: Option<f64>,
    pub time: Option<String>,
    pub video_seconds: Option<f64>,
}

/// An object found in a frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectDetection {
    pub class_name: String,
    pub confidence: f32,
    /// Direction relative to the camera's front (360° frames only).
    pub yaw_deg: Option<f64>,
    pub pitch_deg: Option<f64>,
    /// Width of the box seen from the camera, in degrees.
    pub angular_width_deg: Option<f64>,
    /// Bearing from the frame's position (degrees clockwise from north).
    pub azimuth_deg: Option<f64>,
    pub frame: String,
    pub latitude: f64,
    pub longitude: f64,
    pub time: Option<String>,
    pub video_seconds: Option<f64>,
}

/// A detection on the sphere, before it is attached to a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct SphereDetection {
    pub class_name: String,
    pub confidence: f32,
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub angular_width_deg: f64,
    /// The box touches the edge of its view: the object is cut, and a
    /// neighbouring view probably shows it whole.
    pub truncated: bool,
}

/// A box closer than this to the edge of its view is cut.
const EDGE_MARGIN_PX: f32 = 2.0;

/// Reads `frames.geojson`; frame files are resolved next to it.
pub fn read_frames_index(path: &Path) -> Result<Vec<FrameInfo>, DetectError> {
    let invalid = |detail: String| DetectError::Image {
        path: path.display().to_string(),
        detail,
    };
    let text = std::fs::read_to_string(path).map_err(|e| invalid(e.to_string()))?;
    let geojson: Value = serde_json::from_str(&text).map_err(|e| invalid(e.to_string()))?;
    let directory = path.parent().unwrap_or(Path::new("."));
    let features = geojson["features"].as_array().cloned().unwrap_or_default();
    Ok(features
        .iter()
        .filter_map(|feature| {
            let properties = &feature["properties"];
            let coordinates = feature["geometry"]["coordinates"].as_array()?;
            Some(FrameInfo {
                path: directory.join(properties["file"].as_str()?),
                longitude: coordinates.first()?.as_f64()?,
                latitude: coordinates.get(1)?.as_f64()?,
                altitude_m: coordinates.get(2).and_then(Value::as_f64),
                heading_deg: properties["headingDeg"].as_f64(),
                time: properties["time"].as_str().map(str::to_string),
                video_seconds: properties["videoSeconds"].as_f64(),
            })
        })
        .collect())
}

fn to_sphere(view: &View, d: &BoxDetection) -> SphereDetection {
    let (yaw_deg, pitch_deg) =
        direction_angles(pixel_direction(view, f64::from(d.cx), f64::from(d.cy)));
    let left = direction_angles(pixel_direction(
        view,
        f64::from(d.cx - d.width / 2.0),
        f64::from(d.cy),
    ));
    let right = direction_angles(pixel_direction(
        view,
        f64::from(d.cx + d.width / 2.0),
        f64::from(d.cy),
    ));
    let size = view.size as f32;
    SphereDetection {
        class_name: d.class_name.clone(),
        confidence: d.confidence,
        yaw_deg,
        pitch_deg,
        angular_width_deg: angular_distance(left, right),
        truncated: d.cx - d.width / 2.0 <= EDGE_MARGIN_PX
            || d.cx + d.width / 2.0 >= size - EDGE_MARGIN_PX
            || d.cy - d.height / 2.0 <= EDGE_MARGIN_PX
            || d.cy + d.height / 2.0 >= size - EDGE_MARGIN_PX,
    }
}

/// Merges detections of the same object seen in two overlapping views:
/// same class and centres closer than half the object's width (at least 2°).
/// Whole objects win over cut ones, then the most confident.
pub fn merge_duplicates(mut detections: Vec<SphereDetection>) -> Vec<SphereDetection> {
    detections.sort_by(|a, b| {
        a.truncated
            .cmp(&b.truncated)
            .then(b.confidence.total_cmp(&a.confidence))
    });
    let mut kept: Vec<SphereDetection> = Vec::new();
    for candidate in detections {
        let duplicate = kept.iter().any(|k| {
            k.class_name == candidate.class_name
                && angular_distance(
                    (k.yaw_deg, k.pitch_deg),
                    (candidate.yaw_deg, candidate.pitch_deg),
                ) < (k.angular_width_deg.max(candidate.angular_width_deg) / 2.0).max(2.0)
        });
        if !duplicate {
            kept.push(candidate);
        }
    }
    kept
}

/// Detects objects all around a 360° (equirectangular) image.
pub fn detect_in_panorama(
    detector: &Detector,
    panorama: &RgbImage,
    min_confidence: f32,
) -> Result<Vec<SphereDetection>, DetectError> {
    let mut found = Vec::new();
    for view in horizontal_views(detector.input_size) {
        let image = render_view(panorama, &view);
        for detection in detector.detect(&image, min_confidence)? {
            found.push(to_sphere(&view, &detection));
        }
    }
    Ok(merge_duplicates(found))
}

/// Whether an image is a 360° panorama (2:1, as V360Lab extracts them).
pub fn is_panorama(image: &RgbImage) -> bool {
    let ratio = f64::from(image.width()) / f64::from(image.height().max(1));
    (1.9..=2.1).contains(&ratio)
}

/// Detects objects in one extracted frame.
pub fn detect_in_frame(
    detector: &Detector,
    frame: &FrameInfo,
    min_confidence: f32,
) -> Result<Vec<ObjectDetection>, DetectError> {
    let image = image::open(&frame.path)
        .map_err(|e| DetectError::Image {
            path: frame.path.display().to_string(),
            detail: e.to_string(),
        })?
        .to_rgb8();
    let name = frame
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let attach =
        |class_name: String, confidence: f32, sphere: Option<(f64, f64, f64)>| ObjectDetection {
            class_name,
            confidence,
            yaw_deg: sphere.map(|s| s.0),
            pitch_deg: sphere.map(|s| s.1),
            angular_width_deg: sphere.map(|s| s.2),
            azimuth_deg: sphere
                .and_then(|s| frame.heading_deg.map(|h| (h + s.0).rem_euclid(360.0))),
            frame: name.clone(),
            latitude: frame.latitude,
            longitude: frame.longitude,
            time: frame.time.clone(),
            video_seconds: frame.video_seconds,
        };
    if is_panorama(&image) {
        return Ok(detect_in_panorama(detector, &image, min_confidence)?
            .into_iter()
            .map(|d| {
                attach(
                    d.class_name,
                    d.confidence,
                    Some((d.yaw_deg, d.pitch_deg, d.angular_width_deg)),
                )
            })
            .collect());
    }
    // An ordinary frame: the whole image, without directions.
    let size = detector.input_size;
    let resized =
        image::imageops::resize(&image, size, size, image::imageops::FilterType::Triangle);
    Ok(detector
        .detect(&resized, min_confidence)?
        .into_iter()
        .map(|d| attach(d.class_name, d.confidence, None))
        .collect())
}

/// `detections.geojson`: one point per detection, at the position of the
/// frame it was found in.
pub fn detections_geojson(detections: &[ObjectDetection], video: &str, model: &str) -> Value {
    let features: Vec<Value> = detections
        .iter()
        .map(|d| {
            json!({
                "type": "Feature",
                "geometry": { "type": "Point", "coordinates": [d.longitude, d.latitude] },
                "properties": {
                    "class": d.class_name,
                    "confidence": (f64::from(d.confidence) * 1000.0).round() / 1000.0,
                    "azimuthDeg": d.azimuth_deg,
                    "yawDeg": d.yaw_deg,
                    "pitchDeg": d.pitch_deg,
                    "angularWidthDeg": d.angular_width_deg,
                    "frame": d.frame,
                    "time": d.time,
                    "videoSeconds": d.video_seconds,
                }
            })
        })
        .collect();
    json!({
        "type": "FeatureCollection",
        "features": features,
        "properties": { "video": video, "model": model, "creator": "V360Lab" },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sphere(class: &str, confidence: f32, yaw: f64, width: f64) -> SphereDetection {
        SphereDetection {
            class_name: class.into(),
            confidence,
            yaw_deg: yaw,
            pitch_deg: 0.0,
            angular_width_deg: width,
            truncated: false,
        }
    }

    #[test]
    fn merges_an_object_seen_in_two_views() {
        let merged = merge_duplicates(vec![
            sphere("car", 0.6, 44.0, 20.0),
            sphere("car", 0.8, 46.0, 18.0),
            sphere("person", 0.7, 45.0, 5.0),
            sphere("car", 0.7, 120.0, 20.0),
        ]);
        let merged: Vec<_> = merged
            .iter()
            .map(|d| (d.class_name.as_str(), d.confidence))
            .collect();
        assert_eq!(merged, [("car", 0.8), ("person", 0.7), ("car", 0.7)]);
    }

    #[test]
    fn prefers_the_whole_object_to_a_cut_one() {
        let cut = SphereDetection {
            truncated: true,
            ..sphere("bus", 0.93, 108.0, 44.0)
        };
        let merged = merge_duplicates(vec![cut, sphere("bus", 0.9, 90.0, 88.0)]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].yaw_deg, 90.0);
    }

    #[test]
    fn reads_the_frames_index() {
        let dir = tempfile::tempdir().unwrap();
        let index = dir.path().join("frames.geojson");
        std::fs::write(
            &index,
            r#"{"type":"FeatureCollection","features":[
                {"type":"Feature","geometry":{"type":"Point","coordinates":[11.12,46.07,194.0]},
                 "properties":{"file":"V1_00001.jpg","time":"2021-02-19T18:21:42.000Z","videoSeconds":1.5,"headingDeg":90.0}},
                {"type":"Feature","geometry":null,"properties":{}}
            ]}"#,
        )
        .unwrap();
        let frames = read_frames_index(&index).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].path, dir.path().join("V1_00001.jpg"));
        assert_eq!(frames[0].heading_deg, Some(90.0));
        assert_eq!(frames[0].altitude_m, Some(194.0));
    }

    #[test]
    fn writes_detections_with_bearings() {
        let detection = ObjectDetection {
            class_name: "car".into(),
            confidence: 0.8765,
            yaw_deg: Some(30.0),
            pitch_deg: Some(-5.0),
            angular_width_deg: Some(12.0),
            azimuth_deg: Some(120.0),
            frame: "V1_00001.jpg".into(),
            latitude: 46.07,
            longitude: 11.12,
            time: None,
            video_seconds: Some(1.5),
        };
        let geojson = detections_geojson(&[detection], "V1.MP4", "yolo11n");
        let feature = &geojson["features"][0];
        assert_eq!(feature["geometry"]["coordinates"], json!([11.12, 46.07]));
        assert_eq!(feature["properties"]["confidence"], 0.877);
        assert_eq!(feature["properties"]["azimuthDeg"], 120.0);
    }

    /// A panorama with Ultralytics' bus photo 90° to the right:
    /// `V360LAB_YOLO_MODEL=yolo11n.onnx V360LAB_YOLO_PANORAMA=pano_bus_right.jpg
    /// cargo test detect::frames::tests::detects_around_a_panorama -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a YOLO ONNX model and a panorama (see the doc comment)"]
    fn detects_around_a_panorama() {
        let model = std::env::var("V360LAB_YOLO_MODEL").expect("V360LAB_YOLO_MODEL");
        let panorama = std::env::var("V360LAB_YOLO_PANORAMA").expect("V360LAB_YOLO_PANORAMA");
        let detector = Detector::load(Path::new(&model)).unwrap();
        let panorama = image::open(panorama).unwrap().to_rgb8();
        assert!(is_panorama(&panorama));
        let started = std::time::Instant::now();
        let found = detect_in_panorama(&detector, &panorama, 0.4).unwrap();
        println!("{} ms", started.elapsed().as_millis());
        for d in &found {
            println!(
                "{} {:.2} yaw {:.1} pitch {:.1} width {:.1}",
                d.class_name, d.confidence, d.yaw_deg, d.pitch_deg, d.angular_width_deg
            );
        }
        let bus = found.iter().find(|d| d.class_name == "bus").expect("bus");
        assert!((bus.yaw_deg - 90.0).abs() < 10.0, "bus at {}", bus.yaw_deg);
        assert!(found.iter().filter(|d| d.class_name == "person").count() >= 3);
    }
}
