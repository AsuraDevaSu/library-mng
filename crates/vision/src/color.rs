//! Dominant bag colour → a simple colour name staff can scan shelves for.

use image::imageops::FilterType;

fn bucket(r: u8, g: u8, b: u8) -> &'static str {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let v = max;
    let s = if max > 0.0 { (max - min) / max } else { 0.0 };
    if v < 0.2 {
        return "Black";
    }
    if s < 0.18 {
        return if v > 0.82 { "White" } else if v < 0.35 { "Black" } else { "Grey" };
    }
    let d = max - min;
    let mut hue = if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    if hue < 0.0 {
        hue += 360.0;
    }
    match hue {
        h if !(15.0..345.0).contains(&h) => if v < 0.45 { "Brown" } else { "Red" },
        h if h < 42.0 => if v < 0.6 { "Brown" } else { "Orange" },
        h if h < 68.0 => if v < 0.5 { "Brown" } else { "Yellow" },
        h if h < 165.0 => "Green",
        h if h < 195.0 => "Teal",
        h if h < 255.0 => if v < 0.45 { "Navy" } else { "Blue" },
        h if h < 290.0 => "Purple",
        _ => "Pink",
    }
}

/// Most common colour bucket in the centre 60% of the photo, with its average RGB as hex.
pub fn dominant_color(photo: &[u8]) -> Result<(String, String), image::ImageError> {
    let img = image::load_from_memory(photo)?
        .resize_exact(80, 80, FilterType::Triangle)
        .to_rgb8();
    let mut counts: std::collections::HashMap<&str, (u32, [u64; 3])> = Default::default();
    for (x, y, p) in img.enumerate_pixels() {
        if !(16..64).contains(&x) || !(16..64).contains(&y) {
            continue;
        }
        let e = counts.entry(bucket(p[0], p[1], p[2])).or_default();
        e.0 += 1;
        for c in 0..3 {
            e.1[c] += p[c] as u64;
        }
    }
    let (name, (n, sum)) = counts.into_iter().max_by_key(|(_, (n, _))| *n).expect("non-empty crop");
    let hex = format!("#{:02x}{:02x}{:02x}", sum[0] / n as u64, sum[1] / n as u64, sum[2] / n as u64);
    Ok((name.to_string(), hex))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};

    fn png(color: [u8; 3]) -> Vec<u8> {
        let img = RgbImage::from_pixel(120, 90, Rgb(color));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn names_common_bag_colours() {
        for (rgb, name) in [
            ([20, 20, 22], "Black"),
            ([30, 45, 95], "Navy"),
            ([40, 110, 220], "Blue"),
            ([200, 40, 40], "Red"),
            ([60, 160, 70], "Green"),
            ([240, 210, 60], "Yellow"),
            ([125, 80, 45], "Brown"),
            ([140, 140, 140], "Grey"),
            ([235, 105, 170], "Pink"),
        ] {
            assert_eq!(dominant_color(&png(rgb)).unwrap().0, name, "{rgb:?}");
        }
    }
}
