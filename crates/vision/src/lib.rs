//! Vision for the bag deposit desk, in pure Rust (tract — no native ONNX runtime):
//!
//! * **YuNet** (OpenCV Model Zoo, MIT) — face detection + 5 landmarks
//! * **SFace** (OpenCV Model Zoo, Apache-2.0) — 128-d face embedding, compared by cosine similarity
//! * dominant bag colour → a named colour staff can see at a glance
//!
//! Pre/post-processing follows OpenCV's `FaceDetectorYN` / `FaceRecognizerSF`.

mod align;
pub mod color;

use std::path::Path;

use image::{DynamicImage, GenericImageView, RgbImage, imageops::FilterType};
use tract_onnx::prelude::*;

pub const YUNET_FILE: &str = "face_detection_yunet_2023mar.onnx";
pub const SFACE_FILE: &str = "face_recognition_sface_2021dec.onnx";
/// SFace authors' recommended cosine threshold for "same person".
pub const DEFAULT_MATCH_THRESHOLD: f32 = 0.363;

const DET_SIZE: usize = 640;
const STRIDES: [usize; 3] = [8, 16, 32];
const SCORE_THRESHOLD: f32 = 0.8;
const NMS_IOU: f32 = 0.3;

type Plan = std::sync::Arc<TypedSimplePlan>;

#[derive(Debug, thiserror::Error)]
pub enum VisionError {
    #[error("could not read the image: {0}")]
    Image(#[from] image::ImageError),
    #[error("model error: {0}")]
    Model(String),
}

impl From<TractError> for VisionError {
    fn from(e: TractError) -> Self {
        VisionError::Model(e.to_string())
    }
}

/// One detected face in original-image pixel coordinates.
#[derive(Debug, Clone)]
pub struct Face {
    /// x, y, width, height
    pub bbox: [f32; 4],
    /// right eye, left eye, nose tip, right mouth corner, left mouth corner
    pub landmarks: [[f32; 2]; 5],
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct FaceAnalysis {
    /// How many faces were found (useful to warn "several people in frame").
    pub faces_found: usize,
    /// Embedding of the largest face, L2-normalised. `None` when no face was found.
    pub embedding: Option<Vec<f32>>,
}

pub struct FaceEngine {
    detector: Plan,
    recognizer: Plan,
}

impl FaceEngine {
    /// Loads both models from `dir`.
    pub fn load(dir: impl AsRef<Path>) -> Result<Self, VisionError> {
        let dir = dir.as_ref();
        let detector = tract_onnx::onnx()
            .model_for_path(dir.join(YUNET_FILE))?
            .into_optimized()?
            .into_runnable()?;
        let recognizer = tract_onnx::onnx()
            .model_for_path(dir.join(SFACE_FILE))?
            .into_optimized()?
            .into_runnable()?;
        Ok(Self { detector, recognizer })
    }

    /// Decodes a photo, finds faces and embeds the largest one.
    pub fn analyze(&self, photo: &[u8]) -> Result<FaceAnalysis, VisionError> {
        let img = image::load_from_memory(photo)?.to_rgb8();
        let faces = self.detect(&img)?;
        let largest = faces.iter().max_by(|a, b| (a.bbox[2] * a.bbox[3]).total_cmp(&(b.bbox[2] * b.bbox[3])));
        let embedding = match largest {
            Some(face) => Some(self.embed(&img, face)?),
            None => None,
        };
        Ok(FaceAnalysis { faces_found: faces.len(), embedding })
    }

    /// YuNet detection on a letterboxed 640×640 copy of the image.
    pub fn detect(&self, img: &RgbImage) -> Result<Vec<Face>, VisionError> {
        let (w, h) = img.dimensions();
        let scale = DET_SIZE as f32 / w.max(h) as f32;
        let (nw, nh) = (((w as f32 * scale).round() as u32).max(1), ((h as f32 * scale).round() as u32).max(1));
        let resized = image::imageops::resize(img, nw, nh, FilterType::Triangle);

        // NCHW, BGR, 0–255 (as OpenCV feeds it); padding stays black.
        let mut input = tract_ndarray::Array4::<f32>::zeros((1, 3, DET_SIZE, DET_SIZE));
        for (x, y, p) in resized.enumerate_pixels() {
            let (x, y) = (x as usize, y as usize);
            input[[0, 0, y, x]] = p[2] as f32;
            input[[0, 1, y, x]] = p[1] as f32;
            input[[0, 2, y, x]] = p[0] as f32;
        }
        let out = self.detector.run(tvec!(Tensor::from(input).into()))?;
        let view = |i: usize| -> Result<&[f32], VisionError> {
            out[i].to_plain_array_view::<f32>()?.to_slice().ok_or_else(|| VisionError::Model("non-contiguous output".into()))
        };

        let mut faces = Vec::new();
        for (k, stride) in STRIDES.iter().enumerate() {
            let (cls, obj, bbox, kps) = (view(k)?, view(3 + k)?, view(6 + k)?, view(9 + k)?);
            let cols = DET_SIZE / stride;
            let s = *stride as f32;
            for idx in 0..cls.len() {
                let score = (cls[idx].clamp(0.0, 1.0) * obj[idx].clamp(0.0, 1.0)).sqrt();
                if score < SCORE_THRESHOLD {
                    continue;
                }
                let (r, c) = ((idx / cols) as f32, (idx % cols) as f32);
                let b = &bbox[idx * 4..idx * 4 + 4];
                let (cx, cy) = ((c + b[0]) * s, (r + b[1]) * s);
                let (bw, bh) = (b[2].exp() * s, b[3].exp() * s);
                let k = &kps[idx * 10..idx * 10 + 10];
                let mut landmarks = [[0.0; 2]; 5];
                for (n, lm) in landmarks.iter_mut().enumerate() {
                    *lm = [(k[2 * n] + c) * s / scale, (k[2 * n + 1] + r) * s / scale];
                }
                faces.push(Face {
                    bbox: [(cx - bw / 2.0) / scale, (cy - bh / 2.0) / scale, bw / scale, bh / scale],
                    landmarks,
                    score,
                });
            }
        }
        Ok(nms(faces))
    }

