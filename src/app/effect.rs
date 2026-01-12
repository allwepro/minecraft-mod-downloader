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
}
