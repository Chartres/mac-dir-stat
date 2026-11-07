use std::time::SystemTime;

pub type Color = [u8; 4];
pub type GradientPair = (Color, Color);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Extension,
    Depth,
    Age,
}

// Tinted plates on paper (docs/SIGNATURE.md §4): muted earth and slate,
// each a (shade, light) pair for the cushion gradient. No neon.
const PALETTE: &[GradientPair] = &[
    ([184, 174, 150, 255], [212, 204, 184, 255]), // Stone
    ([150, 164, 146, 255], [184, 196, 178, 255]), // Sage
    ([168, 146, 120, 255], [200, 182, 158, 255]), // Umber
    ([138, 150, 164, 255], [176, 186, 198, 255]), // Slate blue
    ([194, 184, 160, 255], [222, 214, 196, 255]), // Vellum
    ([162, 150, 140, 255], [194, 184, 176, 255]), // Taupe
    ([176, 164, 132, 255], [208, 198, 168, 255]), // Ochre
    ([152, 162, 140, 255], [186, 194, 174, 255]), // Lichen
    ([170, 140, 132, 255], [202, 176, 168, 255]), // Madder
    ([144, 156, 150, 255], [180, 190, 184, 255]), // Verdigris
];

// Used for files with no extension — neutral grey slate.
const SLATE_PAIR: GradientPair = ([134, 138, 142, 255], [170, 174, 178, 255]);

/// Free space: the paper itself, barely tinted — it should not draw attention.
pub const FREE_SPACE_PAIR: GradientPair = ([232, 226, 214, 255], [240, 235, 225, 255]);

/// Skipped / TCC-blocked space — distinct from free; a darker warm wash so
/// the user notices "there's something here we couldn't see."
pub const SKIPPED_PAIR: GradientPair = ([150, 128, 104, 255], [176, 156, 132, 255]);

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
    depth_color_f(hue_index, depth as f32)
}

fn depth_color_f(hue_index: usize, depth: f32) -> GradientPair {
    let base = &PALETTE[hue_index % PALETTE.len()];
    let factor = 1.0 - (depth * 0.1).min(0.5);
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

/// Age at which the age scale ends: this old or older is the cool end.
pub const AGE_SPAN_DAYS: u64 = 3 * 365;
/// Labelled points on the age legend, in days.
pub const AGE_TICKS: [(&str, u64); 4] = [("today", 0), ("1 mo", 30), ("1 y", 365), ("3 y+", AGE_SPAN_DAYS)];
/// Depth at which the depth scale stops getting darker.
pub const DEPTH_SPAN: u16 = 5;

/// Position on the age scale, 0 (today) to 1 (three years or older).
/// Logarithmic: last week and last year both get room.
pub fn age_t(age_days: u64) -> f64 {
    (((1 + age_days) as f64).ln() / ((1 + AGE_SPAN_DAYS) as f64).ln()).min(1.0)
}

/// Colour at position `t` of the age scale: madder and rust when new, slate
/// when old.
pub fn age_color_at(t: f64) -> GradientPair {
    let warm_start: [f64; 3] = [176.0, 104.0, 80.0];
    let warm_end: [f64; 3] = [208.0, 146.0, 120.0];
    let cool_start: [f64; 3] = [116.0, 130.0, 150.0];
    let cool_end: [f64; 3] = [158.0, 170.0, 186.0];
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

pub fn age_color(modified: SystemTime) -> GradientPair {
    let age_secs = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default()
        .as_secs();
    age_color_at(age_t(age_secs / 86_400))
}

/// The colour a legend bar shows at position `t` (0..1) of the active mode:
/// the middle of the block gradient, from the same functions the map uses.
/// Depth runs from level 1 to `DEPTH_SPAN`, age from today to `AGE_SPAN_DAYS`.
pub fn legend_color(mode: ColorMode, t: f32) -> Option<Color> {
    let pair = match mode {
        ColorMode::Extension => return None,
        ColorMode::Depth => depth_color_f(0, 1.0 + t * (DEPTH_SPAN - 1) as f32),
        ColorMode::Age => age_color_at(t as f64),
    };
    let mid = |a: u8, b: u8| ((a as u16 + b as u16) / 2) as u8;
    Some([mid(pair.0[0], pair.1[0]), mid(pair.0[1], pair.1[1]), mid(pair.0[2], pair.1[2]), 255])
}

fn simple_hash(s: &str) -> usize {
    let mut h: usize = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as usize);
    }
    h
}
