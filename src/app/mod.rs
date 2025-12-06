/*
Appstate
GUI Signals
download status model
error messages
 */

use crate::utils;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Minecraft Mod Downloader — starting up...");

    println!("{}", utils::greet());

    Ok(())
}