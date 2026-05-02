use std::time::SystemTime;

pub type Color = [u8; 4];
pub type GradientPair = (Color, Color);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Extension,
    Depth,
    Age,
}

// Curated harmonious palette: even hue spread, ~60-70% lightness,
// ~50% saturation. No neon, no clashing pairs.
const PALETTE: &[GradientPair] = &[
    ([76, 134, 196, 255],  [128, 176, 224, 255]),  // Sky blue
    ([60, 162, 162, 255],  [120, 200, 200, 255]),  // Teal
    ([84, 174, 132, 255],  [136, 208, 176, 255]),  // Mint
    ([148, 184, 96, 255],  [188, 216, 144, 255]),  // Lime
    ([212, 168, 80, 255],  [240, 200, 128, 255]),  // Amber
    ([218, 138, 84, 255],  [240, 176, 132, 255]),  // Tangerine
    ([212, 110, 110, 255], [240, 152, 152, 255]),  // Coral
    ([212, 116, 168, 255], [240, 156, 196, 255]),  // Pink
    ([170, 132, 212, 255], [204, 172, 232, 255]),  // Lavender
    ([116, 132, 208, 255], [156, 172, 232, 255]),  // Periwinkle
];

// Used for files with no extension — neutral cool gray.
const SLATE_PAIR: GradientPair = ([88, 102, 122, 255], [136, 148, 168, 255]);

/// Dark, subtle color for free space — should not draw attention.
/// Tuned to sit just above the new off-black background (no purple cast).
pub const FREE_SPACE_PAIR: GradientPair = ([20, 22, 28, 255], [28, 30, 36, 255]);

/// Skipped / TCC-blocked space — distinct from free; warm muted tone so
/// the user notices "there's something here we couldn't see."
pub const SKIPPED_PAIR: GradientPair = ([60, 50, 38, 255], [88, 70, 50, 255]);

pub fn extension_color(ext: &str) -> GradientPair {
    if ext.is_empty() {
        return SLATE_PAIR;
    }
    if ext == "__free_space__" {
        return FREE_SPACE_PAIR;
    }
    if ext == "__skipped__" {
        return SKIPPED_PAIR;
    }
    let hash = simple_hash(ext);
    PALETTE[hash % PALETTE.len()]
}

pub fn depth_color(hue_index: usize, depth: u16) -> GradientPair {
    let base = &PALETTE[hue_index % PALETTE.len()];
    let factor = 1.0 - (depth as f32 * 0.12).min(0.6);
    let lighten = |c: Color| -> Color {
        [
            (c[0] as f32 * factor) as u8,
            (c[1] as f32 * factor) as u8,
            (c[2] as f32 * factor) as u8,
            c[3],
        ]
    };
    (lighten(base.0), lighten(base.1))
}

pub fn age_color(modified: SystemTime) -> GradientPair {
    let age_secs = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default()
        .as_secs();
    let one_year = 365 * 24 * 3600;
    let t = (age_secs as f64 / one_year as f64).min(1.0);
    let warm_start: [f64; 3] = [219.0, 39.0, 119.0];
    let warm_end: [f64; 3] = [244.0, 114.0, 182.0];
    let cool_start: [f64; 3] = [49.0, 46.0, 129.0];
    let cool_end: [f64; 3] = [79.0, 70.0, 229.0];
    let lerp = |a: f64, b: f64| -> u8 { (a + (b - a) * t).clamp(0.0, 255.0) as u8 };
    let start = [
        lerp(warm_start[0], cool_start[0]),
        lerp(warm_start[1], cool_start[1]),
        lerp(warm_start[2], cool_start[2]),
        255,
    ];
    let end = [
        lerp(warm_end[0], cool_end[0]),
        lerp(warm_end[1], cool_end[1]),
        lerp(warm_end[2], cool_end[2]),
        255,
    ];
    (start, end)
}

fn simple_hash(s: &str) -> usize {
    let mut h: usize = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as usize);
    }
    h
}
