use mac_dir_stat::treemap::color::{extension_color, depth_color, age_color, ColorMode, GradientPair};
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
    let c = extension_color("");
    assert_eq!(c.0, [71, 85, 105, 255]);
    assert_eq!(c.1, [148, 163, 184, 255]);
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
