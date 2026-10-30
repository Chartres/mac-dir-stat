//! The survey register: book typography on paper, ink, hairline rules and
//! one oxblood accent (docs/SIGNATURE.md §4). A dark variant inverts the same
//! ink-on-paper contrast and follows the macOS appearance setting.
//!
//! Colours are functions, not constants, because they depend on which
//! variant egui is drawing this frame. `App::update` calls [`sync`] first.

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Theme, Vec2,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct Palette {
    /// Text fields, windows and the map canvas.
    base: Color32,
    /// Panels: the page itself.
    panel: Color32,
    raised: Color32,
    hover: Color32,
    selection: Color32,
    rule: Color32,
    rule_faint: Color32,
    ink: Color32,
    ink_soft: Color32,
    note: Color32,
    faint: Color32,
    accent: Color32,
    accent_hover: Color32,
    /// Text set on the accent.
    on_accent: Color32,
}

const LIGHT: Palette = Palette {
    base: Color32::from_rgb(0xfb, 0xf8, 0xf2),
    panel: Color32::from_rgb(0xf6, 0xf2, 0xea),
    raised: Color32::from_rgb(0xed, 0xe7, 0xdb),
    hover: Color32::from_rgb(0xe7, 0xe0, 0xd2),
    selection: Color32::from_rgb(0xe6, 0xdc, 0xc9),
    rule: Color32::from_rgb(0xb9, 0xb2, 0xa5),
    rule_faint: Color32::from_rgb(0xdd, 0xd6, 0xc8),
    ink: Color32::from_rgb(0x1f, 0x1d, 0x1a),
    ink_soft: Color32::from_rgb(0x45, 0x41, 0x3b),
    note: Color32::from_rgb(0x6b, 0x65, 0x5c),
    faint: Color32::from_rgb(0x8f, 0x88, 0x7c),
    accent: Color32::from_rgb(0x7a, 0x2a, 0x20),
    accent_hover: Color32::from_rgb(0x8f, 0x34, 0x27),
    on_accent: Color32::from_rgb(0xf6, 0xf2, 0xea),
};

const DARK: Palette = Palette {
    base: Color32::from_rgb(0x16, 0x15, 0x12),
    panel: Color32::from_rgb(0x1c, 0x1b, 0x18),
    raised: Color32::from_rgb(0x26, 0x24, 0x1f),
    hover: Color32::from_rgb(0x2d, 0x2a, 0x24),
    selection: Color32::from_rgb(0x35, 0x31, 0x2a),
    rule: Color32::from_rgb(0x4d, 0x48, 0x3f),
    rule_faint: Color32::from_rgb(0x30, 0x2d, 0x27),
    ink: Color32::from_rgb(0xec, 0xe6, 0xda),
    ink_soft: Color32::from_rgb(0xcf, 0xc8, 0xba),
    note: Color32::from_rgb(0x9d, 0x96, 0x8a),
    faint: Color32::from_rgb(0x75, 0x70, 0x65),
    accent: Color32::from_rgb(0xa5, 0x48, 0x3a),
    accent_hover: Color32::from_rgb(0xb8, 0x5a, 0x4a),
    on_accent: Color32::from_rgb(0xf6, 0xf2, 0xea),
};

static DARK_MODE: AtomicBool = AtomicBool::new(false);

fn p() -> &'static Palette {
    if is_dark() {
        &DARK
    } else {
        &LIGHT
    }
}

pub fn is_dark() -> bool {
    DARK_MODE.load(Ordering::Relaxed)
}

/// Pick the palette egui is drawing with this frame (it follows the system
/// appearance unless told otherwise).
pub fn sync(ctx: &egui::Context) {
    DARK_MODE.store(ctx.style().visuals.dark_mode, Ordering::Relaxed);
}

