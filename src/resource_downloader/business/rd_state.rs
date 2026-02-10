use crate::common::modal_manager::SharedModalManager;
use crate::common::notification_manager::SharedNotificationManager;
use crate::common::pop_up_manager::SharedPopupManager;
use crate::common::prefabs::modal_window::ModalWindow;
use crate::common::prefabs::notification_window::Notification;
use crate::resource_downloader::business::Effect;
use crate::resource_downloader::business::cache::ArtifactCallback;
use crate::resource_downloader::business::list_pool::ListPool;
use crate::resource_downloader::business::services::ApiService;
use crate::resource_downloader::business::{Event, InternalEvent};
use crate::resource_downloader::domain::{
    AppConfig, ListLnk, MutationOutcome, MutationResult, Project, ProjectLnk, ResourceType,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Idle,
    Queued,
    Downloading,
    Complete,
    Failed,
}

#[derive(Clone, Debug)]
pub struct ClipboardContent {
    pub source_list: ListLnk,
    pub projects: Vec<ProjectLnk>,
    pub is_cut: bool,
}

pub type SharedRDState = Arc<RwLock<RDState>>;

pub struct RDState {
    rt_handle: tokio::runtime::Handle,
    event_rx: mpsc::Receiver<InternalEvent>,
    effect_sx: mpsc::Sender<Effect>,

    api_service: Arc<ApiService>,

    pub modal_manager: SharedModalManager,
    pub popup_manager: SharedPopupManager,
    pub notification_manager: SharedNotificationManager,

    pub loading: bool,

    pub default_dirs: HashMap<ResourceType, String>,
    pub config: Arc<RwLock<AppConfig>>,
    pub list_pool: Arc<ListPool>,

    pub open_list: Option<ListLnk>,
    pub found_files: HashMap<PathBuf, Vec<(PathBuf, String)>>,
    pub active_scans: HashSet<PathBuf>,
    pub download_status: HashMap<ProjectLnk, (DownloadStatus, f32)>,

    pub clipboard: Option<ClipboardContent>,

    pub pending_scroll: Option<(ListLnk, ProjectLnk)>,
    pub pending_list_scroll: Option<ListLnk>,
}

impl RDState {
    pub fn new(
        rt_handle: tokio::runtime::Handle,
        modal_manager: SharedModalManager,
        popup_manager: SharedPopupManager,
        notification_manager: SharedNotificationManager,
        api_service: Arc<ApiService>,
        event_rx: mpsc::Receiver<InternalEvent>,
        effect_sx: mpsc::Sender<Effect>,
    ) -> Self {
        let list_pool = Arc::new(ListPool::new(rt_handle.clone(), effect_sx.clone()));

        Self {
            rt_handle,
            event_rx,
            effect_sx,

            api_service,

            modal_manager,
            popup_manager,
            notification_manager,

            loading: true,

            default_dirs: Default::default(),
            config: Arc::new(RwLock::new(AppConfig::default())),
            list_pool,

            open_list: None,
            found_files: Default::default(),
            active_scans: Default::default(),
            download_status: Default::default(),
            clipboard: None,
            pending_scroll: None,
            pending_list_scroll: None,
        }
    }

    pub fn set_clipboard(&mut self, source_list: ListLnk, projects: Vec<ProjectLnk>, is_cut: bool) {
        self.clipboard = Some(ClipboardContent {
            source_list,
            projects,
            is_cut,
        });
    }

    pub fn paste_clipboard(&mut self, target_list_lnk: ListLnk) {
        if let Some(content) = &self.clipboard {
            let source_list_arc = self.list_pool.get(&content.source_list);
            if source_list_arc.is_none() {
                return;
            }
            let source_list = source_list_arc.unwrap();

            let mut projects_to_add = Vec::new();
            {
                let s_list = source_list.read();
                for p_lnk in &content.projects {
                    if let Some(proj) = s_list.get_project(p_lnk) {
                        projects_to_add.push(Project::new_from_existing(proj, false));
                    }
                }
            }

            let is_same_list = content.source_list == target_list_lnk;
            let should_remove_from_source = content.is_cut && !is_same_list;

            if !projects_to_add.is_empty() {
                let projects = projects_to_add;
                self.list_pool.mutate(&target_list_lnk, move |list| {
                    let mut modified = false;
                    let mut versions_to_add = Vec::new();

                    for mut proj in projects {
                        let lnk = proj.get_lnk();
                        if !list.has_project(&lnk) {
                            let version_opt = proj.clear_project_version();
                            let p_lnk = proj.get_lnk();

                            list.add_project(proj);
                            modified = true;

                            if let Some(ver) = version_opt {
                                versions_to_add.push((p_lnk, ver));
                            }
                        }
                    }

                    for (p_lnk, ver) in versions_to_add {
                        list.add_version(&p_lnk, ver);
                    }

                    if modified {
                        MutationResult::new(MutationOutcome::ProjectAdded)
                    } else {
                        MutationResult::unchanged()
                    }
                });

                if should_remove_from_source {
                    let projects_to_remove = content.projects.clone();
                    let src_lnk = content.source_list.clone();
                    self.list_pool.mutate(&src_lnk, move |list| {
                        let mut modified = false;
                        for p_lnk in projects_to_remove {
                            if list.remove_project(&p_lnk).is_success() {
                                modified = true;
                            }
                        }
                        if modified {
                            MutationResult::new(MutationOutcome::ProjectRemoved)
                        } else {
                            MutationResult::unchanged()
                        }
                    });
                }
            }

            if content.is_cut {
                self.clipboard = None;
            }
        }
    }

