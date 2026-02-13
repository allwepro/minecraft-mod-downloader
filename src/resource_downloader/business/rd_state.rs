use crate::common::ui::helper::modal_manager::SharedModalManager;
use crate::common::ui::helper::notification_manager::SharedNotificationManager;
use crate::common::ui::helper::pop_up_manager::SharedPopupManager;
use crate::common::ui::structs::modal_window::ModalWindow;
use crate::common::ui::structs::notification_window::Notification;
use crate::resource_downloader::business::Effect;
use crate::resource_downloader::business::cache::ArtifactCallback;
use crate::resource_downloader::business::list_pool::ListPool;
use crate::resource_downloader::business::services::ApiService;
use crate::resource_downloader::business::{Event, InternalEvent};
use crate::resource_downloader::domain::{
    AppConfig, GameLoader, GameVersion, ListGroupLnk, ListLnk, MutationOutcome, MutationResult,
    Project, ProjectLnk, ResourceType, SidebarItem,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FolderImportCandidate {
    pub original_filename: String,
    pub cleaned_name: String,
    pub detected_version: GameVersion,
    pub detected_loader: GameLoader,
    pub search_results: Option<Vec<(ProjectLnk, String)>>,
}

#[derive(Clone, Debug)]
pub struct FolderImportSession {
    pub path: PathBuf,
    pub resource_type: ResourceType,
    pub candidates: Vec<FolderImportCandidate>,
    pub suggested_version: Option<GameVersion>,
    pub suggested_loader: Option<GameLoader>,
    pub is_scanning: bool,
    pub scan_progress: Option<(usize, usize, String)>,
    pub scan_error: Option<String>,
    pub selected_matches: HashMap<usize, usize>,
    pub skipped_items: HashSet<usize>,
    pub exact_matches: HashSet<usize>,
    pub manually_cleared: HashSet<usize>,
    pub show_only_unresolved: bool,
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

    pub api_service: Arc<ApiService>,

    pub modal_manager: SharedModalManager,
    pub popup_manager: SharedPopupManager,
    pub notification_manager: SharedNotificationManager,

    pub loading: bool,
    pub offline_mode: bool,

    pub default_dirs: HashMap<ResourceType, String>,
    pub config: Arc<RwLock<AppConfig>>,
    pub list_pool: Arc<ListPool>,

    pub open_list: Option<ListLnk>,
    pub open_list_group: Option<ListGroupLnk>,
    pub found_files: HashMap<PathBuf, Vec<(PathBuf, String)>>,
    pub active_scans: HashSet<PathBuf>,
    pub download_status: HashMap<ProjectLnk, (DownloadStatus, f32)>,

    pub clipboard: Option<ClipboardContent>,
    pub folder_import_session: Option<FolderImportSession>,

    pub pending_scroll: Option<(ListLnk, ProjectLnk)>,
    pub pending_sidebar_scroll: Option<SidebarItem>,
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
            offline_mode: false,

            default_dirs: Default::default(),
            config: Arc::new(RwLock::new(AppConfig::default())),
            list_pool,

            open_list: None,
            open_list_group: None,
            found_files: Default::default(),
            active_scans: Default::default(),
            download_status: Default::default(),
            clipboard: None,
            folder_import_session: None,
            pending_scroll: None,
            pending_sidebar_scroll: None,
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
            let target_list_arc = self.list_pool.get(&target_list_lnk);
            if source_list_arc.is_none() || target_list_arc.is_none() {
                return;
            }
            let source_list = source_list_arc.unwrap();
            let target_list = target_list_arc.unwrap();

            let (s_list, t_list) = (source_list.read(), target_list.read());

            let s_list_resource_type = s_list
                .get_resource_types()
                .first()
                .cloned()
                .unwrap_or(ResourceType::Mod);
            let t_list_resource_type = t_list
                .get_resource_types()
                .first()
                .cloned()
                .unwrap_or(ResourceType::Mod);

            if s_list_resource_type != t_list_resource_type {
                return;
            }

            let versions_match = s_list.get_game_version() == t_list.get_game_version()
                && s_list
                    .get_resource_type_config(&s_list_resource_type)
                    .map(|c| &c.loader)
                    == t_list
                        .get_resource_type_config(&t_list_resource_type)
                        .map(|c| &c.loader);

            let mut projects_to_add = Vec::new();
            let mut projects_to_process: Vec<ProjectLnk> = content.projects.clone();
            let mut processed = HashSet::new();
            let explicitly_selected: HashSet<ProjectLnk> =
                content.projects.iter().cloned().collect();

            while let Some(p_lnk) = projects_to_process.pop() {
                if processed.contains(&p_lnk) {
                    continue;
                }
                processed.insert(p_lnk.clone());

                if let Some(proj) = s_list.get_project(&p_lnk) {
                    let is_manual = explicitly_selected.contains(&p_lnk);
                    let mut new_proj = Project::new_from_existing(proj, true);

                    new_proj.set_manual(is_manual);

                    if !versions_match {
                        new_proj.clear_project_version();
                    }

                    projects_to_add.push(new_proj);

                    if let Some(ver) = proj.get_version() {
                        for dep in &ver.depended_on {
                            if !processed.contains(&dep.project) && s_list.has_project(&dep.project)
                            {
                                projects_to_process.push(dep.project.clone());
                            }
                        }
                    }
                }
            }

            drop(s_list);
            drop(t_list);

            let is_same_list = content.source_list == target_list_lnk;
            let should_remove_from_source = content.is_cut && !is_same_list;

            if !projects_to_add.is_empty() {
                let projects = projects_to_add;
                self.list_pool.mutate(&target_list_lnk, move |list| {
                    let mut modified = false;
                    let mut versions_to_add = Vec::new();

                    for proj in projects {
                        let lnk = proj.get_lnk();
                        if !list.has_project(&lnk) {
                            let mut p = proj;
                            let version_opt = p.clear_project_version();
                            let p_lnk = p.get_lnk();

                            list.add_project(p);
                            modified = true;

                            if let Some(ver) = version_opt {
                                versions_to_add.push((p_lnk, ver));
                            }
                        } else if let Some(existing_proj) = list.get_project_mut(&lnk)
                            && proj.is_manual()
                            && !existing_proj.is_manual()
                        {
                            existing_proj.set_manual(true);
                            modified = true;
                        }
                    }

                    for (p_lnk, mut ver) in versions_to_add {
                        ver.depended_on.retain(|dep| list.has_project(&dep.project));
                        list.add_version(&p_lnk, ver);
                    }

                    list.recalculate_dependents();

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
            InternalEvent::ConnectivityChanged { offline } => {
                self.offline_mode = offline;
                None
            }
            InternalEvent::Initialized {
                config,
                lists,
                default_download_dir_by_type,
                offline_mode,
            } => {
                *self.config.write() = config.clone();
                self.offline_mode = offline_mode;

                let list_lnks: Vec<ListLnk> = lists
                    .into_iter()
                    .map(|(lnk, list)| {
                        self.list_pool.insert_arc(list);
                        lnk
                    })
                    .collect();

                self.default_dirs = default_download_dir_by_type.clone();

                if let Some(last_open) = &config.last_open_list_id
                    && list_lnks.contains(last_open)
                {
                    self.set_open_list_no_save(Some(last_open.clone()));
                    self.pending_sidebar_scroll = Some(SidebarItem::from(last_open));
                }

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
                list_lnk,
                list,
            } => {
                self.list_pool.insert_arc(list);
                self.request_full_refresh();

                self.insert_sidebar_item(SidebarItem::List(list_lnk.clone()), false);

                self.pending_sidebar_scroll = Some(SidebarItem::from(&list_lnk));

                Some(Event::ListCreated {
                    name,
                    resource_type,
                    version,
                    loader,
                    download_dir,
                    projects,
                    list: list_lnk,
                })
            }
            InternalEvent::ListDuplicated {
                list_lnk,
                dup_lnk,
                dup_list,
                target_parent,
            } => {
                self.list_pool.insert_arc(dup_list);

                {
                    let mut config = self.config.write();

                    if let Some(pos) = config
                        .sidebar_ui_order
                        .iter()
                        .position(|id| id.match_list(&list_lnk))
                    {
                        config
                            .sidebar_ui_order
                            .insert(pos + 1, SidebarItem::from(&dup_lnk));
                    } else {
                        config
                            .sidebar_ui_order
                            .insert(0, SidebarItem::from(&dup_lnk));
                    }

                    let effective_parent = target_parent
                        .or_else(|| config.list_group_assignments.get(&list_lnk).cloned());

                    if let Some(parent) = effective_parent {
                        config
                            .list_group_assignments
                            .insert(dup_lnk.clone(), parent);
                    }
                }

                self.save_config();
                self.pending_sidebar_scroll = Some(SidebarItem::from(&dup_lnk));

                Some(Event::ListDuplicated {
                    list: list_lnk,
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
                self.request_full_refresh();

                self.insert_sidebar_item(SidebarItem::List(list_lnk.clone()), false);

                self.pending_sidebar_scroll = Some(SidebarItem::from(&list_lnk));

                Some(Event::ListImported {
                    list: list_lnk,
                    path,
                })
            }
            InternalEvent::LegacyListImported {
                path,
                list_lnk,
                list_data,
                version,
                loader,
                download_dir,
                unresolved,
            } => {
                self.list_pool.insert_arc(list_data);
                self.request_full_refresh();

                self.insert_sidebar_item(SidebarItem::List(list_lnk.clone()), false);

                self.pending_sidebar_scroll = Some(SidebarItem::from(&list_lnk));

                Some(Event::LegacyListImported {
                    path,
                    version,
                    loader,
                    download_dir,
                    list: list_lnk,
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
                                p_lnk.to_context_id(),
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
            InternalEvent::FolderImportProgress {
                total,
                current,
                message,
            } => {
                if let Some(session) = &mut self.folder_import_session {
                    session.scan_progress = Some((current, total, message.clone()));
                }
                Some(Event::FolderImportProgress {
                    total,
                    current,
                    message,
                })
            }
            InternalEvent::FolderImportScanned {
                path,
                resource_type,
                candidates,
                suggested_version,
                suggested_loader,
            } => {
                if let Some(session) = &mut self.folder_import_session
                    && session.path == path
                {
                    session.is_scanning = false;
                    session.scan_progress = None;
                    session.candidates = candidates.clone();
                    session.suggested_version = suggested_version.clone();
                    session.suggested_loader = suggested_loader.clone();

                    if let (Some(version), Some(loader)) =
                        (suggested_version.clone(), suggested_loader.clone())
                    {
                        let file_names: Vec<String> =
                            candidates.iter().map(|c| c.cleaned_name.clone()).collect();
                        self.dispatch(Effect::SearchFolderImportCandidates {
                            file_names,
                            resource_type,
                            version,
                            loader,
                        });
                    }

                    let file_names: Vec<String> =
                        candidates.iter().map(|c| c.cleaned_name.clone()).collect();
                    self.dispatch(Effect::SearchFolderImportCandidates {
                        file_names,
                        resource_type,
                        version: suggested_version.clone().unwrap_or(
                            self.api()
                                .game_version_pool
                                .get_versions()
                                .unwrap()
                                .unwrap()[0]
                                .clone(),
                        ),
                        loader: suggested_loader.clone().unwrap_or(
                            self.api()
                                .game_loader_pool
                                .get_loaders(resource_type)
                                .unwrap()
                                .unwrap()[0]
                                .clone(),
                        ),
                    });
                }
                Some(Event::FolderImportScanned {
                    path,
                    resource_type,
                    candidates,
                    suggested_version,
                    suggested_loader,
                })
            }
            InternalEvent::FailedFolderScan {
                path,
                resource_type,
                error,
            } => {
                if let Some(session) = &mut self.folder_import_session {
                    session.is_scanning = false;
                    session.scan_error = Some(error.clone());
                    session.scan_progress = None;
                }
                Some(Event::FailedFolderScan {
                    path,
                    resource_type,
                    error,
                })
            }
            InternalEvent::FolderImportCandidatesFound { results } => {
                if let Some(session) = &mut self.folder_import_session {
                    for candidate in &mut session.candidates {
                        if let Some(res) = results.get(&candidate.cleaned_name) {
                            candidate.search_results = Some(res.clone());
                        }
                    }
                }
                Some(Event::FolderImportCandidatesFound { results })
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

        self.request_full_refresh();
    }

    pub fn set_open_list_no_save(&mut self, list: Option<ListLnk>) {
        if self.open_list == list {
            return;
        }

        self.open_list = list.clone();
        self.config.write().last_open_list_id = list;
        self.save_config();

        self.request_full_refresh();
    }

    pub fn set_open_list_group(&mut self, list_group: Option<ListGroupLnk>) {
        if self.open_list_group == list_group {
            return;
        }

        if list_group.is_some() {
            if let Some(old_list) = self.open_list.clone() {
                self.list_pool.save(&old_list);
            }
            self.open_list = None;
            self.config.write().last_open_list_id = None;
        }

        self.open_list_group = list_group;
        self.save_config();
        self.request_full_refresh();
    }

    pub fn insert_sidebar_item(&self, new_item: SidebarItem, _is_duplication: bool) {
        {
            let mut config = self.config.write();
            let mut inserted = false;

            let context_group = self.open_list_group.clone().or_else(|| {
                self.open_list
                    .as_ref()
                    .and_then(|l| config.list_group_assignments.get(l))
                    .cloned()
            });

            if let Some(lg_lnk) = context_group {
                match &new_item {
                    SidebarItem::List(l_lnk) => {
                        config
                            .list_group_assignments
                            .insert(l_lnk.clone(), lg_lnk.clone());
                    }
                    SidebarItem::ListGroup(new_lg_lnk) => {
                        if let Some(new_lg) =
                            config.list_groups.iter_mut().find(|f| f.lnk == *new_lg_lnk)
                        {
                            new_lg.parent_id = Some(lg_lnk.clone());
                        }
                    }
                }

                let first_child_pos = config.sidebar_ui_order.iter().position(|item| match item {
                    SidebarItem::List(l) => config.list_group_assignments.get(l) == Some(&lg_lnk),
                    SidebarItem::ListGroup(g) => {
                        config
                            .list_groups
                            .iter()
                            .find(|lg| lg.lnk == *g)
                            .and_then(|lg| lg.parent_id.as_ref())
                            == Some(&lg_lnk)
                    }
                });

                if let Some(first_pos) = first_child_pos {
                    config.sidebar_ui_order.insert(first_pos, new_item.clone());
                    inserted = true;
                } else if let Some(group_pos) = config
                    .sidebar_ui_order
                    .iter()
                    .position(|i| i.match_list_group(&lg_lnk))
                {
                    config
                        .sidebar_ui_order
                        .insert(group_pos + 1, new_item.clone());
                    inserted = true;
                }
            }

            if !inserted {
                config.sidebar_ui_order.insert(0, new_item);
            }
        }

        self.save_config();
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

    pub fn request_full_refresh(&mut self) {
        self.found_files.clear();
        self.active_scans.clear();
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

    pub fn start_folder_import(&mut self, path: PathBuf, resource_type: Option<ResourceType>) {
        let rt = resource_type.unwrap_or(ResourceType::Mod);
        self.folder_import_session = Some(FolderImportSession {
            path: path.clone(),
            resource_type: rt,
            candidates: Vec::new(),
            suggested_version: None,
            suggested_loader: None,
            is_scanning: resource_type.is_some(),
            scan_progress: None,
            scan_error: None,
            selected_matches: HashMap::new(),
            skipped_items: HashSet::new(),
            exact_matches: HashSet::new(),
            manually_cleared: HashSet::new(),
            show_only_unresolved: false,
        });
        if resource_type.is_some() {
            self.dispatch(Effect::ScanFolderImport {
                path,
                resource_type: rt,
            });
        }
    }

    pub fn cancel_folder_import(&mut self) {
        self.folder_import_session = None;
    }

    pub fn create_import_folder_list(
        &self,
        name: String,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        projects: Vec<ProjectLnk>,
    ) {
        self.dispatch(Effect::ImportFolderList {
            name,
            resource_type,
            version,
            loader,
            download_dir,
            projects,
        });
    }

    fn normalize_path(path: PathBuf) -> PathBuf {
        let s = path.to_string_lossy().replace('\\', "/");
        PathBuf::from(s)
    }
}
