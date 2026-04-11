use crate::scanner::tree::{FileTree, NodeId, NodeKind};
use crate::scanner::ScanProgress;
use crate::treemap::color::ColorMode;
use crate::treemap::ColoredRect;
use crate::ui;
use crossbeam_channel::Receiver;
use std::path::PathBuf;
use std::time::Instant;

pub struct App {
    pub state: AppState,
    theme_applied: bool,
}

pub struct AppState {
    // Scan state
    pub tree: Option<FileTree>,
    pub scan_root: PathBuf,
    pub scan_receiver: Option<Receiver<ScanProgress>>,
    pub scan_progress: ScanProgressInfo,
    pub scan_start: Option<Instant>,
    pub scan_duration_secs: f32,

    // Extension stats
    pub extension_stats: Vec<(String, u64, usize)>,

    // Treemap
    pub colored_rects: Vec<ColoredRect>,
    pub treemap_dirty: bool,
    pub color_mode: ColorMode,
    pub view_root: Option<NodeId>,
    pub zoom_stack: Vec<NodeId>,

    // Selection
    pub selected_node: Option<NodeId>,
    pub hovered_node: Option<NodeId>,
    pub selected_extension: Option<String>,

    // Search
    pub search_active: bool,
    pub search_query: String,

    // UI
    pub pending_action: Option<PendingAction>,
    pub request_rescan: bool,
    pub last_screen_size: egui::Vec2,
}

pub struct ScanProgressInfo {
    pub files: usize,
    pub dirs: usize,
    pub bytes: u64,
    pub scanning: bool,
}

pub enum PendingAction {
    RevealInFinder(PathBuf),
    MoveToTrash(NodeId),
    ConfirmTrash(NodeId, String, u64),
}

impl App {
    pub fn new() -> Self {
        App {
            state: AppState {
                tree: None,
                scan_root: PathBuf::from("/"),
                scan_receiver: None,
                scan_progress: ScanProgressInfo {
                    files: 0,
                    dirs: 0,
                    bytes: 0,
                    scanning: false,
                },
                scan_start: None,
                scan_duration_secs: 0.0,
                extension_stats: vec![],
                colored_rects: vec![],
                treemap_dirty: true,
                color_mode: ColorMode::Extension,
                view_root: None,
                zoom_stack: vec![],
                selected_node: None,
                hovered_node: None,
                selected_extension: None,
                search_active: false,
                search_query: String::new(),
                pending_action: None,
                request_rescan: false,
                last_screen_size: egui::Vec2::ZERO,
            },
            theme_applied: false,
        }
    }

    pub fn start_scan(&mut self) {
        let (tx, rx) = crossbeam_channel::unbounded();
        self.state.scan_receiver = Some(rx);
        self.state.scan_progress = ScanProgressInfo {
            files: 0,
            dirs: 0,
            bytes: 0,
            scanning: true,
        };
        self.state.scan_start = Some(Instant::now());
        self.state.tree = None;
        self.state.colored_rects.clear();
        self.state.extension_stats.clear();
        self.state.selected_node = None;
        self.state.hovered_node = None;
        self.state.view_root = None;
        self.state.zoom_stack.clear();
        crate::scanner::scan(self.state.scan_root.clone(), tx);
    }

    fn poll_scan(&mut self) {
        if let Some(rx) = &self.state.scan_receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanProgress::Counting { files, dirs, bytes } => {
                        self.state.scan_progress.files = files;
                        self.state.scan_progress.dirs = dirs;
                        self.state.scan_progress.bytes = bytes;
                    }
                    ScanProgress::Done(tree) => {
                        if let Some(start) = self.state.scan_start {
                            self.state.scan_duration_secs = start.elapsed().as_secs_f32();
                        }
                        let root = tree.root();
                        self.state.extension_stats = tree.collect_extensions(root);
                        self.state.view_root = Some(root);
                        self.state.zoom_stack = vec![root];
                        self.state.tree = Some(tree);
                        self.state.scan_progress.scanning = false;
                        self.state.treemap_dirty = true;
                    }
                    ScanProgress::Error(msg) => {
                        eprintln!("Scan error: {}", msg);
                        self.state.scan_progress.scanning = false;
                    }
                }
            }
        }
    }
}

impl AppState {
    pub fn expand_to_node(&mut self, node_id: NodeId) {
        if let Some(tree) = &mut self.tree {
            let mut current = tree.node(node_id).parent;
            while let Some(pid) = current {
                if let NodeKind::Directory { expanded, .. } = &mut tree.node_mut(pid).kind {
                    *expanded = true;
                }
                current = tree.node(pid).parent;
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            ui::theme::apply_theme(ctx);
            self.theme_applied = true;
            self.start_scan();
        }

        self.poll_scan();

        if self.state.request_rescan {
            self.state.request_rescan = false;
            self.start_scan();
        }

        // Mark treemap dirty on resize
        let current_size = ctx.screen_rect().size();
        if (current_size - self.state.last_screen_size).length() > 1.0 {
            self.state.treemap_dirty = true;
            self.state.last_screen_size = current_size;
        }

        if self.state.scan_progress.scanning {
            ctx.request_repaint();
        }

        // Top toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui::toolbar::show(ui, &mut self.state);
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(tree) = &self.state.tree {
                    let root = tree.root();
                    let file_count = tree.collect_files(root).len();
                    ui.label(
                        egui::RichText::new(format!(
                            "{} files  •  {} dirs  •  {:.1}s",
                            file_count,
                            self.state.scan_progress.dirs,
                            self.state.scan_duration_secs,
                        ))
                        .color(ui::theme::TEXT_MUTED)
                        .size(11.0),
                    );
                } else if self.state.scan_progress.scanning {
                    ui.label(
                        egui::RichText::new(format!(
                            "Scanning... {} files  •  {} dirs  •  {}",
                            self.state.scan_progress.files,
                            self.state.scan_progress.dirs,
                            ui::theme::format_size(self.state.scan_progress.bytes),
                        ))
                        .color(ui::theme::ACCENT_LIGHT)
                        .size(11.0),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let (Some(tree), Some(hovered)) = (&self.state.tree, self.state.hovered_node) {
                        if tree.is_alive(hovered) {
                            let path = tree.full_path(hovered);
                            let node = tree.node(hovered);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}  •  {}",
                                    path.display(),
                                    ui::theme::format_size(node.size),
                                ))
                                .color(ui::theme::ACCENT_LIGHT)
                                .size(11.0),
                            );
                        }
                    }
                });
            });
        });

        // Left panel: directory tree
        egui::SidePanel::left("dir_tree")
            .default_width(300.0)
            .min_width(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui::dir_tree::show(ui, &mut self.state);
            });

        // Right panel: extension list
        egui::SidePanel::right("ext_list")
            .default_width(280.0)
            .min_width(180.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui::ext_list::show(ui, &mut self.state);
            });

        // Central panel: treemap
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.search_active {
                ui::search::show(ui, &mut self.state);
            }
            ui::treemap_view::show(ui, &mut self.state);
        });
    }
}