    pub fn submit_modal(&self, modal: Box<dyn ModalWindow>) {
        self.modal_manager.open(modal);
    }

    pub fn submit_notification(&self, notification: Box<dyn Notification>) {
        self.notification_manager.notify(notification);
    }

    pub fn dispatch(&self, effect: Effect) {
        let sx = self.effect_sx.clone();
        self.rt_handle.spawn(async move {
            let _ = sx.send(effect).await;
        });
    }

    pub fn next_event(&mut self) -> Option<Event> {
        let internal = self.event_rx.try_recv().ok()?;
        match internal {
            InternalEvent::Standard(event) => {
                match event {
                    Event::ArtifactDeleted { .. }
                    | Event::ProjectFileArchived { .. }
                    | Event::ProjectFileUnarchived { .. }
                    | Event::ProjectVersionSelected { .. }
                    | Event::ListSaved { .. } => {
                        self.request_full_refresh();
                    }
                    _ => {}
                }
                Some(event)
            }
            InternalEvent::Initialized {
                config,
                lists,
                default_download_dir_by_type,
            } => {
                *self.config.write() = config.clone();

                let list_lnks: Vec<ListLnk> = lists
                    .into_iter()
                    .map(|(lnk, list)| {
                        self.list_pool.insert_arc(list);
                        lnk
                    })
                    .collect();

                self.default_dirs = default_download_dir_by_type.clone();

                self.set_open_list_no_save(config.last_open_list_id.clone());

                Some(Event::Initialized {
                    config,
                    lists: list_lnks,
                    default_download_dir_by_type,
                })
            }
            InternalEvent::ListCreated {
                name,
                resource_type,
                version,
                loader,
                download_dir,
                projects,
                lnk,
                list,
            } => {
                self.list_pool.insert_arc(list);
                self.request_full_refresh();
                self.pending_list_scroll = Some(lnk.clone());
                Some(Event::ListCreated {
                    name,
                    resource_type,
                    version,
                    loader,
                    download_dir,
                    projects,
                    list: lnk,
                })
            }
            InternalEvent::ListDuplicated {
                list,
                dup_lnk,
                dup_list,
            } => {
                self.list_pool.insert_arc(dup_list);
                self.pending_list_scroll = Some(list.clone());
                Some(Event::ListDuplicated {
                    list,
                    dup_list: dup_lnk,
                })
            }
            InternalEvent::ListDeleted { list } => {
                self.list_pool.remove_sync(&list);
                Some(Event::ListDeleted { list })
            }
            InternalEvent::ListImported {
                list_lnk,
                list,
                path,
            } => {
                self.list_pool.insert_arc(list);
                self.pending_list_scroll = Some(list_lnk.clone());
                Some(Event::ListImported {
                    list: list_lnk,
                    path,
                })
            }
            InternalEvent::LegacyListImported {
                path,
                list,
                list_data,
                version,
                loader,
                download_dir,
                unresolved,
            } => {
                self.list_pool.insert_arc(list_data);
                self.pending_list_scroll = Some(list.clone());
                Some(Event::LegacyListImported {
                    path,
                    version,
                    loader,
                    download_dir,
                    list,
                    unresolved,
                })
            }
            InternalEvent::FilesFound {
                directory,
                file_extension,
                files,
            } => {
                let norm_dir = Self::normalize_path(directory);
                self.active_scans.remove(&norm_dir);
                self.found_files.insert(norm_dir.clone(), files.clone());
                Some(Event::FilesFound {
                    directory: norm_dir,
                    file_extension,
                    files,
                })
            }
            InternalEvent::ProjectVersionSelected {
                list_lnk,
                project,
                version,
                dependency_data,
            } => {
                let version_id = version.version_id.clone();
                let p_lnk_clone = project.clone();
                let l_lnk_clone = list_lnk.clone();

                self.list_pool.mutate(&list_lnk, move |list| {
                    for (p_lnk, rt, meta) in dependency_data {
                        if !list.has_project(&p_lnk) {
                            list.add_project(Project::new(
                                p_lnk.to_context_id().unwrap(),
                                rt,
                                false,
                                meta.name,
                                meta.description,
                                meta.author,
                            ));
                        }
                    }
                    list.add_version(&project, version)
                });

                Some(Event::ProjectVersionSelected {
                    list: l_lnk_clone,
                    project: p_lnk_clone,
                    version_id,
                })
            }
        }
    }

