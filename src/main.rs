mod common;
mod launcher;
mod resource_downloader;

use crate::common::app::App;
use crate::common::cli::program_args::{ArgRegistryBuilder, SharedArgRegistry};
use crate::common::cli::structs::args_supplier::ArgsSupplier;
use crate::resource_downloader::RMCLI;
use common::ui::app_icon::get_app_icon;
use eframe::NativeOptions;
use std::env;
use tokio::runtime::Runtime;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let arg_suppliers: Vec<Box<dyn ArgsSupplier>> = vec![Box::new(RMCLI)];
    let args_registry: SharedArgRegistry = {
        let mut arb = ArgRegistryBuilder::new();
        for mut arg_supplier in arg_suppliers {
            arg_supplier.supply(&mut arb);
        }
        arb.add("h", "help", "Lists the help commands");
        arb.build(args)
    };
    if args_registry.get("help").is_some() {
        args_registry.print_help();
        return Ok(());
    }

    let runtime = Runtime::new().expect("Failed to create Tokio runtime");

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_min_inner_size([1000.0, 400.0])
            .with_title("Flux Project")
            .with_icon(get_app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Flux Project",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(cc, runtime, args_registry)) as Box<dyn eframe::App>)
        }),
    )
}
