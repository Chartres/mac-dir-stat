//! "Move to External Disk…" confirmation: pick a volume, see destination,
//! free space and any refusal reasons before anything is copied.

use crate::relocate::{self, Measure, Volume};
use crate::scanner::tree::NodeId;
use crate::ui::{theme, widgets};
use crossbeam_channel::Receiver;
use egui::{Color32, RichText};
use std::path::PathBuf;

pub struct RelocateDialog {
    pub node: NodeId,
    pub name: String,
    pub path: PathBuf,
    volumes: Vec<Volume>,
    selected: usize,
    link: bool,
    measure: Option<Measure>,
    measure_rx: Receiver<Measure>,
}

impl RelocateDialog {
    /// Lists destination volumes and measures the source off the UI thread.
    pub fn new(node: NodeId, name: String, path: PathBuf) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let p = path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(relocate::measure(&p, false));
        });
        RelocateDialog {
            node,
            name,
            path,
            volumes: relocate::list_volumes()
                .into_iter()
                .filter(|v| !v.boot)
                .collect(),
            selected: 0,
            link: true,
            measure: None,
            measure_rx: rx,
        }
    }
}

pub enum Choice {
    Cancel,
    Move { volume: PathBuf, link: bool },
}

pub fn show(ctx: &egui::Context, d: &mut RelocateDialog) -> Option<Choice> {
    if d.measure.is_none() {
        d.measure = d.measure_rx.try_recv().ok();
        ctx.request_repaint();
    }
    let mut choice = None;
    egui::Window::new("Move to External Disk")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.set_max_width(460.0);
            ui.label(
                RichText::new("Move to External Disk")
                    .color(theme::TEXT_PRIMARY)
                    .size(13.0)
                    .strong(),
            );
            ui.add_space(4.0);
            let size = d
                .measure
                .as_ref()
                .map_or("measuring…".into(), |m| theme::format_size(m.bytes));
            ui.label(
                RichText::new(format!("{}  ·  {}", d.name, size))
                    .color(theme::TEXT_SECONDARY)
                    .size(11.0),
            );
            ui.add_space(8.0);

            let mut check = None;
            if d.volumes.is_empty() {
                ui.label(
                    RichText::new("No external disk is mounted.")
                        .color(theme::TEXT_SECONDARY)
                        .size(11.0),
                );
            } else {
                for (i, v) in d.volumes.iter().enumerate() {
                    let label = format!(
                        "{}  ({}, {} free)",
                        v.name,
                        v.fs.fs_type,
                        theme::format_size(v.fs.free)
                    );
                    ui.radio_value(&mut d.selected, i, RichText::new(label).size(12.0));
                }
                let vol = &d.volumes[d.selected];
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "To: {}",
                        relocate::dest_path(&vol.path, &d.path).display()
                    ))
                    .color(theme::TEXT_MUTED)
                    .size(11.0),
                );
                ui.checkbox(
                    &mut d.link,
                    RichText::new("Leave a link behind at the original location").size(12.0),
                );
                if let Some(m) = &d.measure {
                    let c = relocate::check_with(&d.path, &vol.path, m);
                    match &c {
                        Ok(c) => {
                            for e in &c.errors {
                                ui.label(RichText::new(e).color(theme::DANGER).size(11.0));
                            }
                            for w in &c.warnings {
                                ui.label(
                                    RichText::new(w)
                                        .color(Color32::from_rgb(0xe2, 0xa4, 0x3b))
                                        .size(11.0),
                                );
                            }
                        }
                        Err(e) => {
                            ui.label(RichText::new(e).color(theme::DANGER).size(11.0));
                        }
                    }
                    check = c.ok();
                } else {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            RichText::new("Checking…")
                                .color(theme::TEXT_MUTED)
                                .size(11.0),
                        );
                    });
                }
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let ready = check.as_ref().is_some_and(|c| c.ok());
                    if ui
                        .add_enabled_ui(ready, |ui| {
                            widgets::primary_button(ui, "Copy, Verify & Move")
                        })
                        .inner
                        .clicked()
                    {
                        choice = Some(Choice::Move {
                            volume: d.volumes[d.selected].path.clone(),
                            link: d.link,
                        });
                    }
                    if widgets::ghost_button(ui, "Cancel").clicked() {
                        choice = Some(Choice::Cancel);
                    }
                });
            });
        });
    choice
}
