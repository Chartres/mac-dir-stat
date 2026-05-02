use crate::scanner::tree::{FileTree, NodeId, NodeKind};
use crate::scanner::ScanProgress;
use crate::treemap::color::ColorMode;
use crate::treemap::{ColoredRect, DirRect};
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

    // Partial refresh: scan a single subtree and graft the result back in.
    pub partial_refresh_receiver: Option<(NodeId, Receiver<ScanProgress>)>,

    // Extension stats
    pub extension_stats: Vec<(String, u64, usize)>,

    // Treemap
    pub colored_rects: Vec<ColoredRect>,
    pub dir_rects: Vec<DirRect>,
    pub treemap_dirty: bool,
    pub color_mode: ColorMode,
    pub view_root: Option<NodeId>,
    pub zoom_stack: Vec<NodeId>,

    // Selection
    pub selected_node: Option<NodeId>,
    pub hovered_node: Option<NodeId>,
    pub selected_extension: Option<String>,
    pub hovered_dir: Option<NodeId>, // deepest directory region under cursor

    // Reveal-in-tree request — set when user clicks a treemap rect, consumed
    // by dir_tree which scrolls and clears.
    pub scroll_dir_tree_to: Option<NodeId>,

    // Captured at right-click time so the context menu always operates on the
    // node that was under the pointer when opened, not whatever happens to be
    // hovered as the menu is navigated.
    pub context_menu_target: Option<NodeId>,

    // Cleanup suggestions — recomputed on scan completion / partial refresh.
    pub cleanup_window_open: bool,
    pub cleanup_candidates: Vec<crate::cleanup::CleanupCandidate>,
    pub cleanup_selected: std::collections::HashSet<NodeId>,

    // Help / about window — toggled via toolbar or `?` shortcut.
    pub help_window_open: bool,

    // Search
    pub search_active: bool,
    pub search_query: String,

    // UI
    pub pending_action: Option<PendingAction>,
    pub request_rescan: bool,
    pub last_screen_size: egui::Vec2,
    pub last_canvas_size: egui::Vec2,

    // True when scan_root came from persisted state. Drives the welcome
    // screen — no "Last: …" hint on first run.
    pub has_persisted_root: bool,
}

pub struct ScanProgressInfo {
    pub files: usize,
    pub dirs: usize,
    pub bytes: u64,
    pub scanning: bool,
    pub current_path: Option<String>,
}

pub enum PendingAction {
    RevealInFinder(PathBuf),
    MoveToTrash(NodeId),
    ConfirmTrash(NodeId, String, u64),
    ConfirmBatchTrash(Vec<NodeId>, u64),
    ConfirmEmptyTrash,
    RefreshSubtree(NodeId),
}

