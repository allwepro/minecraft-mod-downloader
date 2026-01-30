mod common;
mod resource_downloader;

use crate::common::app::App;
use crate::common::app_icon::get_app_icon;
use eframe::NativeOptions;
use tokio::runtime::Runtime;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let runtime = Runtime::new().expect("Failed to create Tokio runtime");

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_min_inner_size([1000.0, 400.0])
            .with_title("Flux Launcher & Resource Downloader")
            .with_icon(get_app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Flux Launcher & Resource Downloader",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(cc, runtime)) as Box<dyn eframe::App>)
        }),
    )
}
