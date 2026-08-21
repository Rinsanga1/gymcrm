#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("TenneCRM"),
        ..Default::default()
    };
    eframe::run_native(
        "TenneCRM",
        options,
        Box::new(|cc| Ok(Box::new(tenne_crm::app::App::new(cc)))),
    )
}
