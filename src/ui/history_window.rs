//! The same folder over its last scans, as small multiples (docs/SIGNATURE.md
//! Move 3B, after Tufte's *Envisioning Information*). Every panel uses one
//! block order, and a panel's height is proportional to the space used, so
//! a growing folder reads at a glance.

use crate::app::AppState;
use crate::history::{self, ScanSummary, OTHER};
use crate::treemap::color::extension_color;
use crate::treemap::squarify::{layout, LayoutItem, Rect as TRect};
use crate::ui::theme;
use egui::{Align2, Context, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

/// Tallest panel, in points.
const PANEL_H: f32 = 190.0;
const GAP: f32 = 16.0;

pub fn show(ctx: &Context, state: &mut AppState) {
    if !state.history_window_open {
        return;
    }
    let mut open = true;
    let scans: Vec<ScanSummary> = history::for_root(&state.history, &state.scan_root)
        .into_iter()
        .cloned()
        .collect();
    egui::Window::new("History")
        .open(&mut open)
        .default_width(920.0)
        .min_width(480.0)
        .resizable(true)
        .show(ctx, |ui| {
            let title = match scans.len() {
                0 | 1 => "The same folder, over time".to_string(),
                n => format!("The same folder, {} scans", n),
            };
            ui.label(RichText::new(title).color(theme::ink()).size(18.0));
            ui.label(
                RichText::new(format!(
                    "{} · same order in every panel; height is space used. \
                     Kept on this Mac only: totals and top folders, no file lists.",
                    state.scan_root.display()
                ))
                .family(theme::italic())
                .color(theme::note())
                .size(11.5),
            );
            ui.add_space(4.0);
            crate::ui::widgets::ink_rule(ui);
            ui.add_space(10.0);
            if scans.len() < 2 {
                ui.label(
                    RichText::new(if scans.is_empty() {
                        "No finished scan of this folder yet."
                    } else {
                        "One scan so far. Scan this folder again another day and the \
                         panels appear here, one per scan, up to six."
                    })
                    .color(theme::ink_soft())
                    .size(13.0),
                );
                if scans.len() == 1 {
                    ui.add_space(10.0);
                    multiples(ui, &scans);
                }
                return;
            }
            multiples(ui, &scans);
        });
    state.history_window_open = open;
}

fn multiples(ui: &mut egui::Ui, scans: &[ScanSummary]) {
    let refs: Vec<&ScanSummary> = scans.iter().collect();
    let order = history::layout_order(&refs);
    let max_used = scans.iter().map(|s| s.used).max().unwrap_or(1).max(1);
    let n = history::KEEP.max(scans.len()) as f32;
    let width = ((ui.available_width() - GAP * (n - 1.0)) / n).clamp(80.0, 180.0);
    let (area, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), PANEL_H + 40.0),
        Sense::hover(),
    );
    let painter = ui.painter_at(area.expand(2.0));
    let newest_year = scans.last().map(|s| ymd(s.when).0);
    let pointer = ui.ctx().pointer_hover_pos();
    let mut tip: Option<(String, u64)> = None;

    for (i, s) in scans.iter().enumerate() {
        let x = area.left() + i as f32 * (width + GAP);
        let h = (PANEL_H * s.used as f32 / max_used as f32).max(4.0);
        let panel = Rect::from_min_size(Pos2::new(x, area.top() + PANEL_H - h), Vec2::new(width, h));
        // Blocks in the shared order; names a scan lacks are simply absent.
        let items: Vec<LayoutItem> = order
            .iter()
            .enumerate()
            .filter_map(|(id, name)| {
                let size = s.top.iter().find(|(n, _)| n == name)?.1;
                (size > 0).then_some(LayoutItem { id, size: size as f64 })
            })
            .collect();
        let bounds = TRect {
            x: panel.left() as f64,
            y: panel.top() as f64,
            w: panel.width() as f64,
            h: panel.height() as f64,
        };
        for placed in layout(&items, &bounds) {
            let r = Rect::from_min_size(
                Pos2::new(placed.rect.x as f32, placed.rect.y as f32),
                Vec2::new(placed.rect.w as f32, placed.rect.h as f32),
            );
            let name = &order[placed.id];
            painter.rect_filled(r, 0.0, fill(name));
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, theme::map_gap(255)), StrokeKind::Inside);
            if r.width() > 46.0 && r.height() > 18.0 {
                let max = ((r.width() - 6.0) / 6.0) as usize;
                let label: String = if name.chars().count() > max {
                    name.chars().take(max.saturating_sub(1)).chain(['…']).collect()
                } else {
                    name.clone()
                };
                painter.text(
                    r.left_top() + Vec2::new(3.0, 2.0),
                    Align2::LEFT_TOP,
                    label,
                    FontId::proportional(10.0),
                    theme::on_map(200),
                );
            }
            if pointer.is_some_and(|p| r.contains(p)) {
                let size = s.top.iter().find(|(n, _)| n == name).map_or(0, |t| t.1);
                tip = Some((name.clone(), size));
                painter.rect_stroke(r, 0.0, Stroke::new(1.5, theme::highlight()), StrokeKind::Inside);
            }
        }
        let (y, m, d) = ymd(s.when);
        let date = if Some(y) == newest_year {
            format!("{d} {}", MONTHS[m as usize - 1])
        } else {
            format!("{d} {} {y}", MONTHS[m as usize - 1])
        };
        let base = area.top() + PANEL_H + 8.0;
        painter.text(
            Pos2::new(x, base),
            Align2::LEFT_TOP,
            date,
            FontId::proportional(12.0),
            theme::ink(),
        );
        painter.text(
            Pos2::new(x + width, base),
            Align2::RIGHT_TOP,
            theme::format_size(s.used),
            FontId::proportional(12.0),
            theme::note(),
        );
    }
    if let Some((name, size)) = tip {
        egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), egui::Id::new("history_tip"), |ui| {
            ui.label(RichText::new(name).color(theme::ink()));
            ui.label(RichText::new(theme::format_size(size)).color(theme::note()));
        });
    }
}

/// A folder keeps its tint in every panel; "other" is plain paper-grey.
fn fill(name: &str) -> egui::Color32 {
    if name == OTHER {
        return theme::bg_raised();
    }
    let (shade, light) = extension_color(name);
    let mid = [
        ((shade[0] as u16 + light[0] as u16) / 2) as u8,
        ((shade[1] as u16 + light[1] as u16) / 2) as u8,
        ((shade[2] as u16 + light[2] as u16) / 2) as u8,
        255,
    ];
    theme::map_fill(mid, 255)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Civil date (UTC) from seconds since 1970 (Howard Hinnant's algorithm).
pub fn ymd(secs: u64) -> (i64, u32, u32) {
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}
