use crate::app::AppState;
use crate::treemap;
use crate::treemap::squarify::Rect as TRect;
use crate::ui::theme;
use egui::{Color32, Pos2, Rect, StrokeKind, Ui, Vec2};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    // Breadcrumb bar if zoomed
    if state.zoom_stack.len() > 1 {
        let mut breadcrumb_click: Option<(usize, usize)> = None;
        let zoom_snapshot: Vec<usize> = state.zoom_stack.clone();
        ui.horizontal(|ui| {
            if let Some(tree) = &state.tree {
                for (i, &node_id) in zoom_snapshot.iter().enumerate() {
                    if i > 0 {
                        ui.label(
                            egui::RichText::new(" \u{203A} ")
                                .color(theme::TEXT_MUTED)
                                .size(11.0),
                        );
                    }
                    let name = tree.node(node_id).name.to_string_lossy().to_string();
                    let label = ui.add(
                        egui::Label::new(
                            egui::RichText::new(&name)
                                .color(theme::ACCENT_LIGHT)
                                .size(11.0),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if label.clicked() {
                        breadcrumb_click = Some((i, node_id));
                    }
                }
            }
        });
        if let Some((i, node_id)) = breadcrumb_click {
            state.view_root = Some(node_id);
            state.zoom_stack.truncate(i + 1);
            state.treemap_dirty = true;
        }
    }

    let (response, painter) = ui.allocate_painter(
        ui.available_size(),
        egui::Sense::click_and_drag(),
    );
    let canvas_rect = response.rect;

    // Recompute if dirty
    if state.treemap_dirty && state.tree.is_some() {
        if let (Some(tree), Some(view_root)) = (&state.tree, state.view_root) {
            let bounds = TRect {
                x: canvas_rect.min.x as f64,
                y: canvas_rect.min.y as f64,
                w: canvas_rect.width() as f64,
                h: canvas_rect.height() as f64,
            };
            state.colored_rects =
                treemap::compute_treemap(tree, view_root, bounds, state.color_mode);
            state.treemap_dirty = false;
        }
    }

    painter.rect_filled(canvas_rect, 0.0, theme::BG_DARK);

    if state.colored_rects.is_empty() {
        if state.scan_progress.scanning {
            let center = canvas_rect.center();
            let time = ui.input(|i| i.time);
            let dots = match ((time * 2.0) as usize) % 4 {
                0 => "",
                1 => ".",
                2 => "..",
                _ => "...",
            };
            painter.text(
                Pos2::new(center.x, center.y - 20.0),
                egui::Align2::CENTER_CENTER,
                format!("Scanning{}", dots),
                egui::FontId::proportional(20.0),
                theme::ACCENT_LIGHT,
            );
            painter.text(
                Pos2::new(center.x, center.y + 10.0),
                egui::Align2::CENTER_CENTER,
                format!(
                    "{} files  \u{2022}  {} dirs  \u{2022}  {}",
                    state.scan_progress.files,
                    state.scan_progress.dirs,
                    theme::format_size(state.scan_progress.bytes)
                ),
                egui::FontId::proportional(14.0),
                theme::TEXT_SECONDARY,
            );
            // Animated bar
            let bar_width = 200.0;
            let bar_rect = Rect::from_min_size(
                Pos2::new(center.x - bar_width / 2.0, center.y + 35.0),
                Vec2::new(bar_width, 4.0),
            );
            painter.rect_filled(bar_rect, 2.0, theme::BAR_BG);
            let progress_x = ((time * 0.5).sin() * 0.5 + 0.5) as f32;
            let indicator = Rect::from_min_size(
                Pos2::new(
                    bar_rect.left() + (bar_width - 60.0) * progress_x,
                    bar_rect.top(),
                ),
                Vec2::new(60.0, 4.0),
            );
            painter.rect_filled(indicator, 2.0, theme::ACCENT);
            ui.ctx().request_repaint();
        }
        return;
    }

    let pointer_pos = response.hover_pos();
    state.hovered_node = None;

    for cr in &state.colored_rects {
        let rect = Rect::from_min_size(
            Pos2::new(cr.rect.x as f32, cr.rect.y as f32),
            Vec2::new(cr.rect.w as f32, cr.rect.h as f32),
        );
        if rect.width() < 0.5 || rect.height() < 0.5 {
            continue;
        }

        let is_hovered = pointer_pos.map_or(false, |p| rect.contains(p));
        let is_selected = state.selected_node == Some(cr.node_id);

        let is_ext_highlighted = if let Some(ref sel_ext) = state.selected_extension {
            if let Some(tree) = &state.tree {
                tree.node(cr.node_id).extension() == Some(sel_ext.as_str())
            } else {
                false
            }
        } else {
            true
        };

        let is_search_match = if state.search_active && !state.search_query.is_empty() {
            if let Some(tree) = &state.tree {
                let name = tree.node(cr.node_id).name.to_string_lossy();
                name.to_lowercase()
                    .contains(&state.search_query.to_lowercase())
            } else {
                true
            }
        } else {
            true
        };

        let alpha = if (!is_ext_highlighted && state.selected_extension.is_some())
            || !is_search_match
        {
            40
        } else {
            255
        };

        let c_start = Color32::from_rgba_unmultiplied(
            cr.color_start[0],
            cr.color_start[1],
            cr.color_start[2],
            alpha,
        );
        let c_end = Color32::from_rgba_unmultiplied(
            cr.color_end[0],
            cr.color_end[1],
            cr.color_end[2],
            alpha,
        );

        // Gradient mesh
        let mut mesh = egui::Mesh::default();
        let c_mid = lerp_color(&c_start, &c_end, 0.5);
        mesh.colored_vertex(rect.left_top(), c_start);
        mesh.colored_vertex(rect.right_top(), c_mid);
        mesh.colored_vertex(rect.right_bottom(), c_end);
        mesh.colored_vertex(rect.left_bottom(), c_mid);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(egui::Shape::mesh(mesh));

        // Light overlay for depth
        if rect.width() > 4.0 && rect.height() > 4.0 {
            let overlay_rect = Rect::from_min_size(
                Pos2::new(rect.right() - rect.width() * 0.4, rect.top()),
                Vec2::new(rect.width() * 0.4, rect.height() * 0.5),
            );
            painter.rect_filled(
                overlay_rect,
                0.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 8),
            );
        }

        if is_hovered {
            state.hovered_node = Some(cr.node_id);
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(2.0, Color32::WHITE),
                StrokeKind::Outside,
            );
            painter.rect_stroke(
                rect.expand(2.0),
                3.0,
                egui::Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(167, 139, 250, 100),
                ),
                StrokeKind::Outside,
            );
        }

        if is_selected {
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(2.0, theme::ACCENT_LIGHT),
                StrokeKind::Outside,
            );
        }

        // Labels
        if rect.width() > 40.0 && rect.height() > 16.0 {
            if let Some(tree) = &state.tree {
                let node = tree.node(cr.node_id);
                let name = node.name.to_string_lossy();
                let max_chars = (rect.width() / 7.0) as usize;
                let display_name = if name.len() > max_chars && max_chars > 3 {
                    format!("{}...", &name[..max_chars.min(name.len()).saturating_sub(3)])
                } else {
                    name.to_string()
                };
                painter.text(
                    Pos2::new(rect.left() + 3.0, rect.top() + 2.0),
                    egui::Align2::LEFT_TOP,
                    &display_name,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                );
                if rect.height() > 28.0 {
                    painter.text(
                        Pos2::new(rect.left() + 3.0, rect.top() + 14.0),
                        egui::Align2::LEFT_TOP,
                        theme::format_size(node.size),
                        egui::FontId::proportional(9.0),
                        Color32::from_rgba_unmultiplied(255, 255, 255, 150),
                    );
                }
            }
        }
    }

    // Click handling
    if response.clicked() {
        if let Some(hovered) = state.hovered_node {
            state.selected_node = Some(hovered);
            state.selected_extension = None;
            state.expand_to_node(hovered);
        }
    }
    if response.double_clicked() {
        if let Some(hovered) = state.hovered_node {
            if let Some(tree) = &state.tree {
                if let Some(pid) = tree.node(hovered).parent {
                    if tree.node(pid).is_dir() {
                        state.view_root = Some(pid);
                        state.zoom_stack.push(pid);
                        state.treemap_dirty = true;
                    }
                }
            }
        }
    }

    // Context menu
    response.context_menu(|ui| {
        if let Some(node_id) = state.hovered_node.or(state.selected_node) {
            crate::ui::context_menu::show(ui, state, node_id);
        }
    });
}

fn lerp_color(a: &Color32, b: &Color32, t: f32) -> Color32 {
    let lerp =
        |a: u8, b: u8| -> u8 { (a as f32 + (b as f32 - a as f32) * t).clamp(0.0, 255.0) as u8 };
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}
