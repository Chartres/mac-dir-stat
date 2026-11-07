//! The path bar under the toolbar: Back, Up, the folder path as clickable
//! segments, and the legend of the active colour mode.

use crate::app::AppState;
use crate::scanner::tree::NodeId;
use crate::treemap::color::{self, ColorMode, AGE_TICKS, DEPTH_SPAN};
use crate::ui::{theme, widgets};
use egui::{CursorIcon, Pos2, Rect, RichText, Sense, Ui, Vec2};

/// Path segments shown; a longer path keeps the first and the last ones.
const MAX_SEGMENTS: usize = 6;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let Some(tree) = &state.tree else { return };
    let Some(view) = state.view_root.filter(|&v| tree.is_alive(v)) else { return };
    let chain = crate::nav::chain(tree, view);
    let names: Vec<String> = chain.iter().map(|&id| tree.name(id).to_string()).collect();
    let can_up = crate::nav::parent_of_root(state).is_some();
    let can_back = !state.nav_back.is_empty();
    let mut go: Option<NodeId> = None;
    let (mut up, mut back) = (false, false);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let btn = |ui: &mut Ui, on: bool, label: &str, tip: &str| {
            ui.add_enabled_ui(on, |ui| widgets::ghost_button(ui, label).on_hover_text(tip))
                .inner
                .clicked()
        };
        back = btn(ui, can_back, "←", "Back  ⌘[");
        up = btn(ui, can_up, "↑", "Enclosing folder  ⌘↑");
        ui.add_space(6.0);

        let hidden = chain.len().saturating_sub(MAX_SEGMENTS);
        for (i, (&id, name)) in chain.iter().zip(&names).enumerate() {
            if hidden > 0 && i > 0 && i <= hidden {
                if i == 1 {
                    sep(ui);
                    ui.label(RichText::new("…").color(theme::note()))
                        .on_hover_text(names[1..=hidden].join(" › "));
                }
                continue;
            }
            if i > 0 {
                sep(ui);
            }
            let last = i + 1 == chain.len();
            if last {
                ui.label(RichText::new(name).strong().color(theme::ink()).size(13.0));
            } else {
                let b = egui::Button::new(RichText::new(name).color(theme::ink_soft()).size(13.0))
                    .frame(false);
                let r = ui.add(b).on_hover_text(format!("Show {name}"));
                if r.clicked() {
                    go = Some(id);
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            legend(ui, state.color_mode);
        });
    });

    if let Some(id) = go {
        crate::nav::navigate_to(state, id);
    } else if up {
        crate::nav::go_up(state);
    } else if back {
        crate::nav::go_back(state);
    }
}

fn sep(ui: &mut Ui) {
    ui.label(RichText::new("›").color(theme::faint()).size(13.0));
}

/// What the colours mean: a gradient bar with its labelled ends and ticks.
/// Nothing for Extension, where the key is the file-type list.
pub fn legend(ui: &mut Ui, mode: ColorMode) {
    let (lo, hi, ticks): (&str, &str, Vec<(f32, String)>) = match mode {
        ColorMode::Extension => return,
        ColorMode::Depth => (
            "shallow",
            "deep",
            (1..=DEPTH_SPAN)
                .map(|d| {
                    let t = (d - 1) as f32 / (DEPTH_SPAN - 1) as f32;
                    (t, if d == DEPTH_SPAN { format!("{d}+") } else { d.to_string() })
                })
                .collect(),
        ),
        ColorMode::Age => (
            "new",
            "old",
            AGE_TICKS
                .iter()
                .map(|&(l, days)| (color::age_t(days) as f32, l.to_string()))
                .collect(),
        ),
    };
    let tip = match mode {
        ColorMode::Depth => "Folder level, counted from the folder shown. Blocks get darker with each level.",
        _ => "Last modified, on a log scale. Hover a block for its date.",
    };
    let bar = Vec2::new(200.0, 9.0);
    // Laid out right to left: the old end first.
    ui.label(RichText::new(hi).color(theme::note()).size(11.0));
    let resp = ui
        .allocate_ui_with_layout(
            Vec2::new(bar.x, bar.y + 14.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            let (rect, resp) = ui.allocate_exact_size(bar, Sense::hover());
            paint_bar(ui, rect, mode);
            let (lab, _) = ui.allocate_exact_size(Vec2::new(bar.x, 11.0), Sense::hover());
            for (t, text) in &ticks {
                let x = rect.left() + t * bar.x;
                ui.painter().vline(x, rect.bottom()..=rect.bottom() + 2.0, egui::Stroke::new(1.0, theme::faint()));
                let align = if *t <= 0.0 {
                    egui::Align2::LEFT_TOP
                } else if *t >= 1.0 {
                    egui::Align2::RIGHT_TOP
                } else {
                    egui::Align2::CENTER_TOP
                };
                ui.painter().text(
                    Pos2::new(x, lab.top()),
                    align,
                    text,
                    egui::FontId::proportional(9.0),
                    theme::note(),
                );
            }
            resp
        })
        .inner;
    ui.label(RichText::new(lo).color(theme::note()).size(11.0));
    resp.on_hover_text(tip).on_hover_cursor(CursorIcon::Help);
}

fn paint_bar(ui: &Ui, rect: Rect, mode: ColorMode) {
    const N: usize = 30;
    let mut mesh = egui::Mesh::default();
    for i in 0..=N {
        let t = i as f32 / N as f32;
        let Some(c) = color::legend_color(mode, t) else { return };
        let c = theme::map_fill(c, 255);
        let x = rect.left() + t * rect.width();
        mesh.colored_vertex(Pos2::new(x, rect.top()), c);
        mesh.colored_vertex(Pos2::new(x, rect.bottom()), c);
        if i > 0 {
            let b = (i * 2) as u32;
            mesh.add_triangle(b - 2, b - 1, b);
            mesh.add_triangle(b - 1, b, b + 1);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, theme::rule()), egui::StrokeKind::Inside);
}
