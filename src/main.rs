#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("serve") {
        std::process::exit(mac_dir_stat::server::run_cli(&args[1..]));
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("MacDirStat"),
        persist_window: true,
        ..Default::default()
    };
    // app_id is used by eframe for persistence storage; keep it stable.
    eframe::run_native(
        "mac-dir-stat",
        options,
        Box::new(|cc| {
            // Fonts bound now take effect on the first frame; set inside
            // update() they would only arrive on the second.
            mac_dir_stat::ui::theme::apply_theme(&cc.egui_ctx);
            Ok(Box::new(mac_dir_stat::app::App::new()))
        }),
    )
}
