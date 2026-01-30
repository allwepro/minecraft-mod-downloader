use rfd::FileDialog;
use std::path::PathBuf;

pub struct Dialogs;

impl Dialogs {
    pub fn pick_minecraft_mods_folder(default_path: &mut String) -> Option<PathBuf> {
        let result = FileDialog::new()
            .set_title("Select Folder")
            .set_directory(default_path.clone())
            .pick_folder();
        if let Some(path) = result.clone() {
            *default_path = path.as_path().display().to_string();
        }
        result
    }

    pub fn pick_import_list_file() -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("List", &["mmd", "mods", "all-mods", "queue-mods"])
            .set_title("Import List")
            //.set_directory(dirs::desktop_dir().unwrap_or_else(|| PathBuf::from(".")))
            .pick_file()
    }

    pub fn save_export_list_file(default_name: &str, allow_legacy: bool) -> Option<PathBuf> {
        let mut fd = FileDialog::new()
            //.set_directory(dirs::desktop_dir().unwrap_or_else(|| PathBuf::from(".")))
            .add_filter("MMD List", &["mmd"])
            .set_title("Export List")
            .set_file_name(format!("{default_name}.mmd"));
        if allow_legacy {
            fd = fd.add_filter("Legacy Mod List", &["mods", "all-mods", "queue-mods"]);
        }
        fd.save_file()
    }
}
