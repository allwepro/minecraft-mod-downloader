mod common;
mod resource_downloader;

use crate::common::app::App;
use crate::common::app_icon::get_app_icon;
use crate::common::program_args::{ArgRegistryBuilder, SharedArgRegistry};
use crate::resource_downloader::app::rd_handler::RDHandler;
use eframe::NativeOptions;
use std::env;
use tokio::runtime::Runtime;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let args_registry: SharedArgRegistry = {
        let mut arb = ArgRegistryBuilder::new();
        RDHandler::args(&mut arb);
        arb.add("h", "help", "Lists the help commands");
        arb.build(args)
    };
    if args_registry.get("help").is_some() {
        args_registry.print_help();
    }

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
            Ok(Box::new(App::new(cc, runtime, args_registry)) as Box<dyn eframe::App>)
        }),
    )
}
