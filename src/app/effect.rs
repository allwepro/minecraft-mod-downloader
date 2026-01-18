use crate::domain::{ModInfo, ModList, ProjectType};
use std::path::PathBuf;
use std::sync::Arc;
#[derive(Clone)]
pub enum Effect {
    LoadInitialData,

    LoadLoadersForType {
        project_type: ProjectType,
    },

    SearchMods {
        query: String,
        version: String,
        loader: String,
        project_type: ProjectType,
    },

    FetchModDetails {
        mod_id: String,
        version: String,
        loader: String,
    },

    DownloadMod {
        mod_info: Arc<ModInfo>,
        download_dir: String,
    },

    SaveList {
        list: ModList,
    },
    DeleteList {
        list_id: String,
    },

    SaveConfig {
        current_list_id: Option<String>,
        default_list_name: String,
    },

    ExportListToml {
        path: PathBuf,
        list: ModList,
    },

    LegacyListImport {
        path: PathBuf,
        version: String,
        loader: String,
    },

    LegacyListExport {
        path: PathBuf,
        mod_ids: Vec<String>,
        version: String,
        loader: String,
    },

    RemoveFromMetadata {
        download_dir: String,
        mod_id: String,
    },

    DeleteModFile {
        download_dir: String,
        mod_id: String,
    },

    DeleteUnknownFile {
        download_dir: String,
        filename: String,
    },

    ArchiveModFile {
        download_dir: String,
        mod_id: String,
    },

    UnarchiveModFile {
        download_dir: String,
        mod_id: String,
    },

    ValidateMetadata {
        download_dir: String,
    },

    ImportModrinthCollection {
        collection_id: String,
        filter_mods: bool,
        filter_resourcepacks: bool,
        filter_shaders: bool,
        filter_datapacks: bool,
        filter_modpacks: bool,
        filter_plugins: bool,
    },
}