    pub fn api(&self) -> &Arc<ApiService> {
        &self.api_service
    }

    pub fn initialize(&self) {
        self.dispatch(Effect::Initialize);
    }

    pub fn open_explorer(&self, path: PathBuf) {
        self.dispatch(Effect::OpenExplorer { path });
    }

    pub fn set_open_list(&mut self, list: Option<ListLnk>) {
        if self.open_list == list {
            return;
        }

        if let Some(old_list) = self.open_list.clone() {
            self.list_pool.save(&old_list);
        }

        self.open_list = list.clone();
        self.config.write().last_open_list_id = list;
        self.save_config();

        self.found_files.clear();
        self.active_scans.clear();
        self.download_status.clear();
        self.request_full_refresh();
    }

    pub fn set_open_list_no_save(&mut self, list: Option<ListLnk>) {
        if self.open_list == list {
            return;
        }

        self.open_list = list.clone();
        self.config.write().last_open_list_id = list;
        self.save_config();

        self.found_files.clear();
        self.active_scans.clear();
        self.download_status.clear();
        self.request_full_refresh();
    }

    pub fn save_config(&self) {
        let config = self.config.read().clone();
        self.dispatch(Effect::SaveConfig { config });
    }

    pub fn find_files(&mut self, directory: PathBuf, file_extension: String) {
        let norm_dir = Self::normalize_path(directory.clone());
        self.active_scans.insert(norm_dir);
        self.dispatch(Effect::FindFiles {
            directory,
            file_extension: vec![
                file_extension.clone(),
                format!("{}.archive", file_extension),
            ],
        });
    }

    pub fn download_artifact(
        &mut self,
        state_handle: &SharedRDState,
        project: ProjectLnk,
        resource_type: ResourceType,
        version_id: String,
        artifact_id: String,
        target_destination: PathBuf,
    ) {
        self.download_status
            .insert(project.clone(), (DownloadStatus::Queued, 0.0));

        let weak_state = Arc::downgrade(state_handle);
        let p_lnk = project.clone();

        let path = target_destination.parent().unwrap().to_path_buf();
        let ext = resource_type.file_extension();

        let progress_callback: ArtifactCallback = Arc::new(move |status, progress_pct| {
            if let Some(state_arc) = weak_state.upgrade() {
                let mut state = state_arc.write();
                let status_enum = match status {
                    Some(true) => {
                        state.dispatch(Effect::FindFiles {
                            directory: path.clone(),
                            file_extension: vec![ext.clone(), format!("{}.archive", ext)],
                        });
                        DownloadStatus::Complete
                    }
                    Some(false) => DownloadStatus::Failed,
                    None => DownloadStatus::Downloading,
                };
                state
                    .download_status
                    .insert(p_lnk.clone(), (status_enum, progress_pct));
            }
        });

        self.dispatch(Effect::DownloadProjectArtifact {
            project,
            resource_type,
            version_id,
            artifact_id,
            target_destination,
            progress_callback: Some(progress_callback),
        });
    }

    pub fn delete_artifact(&mut self, path: PathBuf, filename: String) {
        self.dispatch(Effect::DeleteArtifact { path, filename });
        self.request_full_refresh();
    }

    pub fn import_modrinth(&self, collection_id: String) {
        self.dispatch(Effect::ImportModrinthCollection { collection_id });
    }

    fn request_full_refresh(&mut self) {
        if self.open_list.is_none() {
            return;
        }
        let list_lnk = self.open_list.as_ref().unwrap();

        if let Some(list_arc) = self.list_pool.get(list_lnk) {
            let list = list_arc.read();
            for rt in list.get_resource_types() {
                if let Some(tc) = list.get_resource_type_config(&rt) {
                    self.find_files(tc.download_dir.clone().into(), rt.file_extension());
                }
            }
        }
    }

    fn normalize_path(path: PathBuf) -> PathBuf {
        let s = path.to_string_lossy().replace('\\', "/");
        PathBuf::from(s)
    }
}
