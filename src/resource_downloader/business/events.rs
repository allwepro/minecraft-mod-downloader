use crate::resource_downloader::business::cache::ArtifactCallback;
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, ProjectList, ProjectLnk, ProjectVersion, RTProjectData,
    ResourceType,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[allow(dead_code)]
pub enum Event {
    // Load-in events
    Initialized {
        last_open_list_id: Option<ListLnk>,
        default_list_name: String,
        lists: Vec<ListLnk>,
        default_download_dir_by_type: HashMap<ResourceType, String>,
    },
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
}

pub enum InternalEvent {
    Standard(Event),
    Initialized {
        last_open_list_id: Option<ListLnk>,
        default_list_name: String,
        lists: Vec<(ListLnk, Arc<RwLock<ProjectList>>)>,
        default_download_dir_by_type: HashMap<ResourceType, String>,
    },
    ListCreated {
        name: String,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        projects: Vec<String>,
        lnk: ListLnk,
        list: Arc<RwLock<ProjectList>>,
    },
    ListDuplicated {
        list: ListLnk,
        dup_lnk: ListLnk,
        dup_list: Arc<RwLock<ProjectList>>,
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
        list: ListLnk,
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
}
