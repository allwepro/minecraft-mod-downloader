use crate::common::modal_manager::SharedModalManager;
use crate::common::notification_manager::SharedNotificationManager;
use crate::common::pop_up_manager::SharedPopupManager;
use crate::common::prefabs::modal_window::ModalWindow;
use crate::common::prefabs::notification_window::Notification;
use crate::resource_downloader::business::cache::ArtifactCallback;
use crate::resource_downloader::business::list_pool::ListPool;
use crate::resource_downloader::business::services::ApiService;
use crate::resource_downloader::business::{Effect, Event, InternalEvent};
use crate::resource_downloader::domain::{AppConfig, ListLnk, Project, ProjectLnk, ResourceType};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub found_files: Option<Vec<(PathBuf, String)>>,
    pub download_status: HashMap<ProjectLnk, (DownloadStatus, f32)>,
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
            found_files: None,
            download_status: Default::default(),
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
                default_list_name,
                last_open_list_id,
                lists,
                default_download_dir_by_type,
            } => {
                self.config.write().default_list_name = default_list_name.clone();
                self.config.write().last_open_list_id = last_open_list_id.clone();

                self.open_list = last_open_list_id.clone();

                let list_lnks: Vec<ListLnk> = lists
                    .into_iter()
                    .map(|(lnk, list)| {
                        self.list_pool.insert_arc(list);
                        lnk
                    })
                    .collect();

                self.default_dirs = default_download_dir_by_type.clone();

                Some(Event::Initialized {
                    last_open_list_id,
                    default_list_name,
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
                self.found_files = Some(files.clone());
                Some(Event::FilesFound {
                    directory,
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

    pub fn save_config(&self) {
        self.dispatch(Effect::SaveConfig {
            last_open_list_id: self.config.read().last_open_list_id.clone(),
            default_list_name: self.config.read().default_list_name.clone(),
        });
    }

    pub fn find_files(&self, directory: PathBuf, file_extension: String) {
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

    pub fn delete_artifact(&self, path: PathBuf, filename: String) {
        self.dispatch(Effect::DeleteArtifact { path, filename });
        self.request_full_refresh();
    }

    pub fn import_modrinth(&self, collection_id: String) {
        self.dispatch(Effect::ImportModrinthCollection { collection_id });
    }

    fn request_full_refresh(&self) {
        if let Some(list_lnk) = &self.open_list {
            if let Some(list_arc) = self.list_pool.get(list_lnk) {
                let list = list_arc.read();
                for rt in list.get_resource_types() {
                    if let Some(tc) = list.get_resource_type_config(&rt) {
                        self.find_files(tc.download_dir.clone().into(), rt.file_extension());
                    }
                }
            }
        }
    }
}
