use crate::app::{AppState, PendingAction};
use crate::scanner::tree::NodeId;
use crate::ui::theme;
use egui::Color32;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, node: NodeId) {
    let (name, size, path) = if let Some(tree) = &state.tree {
        let n = tree.node(node);
        (
            n.name.to_string_lossy().to_string(),
            n.size,
            tree.full_path(node),
        )
    } else {
        return;
    };

    ui.label(
        egui::RichText::new(&name)
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
