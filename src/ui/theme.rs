use egui::Color32;

pub const BG_DARK: Color32 = Color32::from_rgb(14, 12, 24);
pub const BG_PANEL: Color32 = Color32::from_rgb(22, 18, 37);
pub const BG_SELECTION: Color32 = Color32::from_rgb(26, 23, 48);
pub const BG_HOVER: Color32 = Color32::from_rgb(30, 26, 55);
pub const BORDER: Color32 = Color32::from_rgb(42, 42, 74);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(224, 224, 240);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(139, 139, 187);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(102, 102, 102);
pub const ACCENT: Color32 = Color32::from_rgb(124, 58, 237);
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(167, 139, 250);
pub const SECTION_HEADER: Color32 = Color32::from_rgb(107, 91, 149);
pub const BAR_BG: Color32 = Color32::from_rgb(26, 23, 48);
pub const BUTTON_BG: Color32 = Color32::from_rgb(42, 42, 74);
pub const BUTTON_ACTIVE_BORDER: Color32 = Color32::from_rgb(124, 58, 237);

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let visuals = &mut style.visuals;
    visuals.dark_mode = true;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_DARK;
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.fg_stroke.color = TEXT_PRIMARY;
    visuals.widgets.inactive.bg_fill = BUTTON_BG;
    visuals.widgets.inactive.fg_stroke.color = TEXT_SECONDARY;
    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.fg_stroke.color = TEXT_PRIMARY;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    visuals.selection.bg_fill = BG_SELECTION;
    visuals.selection.stroke.color = ACCENT_LIGHT;
    visuals.extreme_bg_color = BG_DARK;
    ctx.set_style(style);
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
