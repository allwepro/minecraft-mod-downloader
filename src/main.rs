mod adapters;
mod app;
mod domain;
mod infra;

use app::App;
use eframe::NativeOptions;
use egui::IconData;
use tokio::runtime::Runtime;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let runtime = Runtime::new().expect("Failed to create Tokio runtime");

    let icon_path = "assets/icon.png";
    let image = image::open(icon_path).expect("Failed to open image");
    let image_rgba = image.to_rgba8();
    let (width, height) = image_rgba.dimensions();

    let icon_data = IconData {
        rgba: image_rgba.into_raw(),
        width,
        height,
    };

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_title("Minecraft Mod Downloader")
            .with_icon(icon_data),
        ..Default::default()
    };

    eframe::run_native(
        "Minecraft Mod Downloader",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(cc, runtime)) as Box<dyn eframe::App>)
        }),
    )
}
