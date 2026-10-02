//! Object detection on video frames with a YOLO model in ONNX format
//! (Ultralytics YOLOv8/YOLO11 "detect" export: input `[1, 3, N, N]`, RGB
//! 0–1; output `[1, 4 + classes, anchors]` with box centre, size and class
//! scores). Inference runs with tract, a pure-Rust ONNX engine, so it
//! works the same on desktop and Android without native libraries.
//!
//! - [`views`]: perspective views of 360° frames and directions back.
//! - [`frames`]: detection over the extracted frames of a video.

pub mod frames;
pub mod views;

use std::sync::atomic::{AtomicBool, Ordering};

/// Set to stop the detection that is running.
static CANCEL: AtomicBool = AtomicBool::new(false);

/// Asks the running detection to stop after the current frame.
pub fn cancel() {
    CANCEL.store(true, Ordering::Relaxed);
}

fn reset_cancel() {
    CANCEL.store(false, Ordering::Relaxed);
}

fn cancelled() -> bool {
    CANCEL.load(Ordering::Relaxed)
}

use std::path::Path;

use image::RgbImage;
use serde::Serialize;
use thiserror::Error;
use tract_onnx::prelude::*;

#[derive(Debug, Error)]
pub enum DetectError {
    #[error("Could not load the detection model: {0}")]
    Model(String),
    #[error("Detection failed: {0}")]
    Inference(String),
    #[error("Could not read the image {path}: {detail}")]
    Image { path: String, detail: String },
    #[error("No frames to analyse: extract frames from the video first")]
    NoFrames,
}

/// Progress of a detection run, sent to the UI after each frame.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionProgress {
    pub done: usize,
    pub total: usize,
    pub detections: usize,
}

/// Result of a detection run over a video's frames.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionReport {
    pub frames: usize,
    pub detections: usize,
    /// (class, count), most frequent first.
    pub by_class: Vec<(String, usize)>,
    /// Path of `detections.geojson`.
    pub path: String,
    pub cancelled: bool,
}

/// Runs the detector over every frame listed in `frames.geojson` in
/// `frames_directory`, writes `detections.geojson` there, and reports
/// progress after each frame. Blocking: run it off the async runtime.
pub fn detect_video_frames(
    model: &Path,
    frames_directory: &Path,
    video: &str,
    min_confidence: f32,
    progress: &dyn Fn(DetectionProgress),
) -> Result<DetectionReport, DetectError> {
    reset_cancel();
    let index = frames_directory.join("frames.geojson");
    if !index.exists() {
        return Err(DetectError::NoFrames);
    }
    let frames = frames::read_frames_index(&index)?;
    if frames.is_empty() {
        return Err(DetectError::NoFrames);
    }
    let detector = Detector::load(model)?;
    let mut detections = Vec::new();
    let mut done = 0;
    for frame in &frames {
        if cancelled() {
            break;
        }
        detections.extend(frames::detect_in_frame(&detector, frame, min_confidence)?);
        done += 1;
        progress(DetectionProgress {
            done,
            total: frames.len(),
            detections: detections.len(),
        });
    }
    let model_name = model
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let geojson = frames::detections_geojson(&detections, video, &model_name);
    let path = frames_directory.join("detections.geojson");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&geojson).unwrap_or_default(),
    )
    .map_err(|e| DetectError::Image {
        path: path.display().to_string(),
        detail: e.to_string(),
    })?;
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    for d in &detections {
        *counts.entry(d.class_name.clone()).or_default() += 1;
    }
    let mut by_class: Vec<(String, usize)> = counts.into_iter().collect();
    by_class.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(DetectionReport {
        frames: done,
        detections: detections.len(),
        by_class,
        path: path.display().to_string(),
        cancelled: done < frames.len(),
    })
}

type Plan = std::sync::Arc<TypedRunnableModel>;

/// A box found in one image, in that image's pixels.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxDetection {
    pub class_id: usize,
    pub class_name: String,
    pub confidence: f32,
    /// Centre and size, in pixels of the model input.
    pub cx: f32,
    pub cy: f32,
    pub width: f32,
    pub height: f32,
}

/// Overlap above which two boxes of the same class are one object.
const NMS_IOU: f32 = 0.45;

pub struct Detector {
    plan: Plan,
    names: Vec<String>,
    /// Side of the square model input, in pixels.
    pub input_size: u32,
}

