//! Face alignment: similarity transform from YuNet's 5 landmarks to the
//! standard 112×112 ArcFace template (what OpenCV's `alignCrop` does).

use image::{Rgb, RgbImage};

const TEMPLATE: [[f32; 2]; 5] = [
    [38.2946, 51.6963],
    [73.5318, 51.5014],
    [56.0252, 71.7366],
    [41.5493, 92.3655],
    [70.7299, 92.2041],
];

/// Least-squares similarity transform `dst ≈ [a -b; b a]·src + t` (Umeyama, 2-D).
fn similarity(src: &[[f32; 2]; 5], dst: &[[f32; 2]; 5]) -> (f32, f32, f32, f32) {
    let mean = |p: &[[f32; 2]; 5]| {
        let (sx, sy) = p.iter().fold((0.0, 0.0), |(x, y), q| (x + q[0], y + q[1]));
        (sx / 5.0, sy / 5.0)
    };
    let (msx, msy) = mean(src);
    let (mdx, mdy) = mean(dst);
    let (mut num_a, mut num_b, mut den) = (0.0, 0.0, 0.0);
    for i in 0..5 {
        let (sx, sy) = (src[i][0] - msx, src[i][1] - msy);
        let (dx, dy) = (dst[i][0] - mdx, dst[i][1] - mdy);
        num_a += sx * dx + sy * dy;
        num_b += sx * dy - sy * dx;
        den += sx * sx + sy * sy;
    }
    let den = den.max(1e-6);
    let (a, b) = (num_a / den, num_b / den);
    let tx = mdx - (a * msx - b * msy);
    let ty = mdy - (b * msx + a * msy);
    (a, b, tx, ty)
}

/// Warps the face into a 112×112 crop using bilinear sampling.
pub fn align_crop(img: &RgbImage, landmarks: &[[f32; 2]; 5]) -> RgbImage {
    let (a, b, tx, ty) = similarity(landmarks, &TEMPLATE);
    // Inverse of [a -b; b a]: (1/(a²+b²))·[a b; -b a]
    let det = (a * a + b * b).max(1e-9);
    let (w, h) = img.dimensions();
    RgbImage::from_fn(112, 112, |u, v| {
        let (du, dv) = (u as f32 - tx, v as f32 - ty);
        let sx = (a * du + b * dv) / det;
        let sy = (-b * du + a * dv) / det;
        bilinear(img, sx, sy, w, h)
    })
}

fn bilinear(img: &RgbImage, x: f32, y: f32, w: u32, h: u32) -> Rgb<u8> {
    if x < 0.0 || y < 0.0 || x > (w - 1) as f32 || y > (h - 1) as f32 {
        return Rgb([0, 0, 0]);
    }
    let (x0, y0) = (x.floor() as u32, y.floor() as u32);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let p = |xx, yy| img.get_pixel(xx, yy).0;
    let (p00, p10, p01, p11) = (p(x0, y0), p(x1, y0), p(x0, y1), p(x1, y1));
    let mut out = [0u8; 3];
    for c in 0..3 {
        let top = p00[c] as f32 * (1.0 - fx) + p10[c] as f32 * fx;
        let bot = p01[c] as f32 * (1.0 - fx) + p11[c] as f32 * fx;
        out[c] = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
    }
    Rgb(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_maps_to_itself() {
        let (a, b, tx, ty) = similarity(&TEMPLATE, &TEMPLATE);
        assert!((a - 1.0).abs() < 1e-4 && b.abs() < 1e-4 && tx.abs() < 1e-3 && ty.abs() < 1e-3);
    }

    #[test]
    fn recovers_scale_and_shift() {
        let src = TEMPLATE.map(|[x, y]| [x * 2.0 + 10.0, y * 2.0 + 20.0]);
        let (a, b, tx, ty) = similarity(&src, &TEMPLATE);
        assert!((a - 0.5).abs() < 1e-4 && b.abs() < 1e-4);
        assert!((tx + 5.0).abs() < 1e-3 && (ty + 10.0).abs() < 1e-3);
    }
}
