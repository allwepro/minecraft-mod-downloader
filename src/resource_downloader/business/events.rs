use crate::resource_downloader::business::cache::ArtifactCallback;
use crate::resource_downloader::business::rm_state::FolderImportCandidate;
use crate::resource_downloader::domain::{
    AppConfig, GameLoader, GameVersion, ListGroupLnk, ListLnk, ProjectList, ProjectLnk,
    ProjectVersion, RTProjectData, ResourceType,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[allow(dead_code)]
pub enum Event {
    // Load-in events
    Initialized {
        config: AppConfig,
        lists: Vec<ListLnk>,
        default_download_dir_by_type: HashMap<ResourceType, String>,
    },
    Reinitialize,
    FailedInitialization {
        error: String,
    },

    // List events
    ListCreated {
        name: String,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        projects: Vec<String>,
        list: ListLnk,
    },

    FailedListCreation {
        name: String,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        projects: Vec<String>,
        error: String,
    },

    ListDuplicated {
        list: ListLnk,
        dup_list: ListLnk,
    },
    FailedListDuplicated {
        list: ListLnk,
        error: String,
    },

    ListSaved {
        list: ListLnk,
    },
    FailedListSave {
        list: ListLnk,
        error: String,
    },

    ListDeleted {
        list: ListLnk,
    },
    FailedListDelete {
        list: ListLnk,
        error: String,
    },

    ListImported {
        list: ListLnk,
        path: PathBuf,
    },
    FailedListImport {
        path: PathBuf,
        error: String,
    },

    ListExported {
        list: ListLnk,
        path: PathBuf,
    },

    FailedListExport {
        list: ListLnk,
        error: String,
    },

    ProjectVersionSelected {
        list: ListLnk,
        project: ProjectLnk,
        version_id: String,
    },
    FailedProjectVersionSelect {
        list: ListLnk,
        project: ProjectLnk,
        version_id: String,
        error: String,
    },

    // File events
    FilesFound {
        directory: PathBuf,
        file_extension: Vec<String>,
        /// A vector of (file path, file hash sha1) tuples.
        files: Vec<(PathBuf, String)>,
    },

    FailedProjectArtifactDownload {
        project: ProjectLnk,
        resource_type: ResourceType,
        version_id: String,
        artifact_id: String,
        target_destination: PathBuf,
        progress_callback: Option<ArtifactCallback>,
        error: String,
    },

    ProjectFileArchived {
        path: PathBuf,
        filename: String,
    },

    FailedProjectFileArchive {
        path: PathBuf,
        filename: String,
        error: String,
    },

    ProjectFileUnarchived {
        path: PathBuf,
        filename: String,
    },

    FailedProjectFileUnarchive {
        path: PathBuf,
        filename: String,
        error: String,
    },

    ArtifactDeleted {
        path: PathBuf,
        filename: String,
    },

    FailedArtifactDelete {
        path: PathBuf,
        filename: String,
        error: String,
    },

    // Legacy events
    LegacyListImported {
        path: PathBuf,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        list: ListLnk,
        unresolved: Vec<String>,
    },

    FailedLegacyListImport {
        path: PathBuf,
        version: GameVersion,
        loader: GameLoader,
        error: String,
    },

    LegacyListProgress {
        import: bool,
        path: PathBuf,
        current: usize,
        total: usize,
        message: String,
    },

    LegacyListExported {
        list: ListLnk,
        path: PathBuf,
        version: GameVersion,
        loader: GameLoader,
        unresolved: Vec<ProjectLnk>,
    },

    FailedLegacyListExport {
        list: ListLnk,
        version: GameVersion,
        loader: GameLoader,
        error: String,
    },

    // Modrinth events
    ModrinthCollectionImported {
        collection_id: String,
        /// A map of resource type to (version, loader, project ids) tuples.
        contained_resource_ids: HashMap<ResourceType, (GameVersion, GameLoader, Vec<String>)>,
    },
    FailedModrinthCollectionImport {
        collection_id: String,
        error: String,
    },

    // Folder Import
    FolderImportScanned {
        path: PathBuf,
        resource_type: ResourceType,
        candidates: Vec<FolderImportCandidate>,
        suggested_version: Option<GameVersion>,
        suggested_loader: Option<GameLoader>,
    },
    FailedFolderScan {
        path: PathBuf,
        resource_type: ResourceType,
        error: String,
    },
    FolderImportProgress {
        total: usize,
        current: usize,
        message: String,
    },
    FolderImportCandidatesFound {
        results: HashMap<String, Vec<(ProjectLnk, String)>>,
    },

    BackupExportStarted,
    BackupExportProgress {
        current: usize,
        total: usize,
        message: String,
    },
    BackupExported {
        path: PathBuf,
    },
    FailedBackupExport {
        error: String,
    },

    BackupImportStarted,
    BackupImportProgress {
        current: usize,
        total: usize,
        message: String,
    },
    BackupImported {
        path: PathBuf,
    },
    FailedBackupImport {
        error: String,
    },
}

pub enum InternalEvent {
    Standard(Event),
    ConnectivityChanged {
        offline: bool,
    },
    Initialized {
        config: AppConfig,
        lists: Vec<(ListLnk, Arc<RwLock<ProjectList>>)>,
        default_download_dir_by_type: HashMap<ResourceType, String>,
        offline_mode: bool,
    },
    Reinitialize,
    ListCreated {
        name: String,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        projects: Vec<String>,
        list_lnk: ListLnk,
        list: Arc<RwLock<ProjectList>>,
    },
    ListDuplicated {
        list_lnk: ListLnk,
        dup_lnk: ListLnk,
        dup_list: Arc<RwLock<ProjectList>>,
        target_parent: Option<ListGroupLnk>,
    },
    ListDeleted {
        list: ListLnk,
    },
    ListImported {
        list_lnk: ListLnk,
        list: Arc<RwLock<ProjectList>>,
        path: PathBuf,
    },
    LegacyListImported {
        path: PathBuf,
        list_lnk: ListLnk,
        list_data: Arc<RwLock<ProjectList>>,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        unresolved: Vec<String>,
    },
    FilesFound {
        directory: PathBuf,
        file_extension: Vec<String>,
        files: Vec<(PathBuf, String)>,
    },
    ProjectVersionSelected {
        list_lnk: ListLnk,
        project: ProjectLnk,
        version: ProjectVersion,
        dependency_data: Vec<(ProjectLnk, ResourceType, RTProjectData)>,
    },
    FolderImportScanned {
        path: PathBuf,
        resource_type: ResourceType,
        candidates: Vec<FolderImportCandidate>,
        suggested_version: Option<GameVersion>,
        suggested_loader: Option<GameLoader>,
    },
    FailedFolderScan {
        path: PathBuf,
        resource_type: ResourceType,
        error: String,
    },
    FolderImportProgress {
        total: usize,
        current: usize,
        message: String,
    },
    FolderImportCandidatesFound {
        results: HashMap<String, Vec<(ProjectLnk, String)>>,
    },
}
