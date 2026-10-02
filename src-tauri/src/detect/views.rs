//! Perspective views of an equirectangular (360°) image, and the way back
//! from a pixel of a view to a direction on the sphere.
//!
//! Object detectors are trained on ordinary photos: on the equirectangular
//! image objects are stretched near the top and bottom and cut at the left
//! and right edges. Each frame is therefore split into square views like
//! an ordinary camera would take, and the detections are mapped back to
//! directions.
//!
//! Directions use the image frame of the 360° viewer: x right, y up, z
//! backwards (the centre of the equirectangular image looks along -z).
//! Yaw is measured from the front, clockwise seen from above (like a
//! compass bearing); pitch is positive upwards.

use image::{Rgb, RgbImage};

/// A square pinhole view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub fov_deg: f64,
    pub size: u32,
}

/// Eight horizontal views, 90° wide, every 45°: every direction within
/// ±45° of the horizon falls in the central part of one of them.
pub fn horizontal_views(size: u32) -> Vec<View> {
    (0..8)
        .map(|i| View {
            yaw_deg: f64::from(i) * 45.0,
            pitch_deg: 0.0,
            fov_deg: 90.0,
            size,
        })
        .collect()
}

/// Direction (unit vector) seen through pixel (`px`, `py`) of `view`.
pub fn pixel_direction(view: &View, px: f64, py: f64) -> [f64; 3] {
    let half = (view.fov_deg.to_radians() / 2.0).tan();
    let size = f64::from(view.size);
    let x = (2.0 * px / size - 1.0) * half;
    let y = (1.0 - 2.0 * py / size) * half;
    let (x, y, z) = (x, y, -1.0);
    // Pitch: rotation about x (positive looks up).
    let (sp, cp) = view.pitch_deg.to_radians().sin_cos();
    let (y, z) = (y * cp - z * sp, y * sp + z * cp);
    // Yaw: clockwise seen from above (positive looks right).
    let (sy, cy) = (-view.yaw_deg.to_radians()).sin_cos();
    let (x, z) = (x * cy + z * sy, -x * sy + z * cy);
    let length = (x * x + y * y + z * z).sqrt();
    [x / length, y / length, z / length]
}

/// (yaw, pitch) in degrees of a direction; yaw in -180..180.
pub fn direction_angles(d: [f64; 3]) -> (f64, f64) {
    let yaw = d[0].atan2(-d[2]).to_degrees();
    let pitch = d[1].clamp(-1.0, 1.0).asin().to_degrees();
    (yaw, pitch)
}

/// Angle in degrees between two directions given as (yaw, pitch).
pub fn angular_distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (ya, pa) = (a.0.to_radians(), a.1.to_radians());
    let (yb, pb) = (b.0.to_radians(), b.1.to_radians());
    let cos = pa.sin() * pb.sin() + pa.cos() * pb.cos() * (ya - yb).cos();
    cos.clamp(-1.0, 1.0).acos().to_degrees()
}

/// Bilinear sample of an equirectangular image in direction `d`, wrapping
/// around horizontally.
fn sample(image: &RgbImage, d: [f64; 3]) -> Rgb<u8> {
    let (w, h) = (f64::from(image.width()), f64::from(image.height()));
    let lon = d[0].atan2(-d[2]);
    let lat = d[1].clamp(-1.0, 1.0).asin();
    let u = (lon / std::f64::consts::TAU + 0.5) * w - 0.5;
    let v = ((0.5 - lat / std::f64::consts::PI) * h - 0.5).clamp(0.0, h - 1.0);
    let (x0, y0) = (u.floor(), v.floor());
    let (fx, fy) = (u - x0, v - y0);
    let wrap = |x: f64| (x.rem_euclid(w)) as u32;
    let row = |y: f64| (y.min(h - 1.0)) as u32;
    let (xa, xb, ya, yb) = (wrap(x0), wrap(x0 + 1.0), row(y0), row(y0 + 1.0));
    let p = |x, y| image.get_pixel(x, y).0;
    let (a, b, c, e) = (p(xa, ya), p(xb, ya), p(xa, yb), p(xb, yb));
    let mut out = [0u8; 3];
    for i in 0..3 {
        let top = f64::from(a[i]) * (1.0 - fx) + f64::from(b[i]) * fx;
        let bottom = f64::from(c[i]) * (1.0 - fx) + f64::from(e[i]) * fx;
        out[i] = (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8;
    }
    Rgb(out)
}

/// Renders `view` from an equirectangular image of any proportions.
pub fn render_view(equirect: &RgbImage, view: &View) -> RgbImage {
    RgbImage::from_fn(view.size, view.size, |x, y| {
        sample(
            equirect,
            pixel_direction(view, f64::from(x) + 0.5, f64::from(y) + 0.5),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(yaw: f64, pitch: f64) -> View {
        View {
            yaw_deg: yaw,
            pitch_deg: pitch,
            fov_deg: 90.0,
            size: 640,
        }
    }

    fn close(a: (f64, f64), b: (f64, f64)) {
        assert!(angular_distance(a, b) < 1e-6, "{a:?} != {b:?}");
    }

    #[test]
    fn view_centres_point_where_the_view_looks() {
        close(
            direction_angles(pixel_direction(&view(0.0, 0.0), 320.0, 320.0)),
            (0.0, 0.0),
        );
        close(
            direction_angles(pixel_direction(&view(90.0, 0.0), 320.0, 320.0)),
            (90.0, 0.0),
        );
        close(
            direction_angles(pixel_direction(&view(-135.0, 30.0), 320.0, 320.0)),
            (-135.0, 30.0),
        );
    }

    #[test]
    fn view_edges_are_half_the_field_of_view_away() {
        // Right edge of the front view: 45° to the right.
        close(
            direction_angles(pixel_direction(&view(0.0, 0.0), 640.0, 320.0)),
            (45.0, 0.0),
        );
        // Top edge: 45° up.
        close(
            direction_angles(pixel_direction(&view(0.0, 0.0), 320.0, 0.0)),
            (0.0, 45.0),
        );
    }

    #[test]
    fn eight_views_cover_the_horizon() {
        let views = horizontal_views(640);
        assert_eq!(views.len(), 8);
        for yaw in (-180..180).step_by(5) {
            let nearest = views
                .iter()
                .map(|v| angular_distance((f64::from(yaw), 0.0), (v.yaw_deg, 0.0)))
                .fold(f64::MAX, f64::min);
            assert!(nearest <= 22.5 + 1e-9);
        }
    }

    #[test]
    fn renders_the_matching_part_of_the_panorama() {
        // Left half red, right half blue: the view looking right sees blue.
        let pano = RgbImage::from_fn(400, 200, |x, _| {
            if x < 200 {
                Rgb([255, 0, 0])
            } else {
                Rgb([0, 0, 255])
            }
        });
        let right = render_view(
            &pano,
            &View {
                size: 32,
                ..view(90.0, 0.0)
            },
        );
        assert_eq!(right.get_pixel(16, 16).0, [0, 0, 255]);
        let left = render_view(
            &pano,
            &View {
                size: 32,
                ..view(-90.0, 0.0)
            },
        );
        assert_eq!(left.get_pixel(16, 16).0, [255, 0, 0]);
    }
}
