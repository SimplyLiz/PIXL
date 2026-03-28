/// OKLab perceptual color space.
///
/// OKLab provides perceptually uniform color distances — equal ΔE in OKLab
/// corresponds to equal perceived color difference. This makes it ideal for:
/// - Color quantization (nearest-palette-color matching)
/// - Palette similarity scoring
/// - Style latent extraction (hue, luminance, saturation)
///
/// Based on Björn Ottosson's OKLab (2020).
/// Reference: https://bottosson.github.io/posts/oklab/

/// OKLab color: L (lightness 0-1), a (green-red), b (blue-yellow).
#[derive(Debug, Clone, Copy)]
pub struct OkLab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

/// Convert sRGB (0-255 per channel) to linear RGB (0-1).
fn srgb_to_linear(c: u8) -> f32 {
    let s = c as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear RGB (0-1) to sRGB (0-255).
fn linear_to_srgb(c: f32) -> u8 {
    let s = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Convert sRGB (0-255) to OKLab.
pub fn rgb_to_oklab(r: u8, g: u8, b: u8) -> OkLab {
    let lr = srgb_to_linear(r);
    let lg = srgb_to_linear(g);
    let lb = srgb_to_linear(b);

    let l = 0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb;
    let m = 0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb;
    let s = 0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    OkLab {
        l: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    }
}

/// Convert OKLab back to sRGB (0-255).
pub fn oklab_to_rgb(lab: &OkLab) -> (u8, u8, u8) {
    let l_ = lab.l + 0.3963377774 * lab.a + 0.2158037573 * lab.b;
    let m_ = lab.l - 0.1055613458 * lab.a - 0.0638541728 * lab.b;
    let s_ = lab.l - 0.0894841775 * lab.a - 1.2914855480 * lab.b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let lr = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let lg = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let lb = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    (linear_to_srgb(lr), linear_to_srgb(lg), linear_to_srgb(lb))
}

/// Perceptual color distance (ΔE) in OKLab space.
/// Returns a value where ~0.02 is just-noticeable-difference.
pub fn delta_e(a: &OkLab, b: &OkLab) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    (dl * dl + da * da + db * db).sqrt()
}

/// Find the nearest color in a palette using OKLab perceptual distance.
/// Returns the index of the closest color.
pub fn nearest_color(r: u8, g: u8, b: u8, palette: &[(u8, u8, u8)]) -> usize {
    let target = rgb_to_oklab(r, g, b);
    let mut best_idx = 0;
    let mut best_dist = f32::MAX;

    for (i, &(pr, pg, pb)) in palette.iter().enumerate() {
        let pal_lab = rgb_to_oklab(pr, pg, pb);
        let dist = delta_e(&target, &pal_lab);
        if dist < best_dist {
            best_dist = dist;
            best_idx = i;
        }
    }

    best_idx
}

/// Extract OKLab lightness from sRGB.
pub fn lightness(r: u8, g: u8, b: u8) -> f32 {
    rgb_to_oklab(r, g, b).l
}

/// Extract OKLab hue angle (0-360°) from sRGB.
pub fn hue(r: u8, g: u8, b: u8) -> f32 {
    let lab = rgb_to_oklab(r, g, b);
    lab.b.atan2(lab.a).to_degrees().rem_euclid(360.0)
}

/// Extract OKLab chroma (saturation intensity) from sRGB.
pub fn chroma(r: u8, g: u8, b: u8) -> f32 {
    let lab = rgb_to_oklab(r, g, b);
    (lab.a * lab.a + lab.b * lab.b).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_has_zero_lightness() {
        let lab = rgb_to_oklab(0, 0, 0);
        assert!(lab.l.abs() < 0.001, "black L should be ~0, got {}", lab.l);
    }

    #[test]
    fn white_has_full_lightness() {
        let lab = rgb_to_oklab(255, 255, 255);
        assert!(
            (lab.l - 1.0).abs() < 0.01,
            "white L should be ~1, got {}",
            lab.l
        );
    }

    #[test]
    fn roundtrip_preserves_color() {
        let test_colors = [
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (128, 64, 192),
            (42, 31, 61),
        ];
        for (r, g, b) in test_colors {
            let lab = rgb_to_oklab(r, g, b);
            let (r2, g2, b2) = oklab_to_rgb(&lab);
            assert!(
                (r as i16 - r2 as i16).abs() <= 1,
                "R roundtrip: {} -> {}",
                r,
                r2
            );
            assert!(
                (g as i16 - g2 as i16).abs() <= 1,
                "G roundtrip: {} -> {}",
                g,
                g2
            );
            assert!(
                (b as i16 - b2 as i16).abs() <= 1,
                "B roundtrip: {} -> {}",
                b,
                b2
            );
        }
    }

    #[test]
    fn perceptual_distance_ordering() {
        // Red and green should be far apart
        let red = rgb_to_oklab(255, 0, 0);
        let green = rgb_to_oklab(0, 255, 0);
        let dark_red = rgb_to_oklab(200, 0, 0);

        let rg_dist = delta_e(&red, &green);
        let rdr_dist = delta_e(&red, &dark_red);

        assert!(
            rg_dist > rdr_dist,
            "red-green ({:.3}) should be farther than red-dark_red ({:.3})",
            rg_dist,
            rdr_dist
        );
    }

    #[test]
    fn nearest_color_finds_closest() {
        let palette = vec![(255, 0, 0), (0, 255, 0), (0, 0, 255), (128, 128, 128)];
        assert_eq!(nearest_color(240, 10, 10, &palette), 0); // closest to red
        assert_eq!(nearest_color(10, 240, 10, &palette), 1); // closest to green
        assert_eq!(nearest_color(100, 100, 100, &palette), 3); // closest to grey
    }

    #[test]
    fn dungeon_palette_contrast() {
        // Verify the dungeon palette has good contrast in OKLab
        let shadow = rgb_to_oklab(0x12, 0x09, 0x1f); // s
        let bg = rgb_to_oklab(0x2a, 0x1f, 0x3d); // #
        let fg = rgb_to_oklab(0x5a, 0x48, 0x78); // +
        let hi = rgb_to_oklab(0x80, 0x70, 0xa8); // h
        let white = rgb_to_oklab(0xd8, 0xd0, 0xe8); // w

        // Each step should have meaningful lightness difference
        assert!(
            fg.l - bg.l > 0.05,
            "fg-bg contrast too low: {:.3}",
            fg.l - bg.l
        );
        assert!(
            hi.l - fg.l > 0.05,
            "hi-fg contrast too low: {:.3}",
            hi.l - fg.l
        );
        assert!(
            white.l - hi.l > 0.10,
            "w-hi contrast too low: {:.3}",
            white.l - hi.l
        );

        eprintln!("Dungeon palette OKLab lightness:");
        eprintln!(
            "  s={:.3} #={:.3} +={:.3} h={:.3} w={:.3}",
            shadow.l, bg.l, fg.l, hi.l, white.l
        );
    }
}
