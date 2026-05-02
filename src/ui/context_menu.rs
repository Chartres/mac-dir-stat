use crate::app::{AppState, PendingAction};
use crate::scanner::tree::NodeId;
use crate::ui::theme;
use egui::Color32;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, node: NodeId) {
    let (name, size, path) = if let Some(tree) = &state.tree {
        let n = tree.node(node);
        (
            tree.name(node).to_string(),
            n.size,
            tree.full_path(node),
        )
    } else {
        return;
    };

    // Force a usable width — without this the menu measures children at zero
    // desired width on the first frame and renders ~2 chars wide.
    ui.set_min_width(200.0);
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

    let display_name = if name.chars().count() > 32 {
        let mut iter = name.chars();
        let head: String = iter.by_ref().take(30).collect();
        format!("{}…", head)
    } else {
        name.clone()
    };

    ui.label(
        egui::RichText::new(&display_name)
            .color(theme::TEXT_PRIMARY)
            .strong()
            .size(12.0),
    );
    ui.label(
        egui::RichText::new(theme::format_size(size))
            .color(theme::TEXT_MUTED)
            .size(11.0),
    );
    ui.separator();

    if ui.button("Show in Finder").clicked() {
        crate::platform::finder::reveal_in_finder(&path);
        ui.close_menu();
    }
    if ui.button("Copy Path").clicked() {
        ui.ctx().copy_text(path.to_string_lossy().to_string());
        ui.close_menu();
    }
    ui.separator();
    if ui
        .button(egui::RichText::new("Move to Trash").color(Color32::from_rgb(248, 113, 113)))
        .clicked()
    {
        state.pending_action = Some(PendingAction::ConfirmTrash(node, name.clone(), size));
        ui.close_menu();
    }
}
