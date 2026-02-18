use crate::resource_downloader::business::cache::ArtifactCallback;
use crate::resource_downloader::domain::{
    AppConfig, GameLoader, GameVersion, ListGroupLnk, ListLnk, ProjectList, ProjectLnk,
    ResourceType,
};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub enum Effect {
    // Load-in effects
    Initialize,

    PingConnectivity,

    // Program effects
    SaveConfig {
        config: AppConfig,
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
        target_parent: Option<ListGroupLnk>,
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
        file_extension: Vec<String>,
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
        list_name: String,
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

    // Folder Import effects
    ScanFolderImport {
        path: PathBuf,
        resource_type: ResourceType,
    },

    SearchFolderImportCandidates {
        file_names: Vec<String>,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
    },

    ImportFolderList {
        name: String,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        projects: Vec<ProjectLnk>,
    },

    // Other
    ClearFileIndexCache,

    // Backup/Restore effects
    ExportBackup {
        path: PathBuf,
    },

    ImportBackup {
        path: PathBuf,
    },
}