pub fn bg_base() -> Color32 {
    p().base
}
pub fn bg_panel() -> Color32 {
    p().panel
}
pub fn bg_raised() -> Color32 {
    p().raised
}
pub fn bg_hover() -> Color32 {
    p().hover
}
pub fn bg_selection() -> Color32 {
    p().selection
}
pub fn rule() -> Color32 {
    p().rule
}
pub fn rule_faint() -> Color32 {
    p().rule_faint
}
pub fn ink() -> Color32 {
    p().ink
}
pub fn ink_soft() -> Color32 {
    p().ink_soft
}
pub fn note() -> Color32 {
    p().note
}
pub fn faint() -> Color32 {
    p().faint
}
/// Oxblood: at most one action per window.
pub fn accent() -> Color32 {
    p().accent
}
pub fn accent_hover() -> Color32 {
    p().accent_hover
}
pub fn on_accent() -> Color32 {
    p().on_accent
}
/// Selection rings, bars, status text: plain ink.
pub fn highlight() -> Color32 {
    p().ink
}
/// Destructive confirmations use the accent; they are the window's one action.
pub fn danger() -> Color32 {
    p().accent
}
pub fn danger_hover() -> Color32 {
    p().accent_hover
}
/// Error and warning text in the status bar and menus.
pub fn warn_text() -> Color32 {
    if is_dark() {
        Color32::from_rgb(0xd9, 0x8a, 0x78)
    } else {
        p().accent
    }
}
pub fn bar_bg() -> Color32 {
    p().raised
}

/// Map fills are tinted plates on paper; in the dark variant they are dimmed
/// so the page stays dark.
pub fn map_fill(c: [u8; 4], alpha: u8) -> Color32 {
    use crate::treemap::color::FREE_SPACE_PAIR;
    // Free space is bare paper in either variant, not the brightest block.
    if is_dark() && (c == FREE_SPACE_PAIR.0 || c == FREE_SPACE_PAIR.1) {
        let r = p().raised;
        return Color32::from_rgba_unmultiplied(r.r(), r.g(), r.b(), alpha);
    }
    let k = if is_dark() { 0.62 } else { 1.0 };
    let f = |v: u8| (v as f32 * k) as u8;
    Color32::from_rgba_unmultiplied(f(c[0]), f(c[1]), f(c[2]), alpha)
}

/// Names and sizes set inside map blocks.
pub fn on_map(alpha: u8) -> Color32 {
    let c = if is_dark() { DARK.ink } else { LIGHT.ink };
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// Hairline between map regions: the paper showing through.
pub fn map_gap(alpha: u8) -> Color32 {
    let c = p().panel;
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

pub const RADIUS_SM: u8 = 2;
pub const RADIUS_MD: u8 = 3;
pub const RADIUS_LG: u8 = 4;

/// Italic face for notes and explanations.
pub fn italic() -> FontFamily {
    FontFamily::Name("italic".into())
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "serif".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/SourceSerif4SmText-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "serif-italic".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/SourceSerif4SmText-It.ttf"
        ))),
    );
    // egui's own fonts stay as fallbacks for arrows, symbols and emoji.
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "serif".into());
    let mut italic_faces = vec!["serif-italic".to_string()];
    italic_faces.extend(fallback);
    fonts.families.insert(italic(), italic_faces);
    fonts
}

pub fn apply_theme(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_style_of(Theme::Light, style(&LIGHT, false));
    ctx.set_style_of(Theme::Dark, style(&DARK, true));
    sync(ctx);
}