impl App {
    pub fn new() -> Self {
        let persisted = crate::state::load();
        let has_persisted_root = persisted.scan_root.is_some();
        let scan_root = persisted.scan_root.unwrap_or_else(|| PathBuf::from("/"));
        let color_mode = persisted.color_mode.unwrap_or(ColorMode::Extension);
        App {
            state: AppState {
                tree: None,
                scan_root,
                scan_receiver: None,
                scan_progress: ScanProgressInfo {
                    files: 0,
                    dirs: 0,
                    bytes: 0,
                    scanning: false,
                    current_path: None,
                },
                scan_start: None,
                scan_duration_secs: 0.0,
                partial_refresh_receiver: None,
                extension_stats: vec![],
                colored_rects: vec![],
                dir_rects: vec![],
                treemap_dirty: true,
                color_mode,
                view_root: None,
                zoom_stack: vec![],
                selected_node: None,
                hovered_node: None,
                hovered_dir: None,
                selected_extension: None,
                scroll_dir_tree_to: None,
                context_menu_target: None,
                cleanup_window_open: false,
                cleanup_candidates: Vec::new(),
                cleanup_selected: std::collections::HashSet::new(),
                help_window_open: false,
                search_active: false,
                search_query: String::new(),
                pending_action: None,
                request_rescan: false,
                last_screen_size: egui::Vec2::ZERO,
                last_canvas_size: egui::Vec2::ZERO,
                has_persisted_root,
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
            current_path: None,
        };
        self.state.scan_start = Some(Instant::now());
        self.state.tree = None;
        self.state.colored_rects.clear();
        self.state.dir_rects.clear();
        self.state.extension_stats.clear();
        self.state.selected_node = None;
        self.state.hovered_node = None;
        self.state.view_root = None;
        self.state.zoom_stack.clear();
        crate::scanner::scan(self.state.scan_root.clone(), tx);
    }

    /// Spawn a fresh scan of just `node_id`'s subtree. Result is grafted
    /// back into the live tree by `poll_partial_refresh`. No-op if the node
    /// isn't a directory.
    pub fn start_partial_refresh(&mut self, node_id: NodeId) {
        let Some(tree) = &self.state.tree else { return };
        if !tree.node(node_id).is_dir() {
            return;
        }
        let path = tree.full_path(node_id);
        let (tx, rx) = crossbeam_channel::unbounded();
        self.state.partial_refresh_receiver = Some((node_id, rx));
        crate::scanner::scan(path, tx);
    }

    fn poll_partial_refresh(&mut self) {
        let mut completed: Option<(NodeId, crate::scanner::tree::FileTree)> = None;
        let mut error_occurred = false;
        if let Some((target_id, rx)) = self.state.partial_refresh_receiver.as_ref() {
            let target_id = *target_id;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanProgress::Counting { .. } => {}
                    ScanProgress::Done(new_subtree) => {
                        completed = Some((target_id, new_subtree));
                        break;
                    }
                    ScanProgress::Error(e) => {
                        eprintln!("Partial refresh error: {}", e);
                        error_occurred = true;
                        break;
                    }
                }
            }
        }
        if error_occurred {
            self.state.partial_refresh_receiver = None;
        }
        if let Some((target_id, new_subtree)) = completed {
            if let Some(tree) = &mut self.state.tree {
                tree.graft_under(target_id, new_subtree);
                let root = tree.root();
                self.state.extension_stats = tree.collect_extensions(root);
                self.state.cleanup_candidates =
                    crate::cleanup::find_candidates(tree, root);

                // Old node IDs under the target are now dead. Sanitize any
                // state that referenced them.
                if let Some(vr) = self.state.view_root {
                    if !tree.is_alive(vr) {
                        self.state.view_root = Some(target_id);
                        self.state.zoom_stack
                            .retain(|id| tree.is_alive(*id));
                        if self.state.zoom_stack.is_empty() {
                            self.state.zoom_stack.push(root);
                        }
                    }
                }
                if let Some(sel) = self.state.selected_node {
                    if !tree.is_alive(sel) {
                        self.state.selected_node = Some(target_id);
                    }
                }
                self.state.hovered_node = None;
                self.state.hovered_dir = None;
                self.state.context_menu_target = None;
                self.state.scroll_dir_tree_to = Some(target_id);
            }
            self.state.partial_refresh_receiver = None;
            self.state.treemap_dirty = true;
        }
    }

    fn poll_scan(&mut self) {
        if let Some(rx) = &self.state.scan_receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanProgress::Counting {
                        files,
                        dirs,
                        bytes,
                        current_path,
                    } => {
                        self.state.scan_progress.files = files;
                        self.state.scan_progress.dirs = dirs;
                        self.state.scan_progress.bytes = bytes;
                        self.state.scan_progress.current_path = current_path;
                    }
                    ScanProgress::Done(tree) => {
                        if let Some(start) = self.state.scan_start {
                            self.state.scan_duration_secs = start.elapsed().as_secs_f32();
                        }
                        let root = tree.root();
                        self.state.extension_stats = tree.collect_extensions(root);
                        self.state.cleanup_candidates =
                            crate::cleanup::find_candidates(&tree, root);
                        self.state.view_root = Some(root);
                        self.state.zoom_stack = vec![root];
                        self.state.tree = Some(tree);
                        self.state.scan_progress.scanning = false;
                        self.state.treemap_dirty = true;
                        crate::state::save(
                            &self.state.scan_root,
                            self.state.color_mode,
                        );
                    }
                    ScanProgress::Error(msg) => {
                        eprintln!("Scan error: {}", msg);
                        self.state.scan_progress.scanning = false;
                    }
                }
            }
        }
    }

    /// Trash a batch of nodes from a single confirmation. Failed individual
    /// items are logged and skipped; successful ones are removed from the
    /// tree. After all items are processed, recompute extension stats and
    /// cleanup candidates once.
    fn perform_batch_delete(&mut self, ids: Vec<NodeId>) {
        let mut succeeded: Vec<NodeId> = Vec::new();
        for id in &ids {
            let path = match &self.state.tree {
                Some(t) if t.is_alive(*id) => t.full_path(*id),
                _ => continue,
            };
            if let Err(e) = crate::platform::trash::move_to_trash(&path) {
                eprintln!("Batch trash failed for {}: {}", path.display(), e);
                continue;
            }
            succeeded.push(*id);
        }
        if let Some(tree) = &mut self.state.tree {
            for id in &succeeded {
                tree.remove_node(*id);
            }
            let root = tree.root();
            self.state.extension_stats = tree.collect_extensions(root);
            self.state.cleanup_candidates =
                crate::cleanup::find_candidates(tree, root);
        }
        for id in &succeeded {
            if self.state.selected_node == Some(*id) {
                self.state.selected_node = None;
            }
            if self.state.hovered_node == Some(*id) {
                self.state.hovered_node = None;
            }
            self.state.cleanup_selected.remove(id);
        }
        self.state.treemap_dirty = true;
    }

    fn perform_delete(&mut self, node_id: NodeId) {
        if let Some(tree) = &self.state.tree {
            let path = tree.full_path(node_id);
            match crate::platform::trash::move_to_trash(&path) {
                Ok(()) => {
                    if let Some(tree) = &mut self.state.tree {
                        tree.remove_node(node_id);
                        let root = tree.root();
                        self.state.extension_stats = tree.collect_extensions(root);
                        self.state.cleanup_candidates =
                            crate::cleanup::find_candidates(tree, root);
                    }
                    if self.state.selected_node == Some(node_id) {
                        self.state.selected_node = None;
                    }
                    if self.state.hovered_node == Some(node_id) {
                        self.state.hovered_node = None;
                    }
                    self.state.treemap_dirty = true;
                }
                Err(e) => {
                    eprintln!("Delete failed: {}", e);
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
        }

        self.poll_scan();
        self.poll_partial_refresh();

        if self.state.request_rescan {
            self.state.request_rescan = false;
            self.start_scan();
        }

        // Drag-and-drop: drop a folder onto the window to scan it.
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|df| df.path.clone())
                .collect()
        });
        for p in dropped {
            if p.is_dir() {
                self.state.scan_root = p;
                self.state.request_rescan = true;
                break;
            }
        }
        let hovering_files = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if hovering_files {
            let screen = ctx.screen_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("drop_overlay"),
            ));
            painter.rect_filled(
                screen,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 160),
            );
            painter.text(
                screen.center(),
                egui::Align2::CENTER_CENTER,
                "Drop folder to scan",
                egui::FontId::proportional(22.0),
                ui::theme::TEXT_PRIMARY,
            );
            ctx.request_repaint();
        }

        // Mark treemap dirty on resize
        let current_size = ctx.screen_rect().size();
        if (current_size - self.state.last_screen_size).length() > 1.0 {
            self.state.treemap_dirty = true;
            self.state.last_screen_size = current_size;
        }

        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::R) && i.modifiers.command) {
            self.state.request_rescan = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.command) {
            if let Some(path) = crate::platform::dialogs::pick_folder(&self.state.scan_root) {
                self.state.scan_root = path;
                self.state.request_rescan = true;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F) && i.modifiers.command) {
            self.state.search_active = !self.state.search_active;
            if !self.state.search_active {
                self.state.search_query.clear();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Backspace) && i.modifiers.command) {
            if let Some(node_id) = self.state.selected_node {
                if let Some(tree) = &self.state.tree {
                    let name = tree.name(node_id).to_string();
                    let size = tree.node(node_id).size;
                    self.state.pending_action = Some(PendingAction::ConfirmTrash(node_id, name, size));
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(node_id) = self.state.selected_node {
                if let Some(tree) = &self.state.tree {
                    let path = tree.full_path(node_id);
                    crate::platform::finder::reveal_in_finder(&path);
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.state.help_window_open {
                self.state.help_window_open = false;
            } else if self.state.cleanup_window_open {
                self.state.cleanup_window_open = false;
            } else if self.state.search_active {
                self.state.search_active = false;
                self.state.search_query.clear();
            } else if self.state.zoom_stack.len() > 1 {
                self.state.zoom_stack.pop();
                self.state.view_root = self.state.zoom_stack.last().copied();
                self.state.treemap_dirty = true;
            } else {
                self.state.selected_node = None;
                self.state.selected_extension = None;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Questionmark)) {
            self.state.help_window_open = !self.state.help_window_open;
        }

        if self.state.scan_progress.scanning {
            ctx.request_repaint();
        }

        // Top toolbar
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(ui::theme::BG_PANEL)
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ctx, |ui| {
                ui::toolbar::show(ui, &mut self.state);
            });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::new()
                    .fill(ui::theme::BG_PANEL)
                    .inner_margin(egui::Margin::symmetric(14, 6)),
            )
            .show(ctx, |ui| {
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
                    // Partial-refresh indicator takes priority over the
                    // hovered-file path so the user sees feedback during the
                    // background re-scan.
                    if let Some((target_id, _)) = &self.state.partial_refresh_receiver {
                        if let Some(tree) = &self.state.tree {
                            if tree.is_alive(*target_id) {
                                let path = tree.full_path(*target_id);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Refreshing  {}  …",
                                        path.display(),
                                    ))
                                    .color(ui::theme::ACCENT_LIGHT)
                                    .size(11.0),
                                );
                                ui.ctx().request_repaint();
                            }
                        }
                    } else if let Some(tree) = &self.state.tree {
                        // Show hovered file if any, otherwise show hovered directory region
                        let show_node = self.state.hovered_node
                            .filter(|&id| tree.is_alive(id))
                            .or(self.state.hovered_dir.filter(|&id| tree.is_alive(id)));
                        if let Some(id) = show_node {
                            let path = tree.full_path(id);
                            let node = tree.node(id);
                            let prefix = if self.state.hovered_node.is_some() { "" } else { "📁 " };
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}{}  •  {}",
                                    prefix,
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

        let side_frame = egui::Frame::new()
            .fill(ui::theme::BG_PANEL)
            .inner_margin(egui::Margin::symmetric(8, 8));

        // Left panel: directory tree
        egui::SidePanel::left("dir_tree")
            .default_width(340.0)
            .min_width(240.0)
            .max_width(560.0)
            .resizable(true)
            .frame(side_frame)
            .show(ctx, |ui| {
                ui::dir_tree::show(ui, &mut self.state);
            });

        // Right panel: extension list
        egui::SidePanel::right("ext_list")
            .default_width(300.0)
            .min_width(200.0)
            .max_width(440.0)
            .resizable(true)
            .frame(side_frame)
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

        // Cleanup-suggestions window (toggleable from the toolbar)
        ui::cleanup_window::show(ctx, &mut self.state);
        // Help / about window (`?` shortcut, also toolbar button)
        ui::help_window::show(ctx, &mut self.state);

        // Handle pending actions
        let mut action_to_process: Option<Option<NodeId>> = None;
        let mut batch_to_process: Option<Option<Vec<NodeId>>> = None;
        let mut empty_trash_confirmed: Option<bool> = None;
        if let Some(action) = &self.state.pending_action {
            match action {
                PendingAction::ConfirmTrash(node_id, name, size) => {
                    let node_id = *node_id;
                    let name = name.clone();
                    let size = *size;
                    let display_name = if name.chars().count() > 36 {
                        let mut iter = name.chars();
                        let head: String = iter.by_ref().take(34).collect();
                        format!("{}…", head)
                    } else {
                        name.clone()
                    };
                    egui::Window::new("Confirm Delete")
                        .title_bar(false)
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .default_width(300.0)
                        .show(ctx, |ui| {
                            ui.set_max_width(320.0);
                            ui.label(
                                egui::RichText::new("Move to Trash?")
                                    .color(ui::theme::TEXT_PRIMARY)
                                    .size(13.0)
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}  ·  {}",
                                    display_name,
                                    ui::theme::format_size(size),
                                ))
                                .color(ui::theme::TEXT_SECONDARY)
                                .size(11.0),
                            );
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui::widgets::danger_button(ui, "Move to Trash")
                                            .clicked()
                                        {
                                            action_to_process = Some(Some(node_id));
                                        }
                                        if ui::widgets::ghost_button(ui, "Cancel").clicked() {
                                            action_to_process = Some(None);
                                        }
                                    },
                                );
                            });
                        });
                }
                PendingAction::RefreshSubtree(node_id) => {
                    let node_id = *node_id;
                    self.state.pending_action = None;
                    self.start_partial_refresh(node_id);
                }
                PendingAction::ConfirmEmptyTrash => {
                    egui::Window::new("Confirm Empty Trash")
                        .title_bar(false)
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .default_width(320.0)
                        .show(ctx, |ui| {
                            ui.set_max_width(360.0);
                            ui.label(
                                egui::RichText::new("Empty Trash?")
                                    .color(ui::theme::TEXT_PRIMARY)
                                    .size(13.0)
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "Permanently deletes everything currently in your Trash. \
                                     Cannot be undone.",
                                )
                                .color(ui::theme::TEXT_SECONDARY)
                                .size(11.0),
                            );
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui::widgets::danger_button(ui, "Empty Trash")
                                            .clicked()
                                        {
                                            empty_trash_confirmed = Some(true);
                                        }
                                        if ui::widgets::ghost_button(ui, "Cancel").clicked() {
                                            empty_trash_confirmed = Some(false);
                                        }
                                    },
                                );
                            });
                        });
                }
                PendingAction::ConfirmBatchTrash(ids, total_size) => {
                    let ids = ids.clone();
                    let total_size = *total_size;
                    egui::Window::new("Confirm Batch Delete")
                        .title_bar(false)
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .default_width(320.0)
                        .show(ctx, |ui| {
                            ui.set_max_width(360.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "Move {} item{} to Trash?",
                                    ids.len(),
                                    if ids.len() == 1 { "" } else { "s" },
                                ))
                                .color(ui::theme::TEXT_PRIMARY)
                                .size(13.0)
                                .strong(),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "Total {}",
                                    ui::theme::format_size(total_size),
                                ))
                                .color(ui::theme::TEXT_SECONDARY)
                                .size(11.0),
                            );
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui::widgets::danger_button(
                                            ui,
                                            &format!("Move {} to Trash", ids.len()),
                                        )
                                        .clicked()
                                        {
                                            batch_to_process = Some(Some(ids.clone()));
                                        }
                                        if ui::widgets::ghost_button(ui, "Cancel").clicked() {
                                            batch_to_process = Some(None);
                                        }
                                    },
                                );
                            });
                        });
                }
                _ => {
                    // RevealInFinder / MoveToTrash variants are unused —
                    // context menu performs those actions inline.
                    self.state.pending_action = None;
                }
            }
        }
        if let Some(result) = action_to_process {
            self.state.pending_action = None;
            if let Some(node_id) = result {
                self.perform_delete(node_id);
            }
        }
        if let Some(result) = batch_to_process {
            self.state.pending_action = None;
            if let Some(ids) = result {
                self.perform_batch_delete(ids);
            }
        }
        if let Some(confirmed) = empty_trash_confirmed {
            self.state.pending_action = None;
            if confirmed {
                if let Err(e) = crate::platform::trash::empty_trash() {
                    eprintln!("Empty trash failed: {}", e);
                }
            }
        }
    }
}
