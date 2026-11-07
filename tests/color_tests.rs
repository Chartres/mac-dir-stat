use mac_dir_stat::treemap::color::{extension_color, depth_color, age_color, GradientPair};
use std::time::{SystemTime, Duration};

#[test]
fn test_extension_color_deterministic() {
    let c1 = extension_color("rs");
    let c2 = extension_color("rs");
    assert_eq!(c1.0, c2.0);
    assert_eq!(c1.1, c2.1);
}

#[test]
fn test_extension_color_different_extensions() {
    let rs = extension_color("rs");
    let txt = extension_color("txt");
    assert_ne!(rs.0, txt.0);
}

#[test]
fn test_extension_color_unknown() {
    // No-extension files use the slate gradient (SLATE_PAIR in color.rs).
    let c = extension_color("");
    assert_eq!(c.0, [134, 138, 142, 255]);
    assert_eq!(c.1, [170, 174, 178, 255]);
}

#[test]
fn test_depth_color_varies_by_depth() {
    let c0 = depth_color(0, 0);
    let c1 = depth_color(0, 1);
    let c2 = depth_color(0, 2);
    assert_ne!(c0, c1);
    assert_ne!(c1, c2);
}

#[test]
fn test_depth_color_varies_by_hue_index() {
    let c0 = depth_color(0, 1);
    let c1 = depth_color(1, 1);
    assert_ne!(c0, c1);
}

#[test]
fn test_age_color_recent_is_warm() {
    let now = SystemTime::now();
    let c = age_color(now);
    assert!(c.0[0] > 150, "Recent file should be warm-toned, got r={}", c.0[0]);
}

#[test]
fn test_age_color_old_is_cool() {
    let old = SystemTime::now() - Duration::from_secs(365 * 24 * 3600 * 2);
    let c = age_color(old);
    assert!(c.0[2] > c.0[0], "Old file should be cool-toned");
}

#[test]
fn test_palette_has_enough_colors() {
    let exts = ["rs", "txt", "mov", "mp4", "dmg", "zip", "jpg", "png", "app", "pdf"];
    let colors: Vec<GradientPair> = exts.iter().map(|e| extension_color(e)).collect();
    let unique: std::collections::HashSet<[u8; 8]> = colors
        .iter()
        .map(|c| {
            let mut key = [0u8; 8];
            key[..4].copy_from_slice(&c.0);
            key[4..].copy_from_slice(&c.1);
            key
        })
        .collect();
    assert!(unique.len() >= 6, "Expected at least 6 unique colors, got {}", unique.len());
}

#[test]
fn age_scale_is_monotonic_and_ends_at_three_years() {
    use mac_dir_stat::treemap::color::{age_t, AGE_SPAN_DAYS, AGE_TICKS};
    assert_eq!(age_t(0), 0.0);
    assert_eq!(age_t(AGE_SPAN_DAYS), 1.0);
    assert_eq!(age_t(AGE_SPAN_DAYS * 10), 1.0);
    let ts: Vec<f64> = AGE_TICKS.iter().map(|t| age_t(t.1)).collect();
    assert!(ts.windows(2).all(|w| w[0] < w[1]), "ticks out of order: {ts:?}");
}

#[test]
fn a_file_of_a_ticks_age_gets_the_legends_colour_at_that_tick() {
    use mac_dir_stat::treemap::color::{age_color_at, age_t, AGE_TICKS};
    use std::time::{Duration, SystemTime};
    for (_, days) in AGE_TICKS {
        // an hour inside the day, so clock drift between the calls cannot cross it
        let modified = SystemTime::now() - Duration::from_secs(days * 86_400 + 3600);
        assert_eq!(age_color(modified), age_color_at(age_t(days)), "{days} d");
    }
}

#[test]
fn legend_ends_are_the_map_colours_at_the_labelled_ends() {
    use mac_dir_stat::treemap::color::{legend_color, ColorMode, DEPTH_SPAN};
    let mid = |p: GradientPair| [((p.0[0] as u16 + p.1[0] as u16) / 2) as u8, ((p.0[1] as u16 + p.1[1] as u16) / 2) as u8, ((p.0[2] as u16 + p.1[2] as u16) / 2) as u8, 255];
    assert_eq!(legend_color(ColorMode::Depth, 0.0), Some(mid(depth_color(0, 1))));
    assert_eq!(legend_color(ColorMode::Depth, 1.0), Some(mid(depth_color(0, DEPTH_SPAN))));
    assert_eq!(legend_color(ColorMode::Age, 0.0), Some(mid(age_color(std::time::SystemTime::now()))));
    assert_eq!(legend_color(ColorMode::Extension, 0.5), None);
    // depth keeps darkening up to the last level, then stops
    assert_eq!(depth_color(0, DEPTH_SPAN + 1), depth_color(0, DEPTH_SPAN + 3));
    assert_ne!(depth_color(0, DEPTH_SPAN - 1), depth_color(0, DEPTH_SPAN));
}
