//! Genuine 360° capture: six independent 90° cubemap views and a 2:1
//! equirectangular PNG. The 3D renderer draws each face; this module only
//! describes the lens and resamples the captured RGBA pixel buffers.
//!
//! Face order is +X, -X, +Y, -Y, +Z, -Z in the simulator's world axes.
//! The viewer uses a right-handed view and reverse-Z DirectX projection.

use glam::{Mat4, Vec3};

pub const FACES: usize = 6;
/// Six 1024² captures produce a 4096×2048 equirectangular image.
pub const DEFAULT_FACE_SIZE: u32 = 1024;
pub const MIN_FACE_SIZE: u32 = 256;

/// The world-facing lens for a single cube face, at a fixed camera origin.
#[derive(Debug, Clone, Copy)]
pub struct FaceView {
    pub eye: Vec3,
    pub forward: Vec3,
    pub view: Mat4,
    pub projection: Mat4,
}

fn axes(face: usize) -> (Vec3, Vec3) {
    match face {
        0 => (Vec3::X, Vec3::Z),
        1 => (Vec3::NEG_X, Vec3::Z),
        2 => (Vec3::Y, Vec3::Z),
        3 => (Vec3::NEG_Y, Vec3::Z),
        4 => (Vec3::Z, Vec3::Y),
        5 => (Vec3::NEG_Z, Vec3::NEG_Y),
        _ => panic!("invalid cubemap face"),
    }
}

pub fn face_view(eye: Vec3, near: f32, face: usize) -> FaceView {
    let (forward, up) = axes(face);
    FaceView {
        eye,
        forward,
        view: glam::camera::rh::view::look_to_mat4(eye, forward, up),
        projection: glam::camera::rh::proj::directx::perspective_infinite_reverse(std::f32::consts::FRAC_PI_2, 1.0, near.max(0.001)),
    }
}

/// Identify a face and its camera-plane coordinates in [-1, 1].
fn face_uv(dir: Vec3) -> (usize, f32, f32) {
    let abs = dir.abs();
    let face = if abs.x >= abs.y && abs.x >= abs.z {
        if dir.x >= 0.0 { 0 } else { 1 }
    } else if abs.y >= abs.z {
        if dir.y >= 0.0 { 2 } else { 3 }
    } else if dir.z >= 0.0 {
        4
    } else {
        5
    };
    let (forward, up) = axes(face);
    let right = forward.cross(up);
    let depth = dir.dot(forward);
    (face, dir.dot(right) / depth, dir.dot(up) / depth)
}

fn sample(face: u32, faces: &[u8], direction: Vec3) -> [u8; 4] {
    let (index, u, v) = face_uv(direction);
    let extent = (face - 1) as f32;
    let xf = ((u + 1.0) * face as f32 * 0.5 - 0.5).clamp(0.0, extent);
    let yf = ((1.0 - v) * face as f32 * 0.5 - 0.5).clamp(0.0, extent);
    let x0 = xf.floor() as usize;
    let y0 = yf.floor() as usize;
    let x1 = (x0 + 1).min(face as usize - 1);
    let y1 = (y0 + 1).min(face as usize - 1);
    let (tx, ty) = (xf.fract(), yf.fract());
    let stride = face as usize;
    let offset = index * stride * stride * 4;
    let pixel = |x: usize, y: usize, channel: usize| faces[offset + (y * stride + x) * 4 + channel] as f32;
    let mut out = [0; 4];
    for (channel, value) in out.iter_mut().enumerate() {
        let top = pixel(x0, y0, channel) * (1.0 - tx) + pixel(x1, y0, channel) * tx;
        let bottom = pixel(x0, y1, channel) * (1.0 - tx) + pixel(x1, y1, channel) * tx;
        *value = (top * (1.0 - ty) + bottom * ty).round() as u8;
    }
    out
}

/// Stitch six consecutive square RGBA faces into a top-left-origin,
/// full-sphere equirectangular image. Caller supplies actual rendered faces,
/// never a duplicated or rotated ordinary screenshot.
pub fn equirectangular(face: u32, faces: &[u8]) -> Result<((u32, u32), Vec<u8>), String> {
    if !(MIN_FACE_SIZE..=2048).contains(&face) {
        return Err(format!("Unsupported panorama face size: {face}"));
    }
    let side = face as usize;
    let expected = side * side * 4 * FACES;
    if faces.len() != expected {
        return Err(format!("Expected {expected} bytes across six cube faces, got {}", faces.len()));
    }
    let (width, height) = (face * 4, face * 2);
    let mut output = vec![0u8; width as usize * height as usize * 4];
    for y in 0..height {
        let latitude = std::f32::consts::FRAC_PI_2 - std::f32::consts::PI * (y as f32 + 0.5) / height as f32;
        let (sin_lat, cos_lat) = latitude.sin_cos();
        for x in 0..width {
            let longitude = std::f32::consts::TAU * (x as f32 + 0.5) / width as f32 - std::f32::consts::PI;
            let (sin_lon, cos_lon) = longitude.sin_cos();
            let direction = Vec3::new(cos_lat * cos_lon, cos_lat * sin_lon, sin_lat);
            let color = sample(face, faces, direction);
            let offset = (y as usize * width as usize + x as usize) * 4;
            output[offset..offset + 4].copy_from_slice(&color);
        }
    }
    Ok(((width, height), output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_point_in_six_distinct_perpendicular_directions() {
        for i in 0..FACES {
            let lens = face_view(Vec3::new(2.0, 3.0, 5.0), 0.1, i);
            assert_eq!(lens.forward, axes(i).0);
            let (face, u, v) = face_uv(lens.forward);
            assert_eq!(face, i);
            assert!(u.abs() < 1e-6 && v.abs() < 1e-6);
            assert!(lens.view.is_finite() && lens.projection.is_finite());
        }
    }

    #[test]
    fn stitched_panorama_has_all_six_directions_and_two_to_one_ratio() {
        let side = MIN_FACE_SIZE as usize;
        let mut faces = vec![0u8; side * side * 4 * FACES];
        for i in 0..FACES {
            for px in faces[i * side * side * 4..(i + 1) * side * side * 4].as_chunks_mut::<4>().0 {
                px.copy_from_slice(&[(i + 1) as u8, 20, 30, 255]);
            }
        }
        let ((w, h), output) = equirectangular(side as u32, &faces).expect("six captured faces");
        assert_eq!((w, h), (1024, 512));
        let pixel = |x: u32, y: u32| output[(y as usize * w as usize + x as usize) * 4];
        assert_eq!(pixel(w / 2, h / 2), 1, "+X at middle");
        assert_eq!(pixel(w / 4, h / 2), 4, "-Y at quarter");
        assert_eq!(pixel(3 * w / 4, h / 2), 3, "+Y at three quarters");
        assert_eq!(pixel(0, h / 2), 2, "-X at seam");
        assert_eq!(pixel(w / 2, 0), 5, "+Z at zenith");
        assert_eq!(pixel(w / 2, h - 1), 6, "-Z at nadir");
        assert_eq!(
            equirectangular(256, &faces[..100]),
            Err("Expected 1572864 bytes across six cube faces, got 100".into())
        );
    }
}