fn style(c: &Palette, dark: bool) -> egui::Style {
    let mut style = egui::Style::default();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
    ]
    .into_iter()
    .collect();

    let spacing = &mut style.spacing;
    spacing.item_spacing = Vec2::new(8.0, 6.0);
    spacing.button_padding = Vec2::new(12.0, 5.0);
    spacing.interact_size = Vec2::new(28.0, 26.0);
    spacing.window_margin = Margin::same(14);
    spacing.menu_margin = Margin::same(6);
    spacing.icon_width = 13.0;
    spacing.icon_spacing = 6.0;
    spacing.scroll.bar_width = 8.0;
    spacing.scroll.bar_inner_margin = 2.0;
    spacing.scroll.bar_outer_margin = 0.0;

    style.animation_time = 0.12;

    let v = &mut style.visuals;
    *v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.override_text_color = Some(c.ink);
    v.panel_fill = c.panel;
    v.window_fill = c.base;
    v.window_stroke = Stroke::new(1.0, c.rule);
    v.window_corner_radius = CornerRadius::same(RADIUS_LG);
    v.window_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 90 } else { 28 }),
    };
    v.popup_shadow = v.window_shadow;
    v.menu_corner_radius = CornerRadius::same(RADIUS_MD);
    v.extreme_bg_color = c.base;
    v.faint_bg_color = c.panel;
    v.code_bg_color = c.raised;

    let r = CornerRadius::same(RADIUS_MD);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = c.panel;
    w.noninteractive.weak_bg_fill = c.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, c.rule_faint);
    w.noninteractive.fg_stroke = Stroke::new(1.0, c.ink);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = c.raised;
    w.inactive.weak_bg_fill = c.raised;
    w.inactive.bg_stroke = Stroke::new(1.0, c.rule);
    w.inactive.fg_stroke = Stroke::new(1.0, c.ink_soft);
    w.inactive.corner_radius = r;
    w.inactive.expansion = 0.0;

    w.hovered.bg_fill = c.hover;
    w.hovered.weak_bg_fill = c.hover;
    w.hovered.bg_stroke = Stroke::new(1.0, c.rule);
    w.hovered.fg_stroke = Stroke::new(1.0, c.ink);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = c.raised;
    w.active.weak_bg_fill = c.raised;
    w.active.bg_stroke = Stroke::new(1.0, c.ink);
    w.active.fg_stroke = Stroke::new(1.0, c.ink);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;

    w.open.bg_fill = c.raised;
    w.open.weak_bg_fill = c.raised;
    w.open.bg_stroke = Stroke::new(1.0, c.rule);
    w.open.fg_stroke = Stroke::new(1.0, c.ink);
    w.open.corner_radius = r;

    v.selection.bg_fill = Color32::from_rgba_unmultiplied(
        c.accent.r(),
        c.accent.g(),
        c.accent.b(),
        0x38,
    );
    v.selection.stroke = Stroke::new(1.0, c.ink);
    v.hyperlink_color = c.ink;

    style
}

/// The highlight colour with a custom alpha — for selection rings,
/// dir-region borders, and other low-opacity overlays.
pub fn highlight_alpha(alpha: u8) -> Color32 {
    let c = highlight();
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// "today", "3 days ago", "5 months ago", "2 years ago".
pub fn format_age(t: std::time::SystemTime) -> String {
    let days = std::time::SystemTime::now()
        .duration_since(t)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0);
    let plural = |n: u64, unit: &str| {
        if n == 1 {
            format!("1 {unit} ago")
        } else {
            format!("{n} {unit}s ago")
        }
    };
    match days {
        0 => "today".into(),
        1..=59 => plural(days, "day"),
        60..=729 => plural(days / 30, "month"),
        _ => plural(days / 365, "year"),
    }
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;
    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_variants_are_ink_on_paper_with_an_oxblood_accent() {
        let light = style(&LIGHT, false).visuals;
        assert!(!light.dark_mode);
        assert_eq!(light.panel_fill, Color32::from_rgb(0xf6, 0xf2, 0xea));
        assert_eq!(light.override_text_color, Some(Color32::from_rgb(0x1f, 0x1d, 0x1a)));
        assert_eq!(LIGHT.accent, Color32::from_rgb(0x7a, 0x2a, 0x20));
        let dark = style(&DARK, true).visuals;
        assert!(dark.dark_mode);
        // Inverted: the dark page is the light ink, and vice versa.
        assert!(dark.panel_fill.r() < 0x30 && DARK.ink.r() > 0xe0);
    }

    #[test]
    fn apply_theme_registers_the_serif_and_italic_faces() {
        let f = fonts();
        assert_eq!(f.families[&FontFamily::Proportional][0], "serif");
        assert_eq!(f.families[&italic()][0], "serif-italic");
    }
}
