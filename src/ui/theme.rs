use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Vec2};

// Surfaces — warm off-black, three elevation steps
pub const BG_DARK: Color32 = Color32::from_rgb(0x0c, 0x0d, 0x10);
pub const BG_PANEL: Color32 = Color32::from_rgb(0x13, 0x14, 0x19);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x1c, 0x1d, 0x24);
pub const BG_HOVER: Color32 = Color32::from_rgb(0x22, 0x24, 0x2c);
pub const BG_SELECTION: Color32 = Color32::from_rgb(0x26, 0x2a, 0x38);

// Borders — visible but soft
pub const BORDER: Color32 = Color32::from_rgb(0x29, 0x2b, 0x33);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x1d, 0x1f, 0x26);

// Text
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe7, 0xe7, 0xea);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xa1, 0xa4, 0xad);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x6b, 0x6f, 0x78);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x4d, 0x50, 0x59);

// Accent — reserved for the primary CTA only.
pub const ACCENT: Color32 = Color32::from_rgb(0x8b, 0x5c, 0xf6);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x9d, 0x74, 0xf8);

// Highlight — used on selected/hovered items, status text, breadcrumbs.
// Cool near-white, no violet cast.
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0xd6, 0xda, 0xe2);

// Mini-bar fill (dir-tree size bars, scan progress indicator)
pub const BAR_FILL: Color32 = Color32::from_rgb(0x6b, 0x7c, 0x96);

// Danger
pub const DANGER: Color32 = Color32::from_rgb(0xef, 0x44, 0x44);
pub const DANGER_HOVER: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71);

// Mini-bar background (used in dir_tree rows and scan progress bar)
pub const BAR_BG: Color32 = BG_ELEVATED;

pub const RADIUS_SM: u8 = 4;
pub const RADIUS_MD: u8 = 6;
pub const RADIUS_LG: u8 = 8;

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Typography hierarchy
    style.text_styles = [
        (TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
    ]
    .into_iter()
    .collect();

    // Spacing — generous, breathable
    let spacing = &mut style.spacing;
    spacing.item_spacing = Vec2::new(8.0, 6.0);
    spacing.button_padding = Vec2::new(12.0, 6.0);
    spacing.interact_size = Vec2::new(28.0, 28.0);
    spacing.window_margin = Margin::same(12);
    spacing.menu_margin = Margin::same(6);
    spacing.icon_width = 14.0;
    spacing.icon_spacing = 6.0;
    spacing.scroll.bar_width = 8.0;
    spacing.scroll.bar_inner_margin = 2.0;
    spacing.scroll.bar_outer_margin = 0.0;

    // Animation
    style.animation_time = 0.12;

    // Visuals
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.override_text_color = Some(TEXT_PRIMARY);
    v.panel_fill = BG_PANEL;
    v.window_fill = BG_DARK;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = CornerRadius::same(RADIUS_LG);
    v.menu_corner_radius = CornerRadius::same(RADIUS_MD);
    v.extreme_bg_color = BG_DARK;
    v.faint_bg_color = BG_PANEL;
    v.code_bg_color = BG_ELEVATED;

    let r = CornerRadius::same(RADIUS_MD);

    // Noninteractive (labels, frames)
    v.widgets.noninteractive.bg_fill = BG_PANEL;
    v.widgets.noninteractive.weak_bg_fill = BG_PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.noninteractive.corner_radius = r;

    // Inactive (default button rest state)
    v.widgets.inactive.bg_fill = BG_ELEVATED;
    v.widgets.inactive.weak_bg_fill = BG_ELEVATED;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    v.widgets.inactive.corner_radius = r;
    v.widgets.inactive.expansion = 0.0;

    // Hovered
    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.hovered.weak_bg_fill = BG_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.hovered.corner_radius = r;
    v.widgets.hovered.expansion = 0.0;

    // Active (pressed)
    v.widgets.active.bg_fill = BG_ELEVATED;
    v.widgets.active.weak_bg_fill = BG_ELEVATED;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.active.corner_radius = r;
    v.widgets.active.expansion = 0.0;

    // Open (e.g., expanded combobox / context menu trigger)
    v.widgets.open.bg_fill = BG_ELEVATED;
    v.widgets.open.weak_bg_fill = BG_ELEVATED;
    v.widgets.open.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.open.corner_radius = r;

    // Text selection
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0x8b, 0x5c, 0xf6, 0x40);
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    v.hyperlink_color = ACCENT_LIGHT;

    ctx.set_style(style);
}

/// Returns ACCENT_LIGHT with a custom alpha — for selection rings, dir-region
/// borders, and other low-opacity highlight overlays.
pub fn highlight_alpha(alpha: u8) -> Color32 {
    let c = ACCENT_LIGHT;
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
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
