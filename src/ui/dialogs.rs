use crate::domain::ProjectType;
use rfd::FileDialog;
use std::path::PathBuf;

pub struct Dialogs;

impl Dialogs {
    pub fn pick_minecraft_mods_folder() -> Option<PathBuf> {
        if let Some(default_path) =
            crate::infra::ConfigManager::get_default_minecraft_download_dir(ProjectType::Mod)
        {
            return FileDialog::new()
                .set_title("Select Minecraft Mods Folder")
                .set_directory(&default_path)
                .pick_folder();
        }

        FileDialog::new()
            .set_title("Select Minecraft Mods Folder")
            .pick_folder()
    }

    pub fn save_export_list_file(default_name: &str) -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("MMD List", &["mmd"])
            .add_filter("Legacy Mod List", &["mods", "all-mods", "queue-mods"])
            .set_title("Export List")
            .set_file_name(format!("{default_name}.mmd"))
            .save_file()
    }

    pub fn pick_import_list_file() -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("MMD List", &["mmd"])
            .add_filter("Legacy Mod List", &["mods", "all-mods", "queue-mods"])
            .pick_file()
    }
}