    /// SFace embedding of an aligned 112×112 crop, L2-normalised.
    pub fn embed(&self, img: &RgbImage, face: &Face) -> Result<Vec<f32>, VisionError> {
        let crop = align::align_crop(img, &face.landmarks);
        // OpenCV: blobFromImage(aligned_bgr, swapRB=true) → RGB, 0–255.
        let mut input = tract_ndarray::Array4::<f32>::zeros((1, 3, 112, 112));
        for (x, y, p) in crop.enumerate_pixels() {
            let (x, y) = (x as usize, y as usize);
            input[[0, 0, y, x]] = p[0] as f32;
            input[[0, 1, y, x]] = p[1] as f32;
            input[[0, 2, y, x]] = p[2] as f32;
        }
        let out = self.recognizer.run(tvec!(Tensor::from(input).into()))?;
        let v = out[0].to_plain_array_view::<f32>()?;
        Ok(l2_normalize(&v.iter().copied().collect::<Vec<_>>()))
    }
}

/// Cosine similarity of two L2-normalised embeddings (1.0 = identical).
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
    v.iter().map(|x| x / n).collect()
}

fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let (x1, y1) = (a[0].max(b[0]), a[1].max(b[1]));
    let (x2, y2) = ((a[0] + a[2]).min(b[0] + b[2]), (a[1] + a[3]).min(b[1] + b[3]));
    let inter = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    inter / (a[2] * a[3] + b[2] * b[3] - inter).max(1e-6)
}

fn nms(mut faces: Vec<Face>) -> Vec<Face> {
    faces.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut keep: Vec<Face> = Vec::new();
    for f in faces {
        if keep.iter().all(|k| iou(&k.bbox, &f.bbox) < NMS_IOU) {
            keep.push(f);
        }
    }
    keep
}

/// Re-encodes a photo as a JPEG no larger than `max_side` px (strips EXIF/GPS metadata too).
pub fn normalize_jpeg(photo: &[u8], max_side: u32) -> Result<Vec<u8>, VisionError> {
    let img = image::load_from_memory(photo)?;
    let img = if img.width().max(img.height()) > max_side {
        img.resize(max_side, max_side, FilterType::Triangle)
    } else {
        img
    };
    let mut out = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img.to_rgb8()).write_to(&mut out, image::ImageFormat::Jpeg)?;
    Ok(out.into_inner())
}

/// Image dimensions without full processing (for validation).
pub fn dimensions(photo: &[u8]) -> Result<(u32, u32), VisionError> {
    Ok(image::load_from_memory(photo)?.dimensions())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Option<FaceEngine> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../models");
        FaceEngine::load(dir).ok()
    }

    #[test]
    fn blank_image_has_no_faces() {
        let Some(e) = engine() else { return eprintln!("models not present; skipped") };
        let img = RgbImage::from_pixel(800, 600, image::Rgb([200, 200, 200]));
        assert!(e.detect(&img).unwrap().is_empty());
    }

    #[test]
    fn embedding_is_normalized_and_self_similar() {
        let Some(e) = engine() else { return eprintln!("models not present; skipped") };
        let img = RgbImage::from_fn(300, 300, |x, y| image::Rgb([(x % 255) as u8, (y % 255) as u8, 120]));
        let face = Face {
            bbox: [100.0, 100.0, 100.0, 120.0],
            landmarks: [[130.0, 140.0], [170.0, 140.0], [150.0, 165.0], [135.0, 190.0], [165.0, 190.0]],
            score: 1.0,
        };
        let a = e.embed(&img, &face).unwrap();
        assert_eq!(a.len(), 128);
        assert!((cosine(&a, &a) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn nms_drops_overlaps() {
        let f = |x: f32, s: f32| Face { bbox: [x, 0.0, 100.0, 100.0], landmarks: [[0.0; 2]; 5], score: s };
        let kept = nms(vec![f(0.0, 0.9), f(5.0, 0.95), f(300.0, 0.85)]);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].score, 0.95);
    }
}
