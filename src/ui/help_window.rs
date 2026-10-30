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
                    .color(theme::ink())
                    .strong()
                    .size(15.0),
            );
            ui.label(
                RichText::new("Treemap directory-size visualizer for macOS")
                    .color(theme::ink_soft())
                    .size(11.0),
            );
            ui.add_space(10.0);
            widgets::subtle_divider(ui);
            ui.add_space(10.0);

            ui.label(
                RichText::new("Keyboard shortcuts")
                    .color(theme::ink())
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
            shortcut(ui, "↑ ↓", "Move selection up/down the directory list");
            shortcut(ui, "→ ←", "Expand / collapse the selected directory");
            shortcut(ui, "⌘⌫", "Move selected node to Trash");
            shortcut(ui, "↩", "Reveal selected node in Finder");
            shortcut(ui, "Esc", "Close help/cleanup/search · pop zoom · clear selection");
            shortcut(ui, "?", "Toggle this help");

            ui.add_space(10.0);
            widgets::subtle_divider(ui);
            ui.add_space(10.0);

            ui.label(
                RichText::new("Tips")
                    .color(theme::ink())
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

            feedback_section(ui, state);

            ui.add_space(10.0);
            widgets::subtle_divider(ui);
            ui.add_space(10.0);

            privacy_section(ui, state);

            ui.add_space(10.0);
            widgets::subtle_divider(ui);
            ui.add_space(10.0);

            ui.label(
                RichText::new("github.com/Chartres/mac-dir-stat")
                    .color(theme::highlight())
                    .size(10.0),
            );
        });
    state.help_window_open = open;
}

/// Sean Ellis product-market-fit micro-survey. Feeds the flywheel feedback
/// table; only submits when telemetry is on (a no-op otherwise).
fn feedback_section(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(
        RichText::new("How would you feel if MacDirStat went away?")
            .color(theme::ink())
            .strong()
            .size(12.0),
    );
    ui.add_space(4.0);
    if state.feedback_sent {
        ui.label(
            RichText::new("Thanks — feedback sent.")
                .color(theme::ink_soft())
                .size(11.0),
        );
        return;
    }
    ui.horizontal(|ui| {
        for (val, label) in [
            ("very", "Very disappointed"),
            ("somewhat", "Somewhat"),
            ("not", "Not disappointed"),
        ] {
            let selected = state.feedback_choice == Some(val);
            if ui.selectable_label(selected, label).clicked() {
                state.feedback_choice = Some(val);
            }
        }
    });
    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(&mut state.feedback_text)
            .hint_text("Anything you'd change? (optional)")
            .desired_width(f32::INFINITY),
    );
    ui.add_space(4.0);
    let can_send = state.feedback_choice.is_some();
    if ui.add_enabled(can_send, egui::Button::new("Send feedback")).clicked() {
        let text = state.feedback_text.trim();
        state.flywheel.feedback(
            state.feedback_choice,
            None,
            if text.is_empty() { None } else { Some(text) },
        );
        state.feedback_sent = true;
        state.feedback_text.clear();
    }
    if !state.flywheel.is_enabled() {
        ui.label(
            RichText::new("(Telemetry is off — feedback is not sent.)")
                .color(theme::note())
                .size(10.0),
        );
    }
}

/// Telemetry disclosure + opt-out toggle. Release builds bake in the analytics
/// endpoint and send until the user ticks this box (or sets
/// MACDIRSTAT_NO_TELEMETRY=1); source builds have no endpoint and send nothing.
fn privacy_section(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(
        RichText::new("Privacy")
            .color(theme::ink())
            .strong()
            .size(12.0),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "MacDirStat sends anonymous, cookieless usage counts (app opens, scans, \
             space reclaimed) to help prioritize features. No file names or paths are \
             ever sent. You can turn this off.",
        )
        .color(theme::ink_soft())
        .size(11.0),
    );
    ui.add_space(4.0);
    let mut opted_out = crate::flywheel::telemetry_opted_out();
    if ui
        .checkbox(&mut opted_out, "Don't send anonymous usage data")
        .changed()
    {
        crate::flywheel::set_telemetry_opt_out(opted_out);
        state.flywheel.refresh_enabled();
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "History keeps a short summary of each scan (date, total, the names and \
             sizes of the largest top-level folders; never file names or lists) in ~/Library/Application Support/\
             mac-dir-stat/history.json, six per folder. It never leaves this Mac. \
             Delete the file to forget it.",
        )
        .color(theme::ink_soft())
        .size(11.0),
    );
}

fn shortcut(ui: &mut egui::Ui, keys: &str, desc: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{:>6}", keys))
                .color(theme::highlight())
                .monospace()
                .size(11.0),
        );
        ui.label(
            RichText::new(desc)
                .color(theme::ink_soft())
                .size(11.0),
        );
    });
}

fn tip(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .color(theme::ink_soft())
            .size(11.0),
    );
}
