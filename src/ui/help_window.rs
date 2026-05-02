use crate::app::AppState;
use crate::ui::{theme, widgets};
use egui::{Context, RichText};

pub fn show(ctx: &Context, state: &mut AppState) {
    if !state.help_window_open {
        return;
    }
    let mut open = state.help_window_open;
    egui::Window::new("MacDirStat — Help")
        .open(&mut open)
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.label(
                RichText::new(format!("MacDirStat v{}", env!("CARGO_PKG_VERSION")))
                    .color(theme::TEXT_PRIMARY)
                    .strong()
                    .size(15.0),
            );
            ui.label(
                RichText::new("Treemap directory-size visualizer for macOS")
                    .color(theme::TEXT_SECONDARY)
                    .size(11.0),
            );
            ui.add_space(10.0);
            widgets::subtle_divider(ui);
            ui.add_space(10.0);

            ui.label(
                RichText::new("Keyboard shortcuts")
                    .color(theme::TEXT_PRIMARY)
                    .strong()
                    .size(12.0),
            );
            ui.add_space(4.0);
            shortcut(ui, "⌘O", "Pick a directory to scan");
            shortcut(ui, "⌘R", "Re-scan current root");
            shortcut(ui, "⇧⌘R", "Re-scan only the selected subtree");
            shortcut(ui, "⌘1", "Color treemap by extension");
            shortcut(ui, "⌘2", "Color treemap by depth");
            shortcut(ui, "⌘3", "Color treemap by modified age");
            shortcut(ui, "⌘F", "Search files in scanned tree");
            shortcut(ui, "⌘⌫", "Move selected node to Trash");
            shortcut(ui, "↩", "Reveal selected node in Finder");
            shortcut(ui, "Esc", "Close help/cleanup/search · pop zoom · clear selection");
            shortcut(ui, "?", "Toggle this help");

            ui.add_space(10.0);
            widgets::subtle_divider(ui);
            ui.add_space(10.0);

            ui.label(
                RichText::new("Tips")
                    .color(theme::TEXT_PRIMARY)
                    .strong()
                    .size(12.0),
            );
            ui.add_space(4.0);
            tip(ui, "• Click any treemap rect — even tiny — to select what's there or its containing folder.");
            tip(ui, "• Right-click for Reveal · Refresh · Zoom · Trash actions.");
            tip(ui, "• Drag a folder onto the window to scan it.");
            tip(ui, "• The Cleanup button surfaces regenerable directories (caches, build outputs, simulators).");
            tip(ui, "• Hover any rect for path · size · type · modified-time tooltip.");

            ui.add_space(10.0);
            widgets::subtle_divider(ui);
            ui.add_space(10.0);

            ui.label(
                RichText::new("github.com/Chartres/mac-dir-stat")
                    .color(theme::ACCENT_LIGHT)
                    .size(10.0),
            );
        });
    state.help_window_open = open;
}

fn shortcut(ui: &mut egui::Ui, keys: &str, desc: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{:>6}", keys))
                .color(theme::ACCENT_LIGHT)
                .monospace()
                .size(11.0),
        );
        ui.label(
            RichText::new(desc)
                .color(theme::TEXT_SECONDARY)
                .size(11.0),
        );
    });
}

fn tip(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .color(theme::TEXT_SECONDARY)
            .size(11.0),
    );
}
