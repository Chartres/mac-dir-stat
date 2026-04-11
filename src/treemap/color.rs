use std::time::SystemTime;

pub type Color = [u8; 4];
pub type GradientPair = (Color, Color);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Extension,
    Depth,
    Age,
}

const PALETTE: &[GradientPair] = &[
    ([124, 58, 237, 255], [167, 139, 250, 255]),   // Purple
    ([37, 99, 235, 255], [96, 165, 250, 255]),      // Blue
    ([8, 145, 178, 255], [103, 232, 249, 255]),     // Cyan
    ([220, 38, 38, 255], [248, 113, 113, 255]),     // Red
    ([234, 88, 12, 255], [251, 146, 60, 255]),      // Orange
    ([22, 163, 74, 255], [74, 222, 128, 255]),      // Green
    ([202, 138, 4, 255], [250, 204, 21, 255]),      // Yellow
    ([147, 51, 234, 255], [192, 132, 252, 255]),    // Magenta
    ([219, 39, 119, 255], [244, 114, 182, 255]),    // Pink
    ([13, 148, 136, 255], [94, 234, 212, 255]),     // Teal
    ([101, 163, 13, 255], [163, 230, 53, 255]),     // Lime
    ([79, 70, 229, 255], [129, 140, 248, 255]),     // Indigo
    ([217, 70, 239, 255], [232, 121, 249, 255]),    // Fuchsia
    ([245, 158, 11, 255], [252, 211, 77, 255]),     // Amber
    ([6, 182, 212, 255], [34, 211, 238, 255]),      // Sky
    ([16, 185, 129, 255], [52, 211, 153, 255]),     // Emerald
    ([239, 68, 68, 255], [252, 165, 165, 255]),     // Rose
    ([168, 85, 247, 255], [196, 181, 253, 255]),    // Violet
    ([14, 165, 233, 255], [125, 211, 252, 255]),    // Light Blue
    ([132, 204, 22, 255], [190, 242, 100, 255]),    // Yellow-Green
];

const SLATE_PAIR: GradientPair = ([71, 85, 105, 255], [148, 163, 184, 255]);

/// Dark, subtle color for free space — should not draw attention.
pub const FREE_SPACE_PAIR: GradientPair = ([24, 20, 40, 255], [30, 26, 50, 255]);

pub fn extension_color(ext: &str) -> GradientPair {
    if ext.is_empty() {
        return SLATE_PAIR;
    }
    if ext == "__free_space__" {
        return FREE_SPACE_PAIR;
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