/// Class names from the Ultralytics `names` metadata, a Python dict such
/// as `{0: 'person', 1: 'bicycle'}`.
fn parse_names(text: &str) -> Vec<String> {
    let mut names: Vec<(usize, String)> = text
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .split(", ")
        .filter_map(|entry| {
            let (index, name) = entry.split_once(':')?;
            let name = name.trim().trim_matches(|c| c == '\'' || c == '"');
            Some((index.trim().parse().ok()?, name.to_string()))
        })
        .collect();
    names.sort_by_key(|(i, _)| *i);
    names.into_iter().map(|(_, name)| name).collect()
}

fn iou(a: &BoxDetection, b: &BoxDetection) -> f32 {
    let (ax0, ay0, ax1, ay1) = (
        a.cx - a.width / 2.0,
        a.cy - a.height / 2.0,
        a.cx + a.width / 2.0,
        a.cy + a.height / 2.0,
    );
    let (bx0, by0, bx1, by1) = (
        b.cx - b.width / 2.0,
        b.cy - b.height / 2.0,
        b.cx + b.width / 2.0,
        b.cy + b.height / 2.0,
    );
    let w = (ax1.min(bx1) - ax0.max(bx0)).max(0.0);
    let h = (ay1.min(by1) - ay0.max(by0)).max(0.0);
    let intersection = w * h;
    let union = a.width * a.height + b.width * b.height - intersection;
    if union <= 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Non-maximum suppression per class: keeps the most confident box of
/// each group of overlapping boxes.
pub fn non_max_suppression(mut boxes: Vec<BoxDetection>) -> Vec<BoxDetection> {
    boxes.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    let mut kept: Vec<BoxDetection> = Vec::new();
    for candidate in boxes {
        if kept
            .iter()
            .all(|k| k.class_id != candidate.class_id || iou(k, &candidate) < NMS_IOU)
        {
            kept.push(candidate);
        }
    }
    kept
}

impl Detector {
    pub fn load(path: &Path) -> Result<Self, DetectError> {
        let model_error = |e: TractError| DetectError::Model(format!("{e:#}"));
        let model = tract_onnx::onnx()
            .model_for_path(path)
            .map_err(model_error)?;
        let property = |key: &str| {
            let tensor = model.properties.get(key)?;
            let view = tensor.to_plain_array_view::<String>().ok()?;
            view.iter().next().cloned()
        };
        let names = property("onnx.metadata_props.names")
            .map(|text| parse_names(&text))
            .unwrap_or_default();
        let input_size = property("onnx.metadata_props.imgsz")
            .and_then(|text| {
                text.trim_matches(['[', ']'])
                    .split(',')
                    .next()?
                    .trim()
                    .parse()
                    .ok()
            })
            .unwrap_or(640u32);
        let plan = model
            .with_input_fact(
                0,
                f32::fact([1, 3, input_size as usize, input_size as usize]).into(),
            )
            .map_err(model_error)?
            .into_optimized()
            .map_err(model_error)?
            .into_runnable()
            .map_err(model_error)?;
        if names.is_empty() {
            log::warn!("Detection model has no class names; classes are shown as numbers");
        }
        Ok(Self {
            plan,
            names,
            input_size,
        })
    }

    pub fn class_names(&self) -> &[String] {
        &self.names
    }

    fn class_name(&self, id: usize) -> String {
        self.names
            .get(id)
            .cloned()
            .unwrap_or_else(|| format!("class {id}"))
    }

    /// Detects objects in a square image of `input_size` pixels.
    pub fn detect(
        &self,
        image: &RgbImage,
        min_confidence: f32,
    ) -> Result<Vec<BoxDetection>, DetectError> {
        let size = self.input_size as usize;
        if image.width() as usize != size || image.height() as usize != size {
            return Err(DetectError::Inference(format!(
                "image is {}×{}, the model expects {size}×{size}",
                image.width(),
                image.height()
            )));
        }
        let input: Tensor =
            tract_ndarray::Array4::from_shape_fn((1, 3, size, size), |(_, c, y, x)| {
                f32::from(image.get_pixel(x as u32, y as u32)[c]) / 255.0
            })
            .into();
        let outputs = self
            .plan
            .run(tvec!(input.into()))
            .map_err(|e| DetectError::Inference(format!("{e:#}")))?;
        let output = outputs[0]
            .to_plain_array_view::<f32>()
            .map_err(|e| DetectError::Inference(format!("{e:#}")))?;
        let shape = output.shape().to_vec();
        if shape.len() != 3 || shape[1] < 5 {
            return Err(DetectError::Inference(format!(
                "unexpected output shape {shape:?}"
            )));
        }
        let (rows, anchors) = (shape[1], shape[2]);
        let mut boxes = Vec::new();
        for a in 0..anchors {
            let (mut best, mut score) = (0, f32::MIN);
            for c in 4..rows {
                let s = output[[0, c, a]];
                if s > score {
                    score = s;
                    best = c - 4;
                }
            }
            if score < min_confidence {
                continue;
            }
            boxes.push(BoxDetection {
                class_id: best,
                class_name: self.class_name(best),
                confidence: score,
                cx: output[[0, 0, a]],
                cy: output[[0, 1, a]],
                width: output[[0, 2, a]],
                height: output[[0, 3, a]],
            });
        }
        Ok(non_max_suppression(boxes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detection(class_id: usize, confidence: f32, cx: f32) -> BoxDetection {
        BoxDetection {
            class_id,
            class_name: String::new(),
            confidence,
            cx,
            cy: 100.0,
            width: 50.0,
            height: 50.0,
        }
    }

    #[test]
    fn reads_ultralytics_class_names() {
        assert_eq!(
            parse_names("{0: 'person', 1: 'bicycle', 2: 'traffic light'}"),
            ["person", "bicycle", "traffic light"]
        );
        assert!(parse_names("").is_empty());
    }

    #[test]
    fn suppresses_overlapping_boxes_of_the_same_class() {
        let kept = non_max_suppression(vec![
            detection(0, 0.6, 100.0),
            detection(0, 0.9, 105.0),
            detection(1, 0.5, 105.0),
            detection(0, 0.7, 300.0),
        ]);
        let kept: Vec<_> = kept.iter().map(|d| (d.class_id, d.confidence)).collect();
        assert_eq!(kept, [(0, 0.9), (0, 0.7), (1, 0.5)]);
    }

    /// Runs a real model on Ultralytics' bus example, resized to 640×640:
    /// `V360LAB_YOLO_MODEL=yolo11n.onnx V360LAB_YOLO_IMAGE=bus640.jpg
    /// cargo test detect::tests::detects_with_a_real_model -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a YOLO ONNX model and an image (see the doc comment)"]
    fn detects_with_a_real_model() {
        let model = std::env::var("V360LAB_YOLO_MODEL").expect("V360LAB_YOLO_MODEL");
        let image = std::env::var("V360LAB_YOLO_IMAGE").expect("V360LAB_YOLO_IMAGE");
        let detector = Detector::load(Path::new(&model)).unwrap();
        let image = image::open(image).unwrap().to_rgb8();
        let started = std::time::Instant::now();
        let found = detector.detect(&image, 0.25).unwrap();
        println!("{} ms", started.elapsed().as_millis());
        for d in &found {
            println!(
                "{} {:.3} cx {:.1} cy {:.1} w {:.1} h {:.1}",
                d.class_name, d.confidence, d.cx, d.cy, d.width, d.height
            );
        }
        assert!(found.iter().any(|d| d.class_name == "bus"));
        assert!(found.iter().filter(|d| d.class_name == "person").count() >= 3);
    }

    /// Whole pipeline on a frames folder with one panorama:
    /// `V360LAB_YOLO_MODEL=yolo11n.onnx V360LAB_YOLO_PANORAMA=pano_bus_right.jpg
    /// cargo test detect::tests::detects_a_frames_folder -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a YOLO ONNX model and a panorama (see the doc comment)"]
    fn detects_a_frames_folder() {
        let model = std::env::var("V360LAB_YOLO_MODEL").expect("V360LAB_YOLO_MODEL");
        let panorama = std::env::var("V360LAB_YOLO_PANORAMA").expect("V360LAB_YOLO_PANORAMA");
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(&panorama, dir.path().join("V1_00001.jpg")).unwrap();
        std::fs::write(
            dir.path().join("frames.geojson"),
            r#"{"type":"FeatureCollection","features":[{"type":"Feature",
               "geometry":{"type":"Point","coordinates":[11.12,46.07]},
               "properties":{"file":"V1_00001.jpg","headingDeg":10.0,"videoSeconds":0.0}}]}"#,
        )
        .unwrap();
        let report = detect_video_frames(Path::new(&model), dir.path(), "V1.MP4", 0.4, &|p| {
            println!("progress {p:?}")
        })
        .unwrap();
        println!("{report:?}");
        let geojson: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report.path).unwrap()).unwrap();
        let bus = geojson["features"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["properties"]["class"] == "bus")
            .expect("bus");
        // Heading 10° + bus 90° to the right of the camera.
        let azimuth = bus["properties"]["azimuthDeg"].as_f64().unwrap();
        assert!((azimuth - 100.0).abs() < 10.0, "azimuth {azimuth}");
        assert_eq!(report.frames, 1);
        assert!(!report.cancelled);
    }
}
