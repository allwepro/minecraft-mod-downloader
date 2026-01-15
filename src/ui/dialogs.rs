use rfd::FileDialog;
use std::path::PathBuf;

pub struct Dialogs;

impl Dialogs {
    pub fn pick_folder() -> Option<PathBuf> {
        FileDialog::new().pick_folder()
    }

    pub fn save_export_list_file(default_name: &str) -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("MMD List", &["mmd"])
            .add_filter("Legacy Mod List", &["mods", "all-mods", "queue-mods"])
            .set_title("Export List")
            .set_file_name(format!("{}.mmd", default_name))
            .save_file()
    }

    pub fn pick_import_list_file() -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("MMD List", &["mmd"])
            .add_filter("Legacy Mod List", &["mods", "all-mods", "queue-mods"])
            .pick_file()
    }
}
