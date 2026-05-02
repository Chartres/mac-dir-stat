#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("mac-dir-stat"),
        ..Default::default()
    };
    eframe::run_native(
        "mac-dir-stat",
        options,
        Box::new(|_cc| Ok(Box::new(mac_dir_stat::app::App::new()))),
    )
}
