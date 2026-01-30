use crate::resource_downloader::business::cache::ArtifactCallback;
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, ProjectList, ProjectLnk, ResourceType,
};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub enum Effect {
    // Load-in effects
    Initialize,

    // Program effects
    SaveConfig {
        last_open_list_id: Option<ListLnk>,
        default_list_name: String,
    },

    // List effects
    CreateList {
        name: String,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        projects: Vec<String>,
    },

    SaveList {
        list: Arc<RwLock<ProjectList>>,
    },

    DuplicateList {
        list: Arc<RwLock<ProjectList>>,
    },

    DeleteList {
        list: ListLnk,
    },

    ImportList {
        path: PathBuf,
    },

    ExportList {
        list: Arc<RwLock<ProjectList>>,
        path: PathBuf,
    },

    // Project effects
    #[allow(dead_code)]
    SelectProjectVersion {
        list: Arc<RwLock<ProjectList>>,
        project: ProjectLnk,
        version_id: String,
    },

    // File effects
    FindFiles {
        directory: PathBuf,
        file_extension: String,
    },

    DownloadProjectArtifact {
        project: ProjectLnk,
        resource_type: ResourceType,
        version_id: String,
        artifact_id: String,
        target_destination: PathBuf,
        progress_callback: Option<ArtifactCallback>,
    },

    ArchiveProjectFile {
        path: PathBuf,
        filename: String,
    },

    UnarchiveProjectFile {
        path: PathBuf,
        filename: String,
    },

    DeleteArtifact {
        path: PathBuf,
        filename: String,
    },

    OpenExplorer {
        path: PathBuf,
    },

    // Legacy effects
    ImportLegacyList {
        path: PathBuf,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
    },

    ExportLegacyList {
        list: Arc<RwLock<ProjectList>>,
        path: PathBuf,
        version: GameVersion,
        loader: GameLoader,
    },

    // Modrinth effects
    ImportModrinthCollection {
        collection_id: String,
    },
}
