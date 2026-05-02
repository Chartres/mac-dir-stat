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
                    let name = tree.name(node_id).to_string();
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

    let (response, painter) =
        ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
    let canvas_rect = response.rect;

    // React to side-panel resizes: when the central canvas changes size,
    // recompute the treemap layout (caching is invalidated).
    let canvas_size = canvas_rect.size();
    if (canvas_size - state.last_canvas_size).length() > 0.5 {
        state.treemap_dirty = true;
        state.last_canvas_size = canvas_size;
    }

    // Recompute if dirty
    if state.treemap_dirty && state.tree.is_some() {
        if let (Some(tree), Some(view_root)) = (&state.tree, state.view_root) {
            let bounds = TRect {
                x: canvas_rect.min.x as f64,
                y: canvas_rect.min.y as f64,
                w: canvas_rect.width() as f64,
                h: canvas_rect.height() as f64,
            };
            let layout = treemap::compute_treemap(tree, view_root, bounds, state.color_mode);
            state.colored_rects = layout.file_rects;
            state.dir_rects = layout.dir_rects;
            state.treemap_dirty = false;
        }
    }

    painter.rect_filled(canvas_rect, 0.0, theme::BG_DARK);

    if state.colored_rects.is_empty() {
        let center = canvas_rect.center();
        if state.scan_progress.scanning {
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
        } else {
            // Welcome / empty state — no auto-scan on first launch.
            painter.text(
                Pos2::new(center.x, center.y - 30.0),
                egui::Align2::CENTER_CENTER,
                "MacDirStat",
                egui::FontId::proportional(24.0),
                theme::TEXT_PRIMARY,
            );
            painter.text(
                Pos2::new(center.x, center.y),
                egui::Align2::CENTER_CENTER,
                "Click  Scan Directory…  in the toolbar to begin",
                egui::FontId::proportional(13.0),
                theme::TEXT_SECONDARY,
            );
            painter.text(
                Pos2::new(center.x, center.y + 22.0),
                egui::Align2::CENTER_CENTER,
                "or drop a folder onto this window",
                egui::FontId::proportional(12.0),
                theme::TEXT_MUTED,
            );
            painter.text(
                Pos2::new(center.x, center.y + 56.0),
                egui::Align2::CENTER_CENTER,
                format!("Last: {}   ·   ⌘R to scan it", state.scan_root.display()),
                egui::FontId::proportional(11.0),
                theme::TEXT_DIM,
            );
        }
        return;
    }

    let pointer_pos = response.hover_pos();
    state.hovered_node = None;

    // Pre-compute filter state once (not per-rect)
    let search_active = state.search_active && !state.search_query.is_empty();
    let search_query_lower = if search_active {
        Some(state.search_query.to_lowercase())
    } else {
        None
    };

    // Batch all rects into a single mesh for performance
    let mut mesh = egui::Mesh::default();

    for cr in &state.colored_rects {
        let w = (cr.rect.w as f32).max(0.5);
        let h = (cr.rect.h as f32).max(0.5);
        let rect = Rect::from_min_size(
            Pos2::new(cr.rect.x as f32, cr.rect.y as f32),
            Vec2::new(w, h),
        );

        let is_large = w > 3.0 && h > 3.0;

        // Hover detection only for visible-sized rects
        if is_large && state.hovered_node.is_none() {
            if pointer_pos.map_or(false, |p| rect.contains(p)) {
                state.hovered_node = Some(cr.node_id);
            }
        }

        // Compute alpha for filtering
        let dimmed = if let Some(ref sel_ext) = state.selected_extension {
            if let Some(tree) = &state.tree {
                tree.extension(cr.node_id) != Some(sel_ext.as_str())
            } else {
                false
            }
        } else {
            false
        };

        let search_dimmed = if let Some(ref query) = search_query_lower {
            if let Some(tree) = &state.tree {
                let name = tree.name(cr.node_id);
                !name.to_lowercase().contains(query.as_str())
            } else {
                false
            }
        } else {
            false
        };

        let alpha: u8 = if dimmed || search_dimmed { 40 } else { 255 };

        let c_start = Color32::from_rgba_unmultiplied(
            cr.color_start[0],
            cr.color_start[1],
            cr.color_start[2],
            alpha,
        );

        if !is_large {
            // Tiny rect: 2 triangles, solid color
            let base = mesh.vertices.len() as u32;
            mesh.colored_vertex(rect.left_top(), c_start);
            mesh.colored_vertex(rect.right_top(), c_start);
            mesh.colored_vertex(rect.right_bottom(), c_start);
            mesh.colored_vertex(rect.left_bottom(), c_start);
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
            continue;
        }

        let c_end = Color32::from_rgba_unmultiplied(
            cr.color_end[0],
            cr.color_end[1],
            cr.color_end[2],
            alpha,
        );
        let c_mid = lerp_color(&c_start, &c_end, 0.5);

        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(rect.left_top(), c_start);
        mesh.colored_vertex(rect.right_top(), c_mid);
        mesh.colored_vertex(rect.right_bottom(), c_end);
        mesh.colored_vertex(rect.left_bottom(), c_mid);
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }

    // Add entire mesh as one shape
    painter.add(egui::Shape::mesh(mesh));

    // Draw subtle borders on shallow directory regions (depth 1-2) for orientation
    for dr in &state.dir_rects {
        if dr.depth >= 3 || dr.rect.w < 10.0 || dr.rect.h < 10.0 {
            continue;
        }
        let rect = Rect::from_min_size(
            Pos2::new(dr.rect.x as f32, dr.rect.y as f32),
            Vec2::new(dr.rect.w as f32, dr.rect.h as f32),
        );
        let alpha = if dr.depth <= 1 { 40 } else { 20 };
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, alpha)),
            StrokeKind::Inside,
        );
    }

    // Hover: highlight the containing directory regions (depth 1-3) and track deepest dir
    state.hovered_dir = None;
    if let Some(pos) = pointer_pos {
        // Find deepest dir containing pointer (for status bar)
        let mut best_depth = 0u16;
        for dr in &state.dir_rects {
            if dr.depth == 0 { continue; }
            let rect = Rect::from_min_size(
                Pos2::new(dr.rect.x as f32, dr.rect.y as f32),
                Vec2::new(dr.rect.w as f32, dr.rect.h as f32),
            );
            if rect.contains(pos) && dr.depth > best_depth {
                best_depth = dr.depth;
                state.hovered_dir = Some(dr.node_id);
            }
        }
        for dr in &state.dir_rects {
            if dr.depth == 0 || dr.depth > 3 {
                continue;
            }
            let rect = Rect::from_min_size(
                Pos2::new(dr.rect.x as f32, dr.rect.y as f32),
                Vec2::new(dr.rect.w as f32, dr.rect.h as f32),
            );
            if rect.contains(pos) && rect.width() > 5.0 && rect.height() > 5.0 {
                let alpha = if dr.depth == 1 { 50 } else { 30 };
                painter.rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(
                        if dr.depth == 1 { 2.0 } else { 1.0 },
                        theme::highlight_alpha(alpha),
                    ),
                    StrokeKind::Inside,
                );
            }
        }
    }

    // Hovered file highlight
    if let Some(hovered_id) = state.hovered_node {
        for cr in &state.colored_rects {
            if cr.node_id == hovered_id {
                let rect = Rect::from_min_size(
                    Pos2::new(cr.rect.x as f32, cr.rect.y as f32),
                    Vec2::new(cr.rect.w as f32, cr.rect.h as f32),
                );
                painter.rect_stroke(
                    rect,
                    2.0,
                    egui::Stroke::new(2.0, Color32::WHITE),
                    StrokeKind::Outside,
                );
                painter.rect_stroke(
                    rect.expand(2.0),
                    3.0,
                    egui::Stroke::new(1.0, theme::highlight_alpha(100)),
                    StrokeKind::Outside,
                );
                break;
            }
        }
    }

    // Selected node highlight — works for both files AND directories
    if let Some(selected_id) = state.selected_node {
        // First check if it's a directory (selected from tree panel)
        let mut found = false;
        for dr in &state.dir_rects {
            if dr.node_id == selected_id {
                let rect = Rect::from_min_size(
                    Pos2::new(dr.rect.x as f32, dr.rect.y as f32),
                    Vec2::new(dr.rect.w as f32, dr.rect.h as f32),
                );
                painter.rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(2.5, theme::ACCENT),
                    StrokeKind::Inside,
                );
                // Subtle fill overlay to make the region stand out
                painter.rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(
                        theme::ACCENT.r(),
                        theme::ACCENT.g(),
                        theme::ACCENT.b(),
                        24,
                    ),
                );
                found = true;
                break;
            }
        }
        // If not a directory, check files
        if !found {
            for cr in &state.colored_rects {
                if cr.node_id == selected_id {
                    let rect = Rect::from_min_size(
                        Pos2::new(cr.rect.x as f32, cr.rect.y as f32),
                        Vec2::new(cr.rect.w as f32, cr.rect.h as f32),
                    );
                    // Inner white + outer accent ring — readable against any
                    // treemap color.
                    painter.rect_stroke(
                        rect,
                        2.0,
                        egui::Stroke::new(2.0, Color32::WHITE),
                        StrokeKind::Outside,
                    );
                    painter.rect_stroke(
                        rect.expand(2.0),
                        3.0,
                        egui::Stroke::new(2.0, theme::ACCENT),
                        StrokeKind::Outside,
                    );
                    break;
                }
            }
        }
    }

    // Labels drawn separately (on top of the mesh)
    for cr in &state.colored_rects {
        let w = cr.rect.w as f32;
        let h = cr.rect.h as f32;
        if w > 40.0 && h > 16.0 {
            if let Some(tree) = &state.tree {
                let node = tree.node(cr.node_id);
                let name = tree.name(cr.node_id);
                let max_chars = (w / 7.0) as usize;
                let display_name = if name.len() > max_chars && max_chars > 3 {
                    format!(
                        "{}...",
                        &name[..max_chars.min(name.len()).saturating_sub(3)]
                    )
                } else {
                    name.to_string()
                };
                let x = cr.rect.x as f32 + 3.0;
                let y = cr.rect.y as f32 + 2.0;
                painter.text(
                    Pos2::new(x, y),
                    egui::Align2::LEFT_TOP,
                    &display_name,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                );
                if h > 28.0 {
                    painter.text(
                        Pos2::new(x, y + 12.0),
                        egui::Align2::LEFT_TOP,
                        theme::format_size(node.size),
                        egui::FontId::proportional(9.0),
                        Color32::from_rgba_unmultiplied(255, 255, 255, 150),
                    );
                }
            }
        }
    }

    // Click handling — falls back to the deepest directory under the cursor
    // when the file rect is too small to be hover-targeted (the is_large
    // filter above only registers rects > 3px). Without this fallback,
    // clicking on tiny / dense regions does nothing.
    if response.clicked() {
        let target = state.hovered_node.or(state.hovered_dir);
        if let Some(node_id) = target {
            state.selected_node = Some(node_id);
            state.selected_extension = None;
            state.expand_to_node(node_id);
            if let Some(tree) = &state.tree {
                let dir_target = if tree.node(node_id).is_dir() {
                    Some(node_id)
                } else {
                    tree.node(node_id).parent
                };
                state.scroll_dir_tree_to = dir_target;
            }
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

    // Capture context-menu target at right-click time. This must NOT be
    // re-read from hovered_node inside the menu closure — by then the pointer
    // has moved to whichever menu item is being clicked, and hovered_node
    // points to a different (or no) rect, leading to operations on the wrong
    // file (in the worst case: hovering over /Applications and triggering a
    // delete there).
    if response.secondary_clicked() {
        state.context_menu_target = state
            .hovered_node
            .or(state.hovered_dir)
            .or(state.selected_node);
    }
    response.context_menu(|ui| {
        if let Some(node_id) = state.context_menu_target {
            crate::ui::context_menu::show(ui, state, node_id);
        }
    });

    // Hover tooltip — pre-extract data so the closure doesn't re-borrow state.
    let hover_data = {
        let target = state.hovered_node.or(state.hovered_dir);
        target.and_then(|id| {
            state.tree.as_ref().map(|t| {
                let n = t.node(id);
                (
                    t.name(id).to_string(),
                    n.size,
                    t.extension(id).map(|s| s.to_string()),
                    n.modified,
                    n.is_dir(),
                )
            })
        })
    };
    if let Some((name, size, ext, modified, is_dir)) = hover_data {
        response.on_hover_ui_at_pointer(|ui| {
            ui.set_max_width(420.0);
            let display_name = if name.chars().count() > 60 {
                let mut iter = name.chars();
                let head: String = iter.by_ref().take(58).collect();
                format!("{}…", head)
            } else {
                name.clone()
            };
            ui.label(
                egui::RichText::new(display_name)
                    .strong()
                    .color(theme::TEXT_PRIMARY)
                    .size(12.5),
            );
            ui.label(
                egui::RichText::new(theme::format_size(size))
                    .color(theme::ACCENT_LIGHT)
                    .size(11.5),
            );
            if is_dir {
                ui.label(
                    egui::RichText::new("Directory")
                        .color(theme::TEXT_MUTED)
                        .size(10.0),
                );
            } else if let Some(e) = &ext {
                if !e.is_empty() && !e.starts_with("__") {
                    ui.label(
                        egui::RichText::new(format!(".{}", e))
                            .color(theme::TEXT_MUTED)
                            .size(10.0),
                    );
                }
            }
            if let Ok(elapsed) = std::time::SystemTime::now().duration_since(modified) {
                let secs = elapsed.as_secs();
                let pretty = if secs < 60 {
                    format!("{}s ago", secs)
                } else if secs < 3600 {
                    format!("{}m ago", secs / 60)
                } else if secs < 86400 {
                    format!("{}h ago", secs / 3600)
                } else if secs < 86400 * 30 {
                    format!("{}d ago", secs / 86400)
                } else if secs < 86400 * 365 {
                    format!("{}mo ago", secs / 86400 / 30)
                } else {
                    format!("{}y ago", secs / 86400 / 365)
                };
                ui.label(
                    egui::RichText::new(format!("Modified {}", pretty))
                        .color(theme::TEXT_MUTED)
                        .size(10.0),
                );
            }
        });
    }
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
