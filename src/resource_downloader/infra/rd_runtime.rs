use crate::resource_downloader::business::InternalEvent;
use crate::resource_downloader::business::cache::ArtifactRequest;
use crate::resource_downloader::business::services::ApiService;
use crate::resource_downloader::business::{Effect, Event};
use crate::resource_downloader::domain::{
    AppConfig, GameVersion, Project, ProjectDependency, ProjectList, ProjectLnk, ProjectTypeConfig,
    ProjectVersion, RESOURCE_TYPES, ResourceType,
};
use crate::resource_downloader::infra::{
    ConfigManager, GameDetection, LegacyListService, ListFileManager,
};
use parking_lot::RwLock;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

pub type AsyncRunFn = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub struct RDRuntime {
    rt_handle: tokio::runtime::Handle,
    game_detection: Arc<GameDetection>,
    config_manager: Arc<ConfigManager>,
    list_manager: Arc<ListFileManager>,
    api_service: Arc<ApiService>,
    legacy_list_manager: Arc<LegacyListService>,
    effect_rx: mpsc::Receiver<Effect>,
    event_tx: mpsc::Sender<InternalEvent>,
}

impl RDRuntime {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        rt_handle: tokio::runtime::Handle,
        api_service: Arc<ApiService>,
        effect_rx: mpsc::Receiver<Effect>,
        event_tx: mpsc::Sender<InternalEvent>,
        game_detection: Arc<GameDetection>,
        config_manager: Arc<ConfigManager>,
        list_manager: Arc<ListFileManager>,
        legacy_list_manager: Arc<LegacyListService>,
    ) -> AsyncRunFn {
        let mut runtime = Self {
            rt_handle,
            game_detection,
            config_manager,
            list_manager,
            api_service,
            legacy_list_manager,
            effect_rx,
            event_tx,
        };

        Box::pin(async move {
            runtime.run().await;
        })
    }

    pub async fn run(&mut self) {
        while let Some(effect) = self.effect_rx.recv().await {
            self.handle_effect(effect).await;
        }
    }

    async fn handle_effect(&self, effect: Effect) {
        let api = self.api_service.clone();
        let cm = self.config_manager.clone();
        let lm = self.list_manager.clone();
        let legacy = self.legacy_list_manager.clone();
        let gd = self.game_detection.clone();
        let tx = self.event_tx.clone();
        match effect {
            Effect::Initialize => {
                self.rt_handle.spawn(async move {
                    if let Err(e) = cm.init().await {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::FailedInitialization {
                                error: e.to_string(),
                            }))
                            .await;
                        return;
                    }
                    if let Err(e) = lm.init().await {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::FailedInitialization {
                                error: e.to_string(),
                            }))
                            .await;
                        return;
                    }

                    let config = cm.load_config().await.unwrap_or_default();
                    let raw_lists: Vec<ProjectList> = lm.load_all().await.unwrap_or_default();

                    let mut lists_with_lnks = Vec::new();
                    for list in raw_lists {
                        let lnk = list.get_lnk();
                        lists_with_lnks.push((lnk, Arc::new(RwLock::new(list))));
                    }

                    let _ = api.game_version_pool.get_versions_blocking().await;

                    for rt in RESOURCE_TYPES {
                        let _ = api.game_loader_pool.get_loaders_blocking(rt).await;
                    }

                    let mut default_download_dir_by_type = HashMap::new();
                    for rt in RESOURCE_TYPES {
                        default_download_dir_by_type.insert(
                            rt,
                            gd.get_default_minecraft_download_dir(rt)
                                .to_str()
                                .unwrap()
                                .to_string(),
                        );
                    }

                    let _ = tx
                        .send(InternalEvent::Initialized {
                            last_open_list_id: config.last_open_list_id,
                            default_list_name: config.default_list_name,
                            lists: lists_with_lnks,
                            default_download_dir_by_type,
                        })
                        .await;
                });
            }

            Effect::SaveConfig {
                last_open_list_id,
                default_list_name,
            } => {
                self.rt_handle.spawn(async move {
                    let _ = cm
                        .save_config(&AppConfig {
                            last_open_list_id,
                            default_list_name,
                        })
                        .await;
                });
            }

            Effect::CreateList {
                name,
                resource_type,
                version,
                loader,
                download_dir,
                projects,
            } => {
                self.rt_handle.spawn(async move {
                    let new_id = ProjectList::generate_id();
                    let mut new_list =
                        ProjectList::new(new_id.clone(), name.clone(), version.clone());
                    new_list.set_resource_type(
                        resource_type,
                        ProjectTypeConfig::new(loader.clone(), download_dir.clone()),
                    );

                    for project_id in projects.clone() {
                        if let Some(rtpm) = api
                            .rt_project_pool
                            .get_metadata_blocking(ProjectLnk::from(&project_id), resource_type)
                            .await
                            .unwrap_or(None)
                        {
                            new_list.add_project(Project::new(
                                project_id,
                                resource_type,
                                true,
                                rtpm.name,
                                rtpm.description,
                                rtpm.author,
                            ));
                        }
                    }

                    match lm.save(&new_list).await {
                        Ok(_) => {
                            let list_arc = Arc::new(RwLock::new(new_list));
                            let lnk = list_arc.read().get_lnk();
                            let _ = tx
                                .send(InternalEvent::ListCreated {
                                    name,
                                    resource_type,
                                    version,
                                    loader,
                                    download_dir,
                                    projects,
                                    lnk,
                                    list: list_arc,
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(InternalEvent::Standard(Event::FailedListCreation {
                                    name,
                                    resource_type,
                                    version,
                                    loader,
                                    download_dir,
                                    projects,
                                    error: e.to_string(),
                                }))
                                .await;
                        }
                    }
                });
            }

            Effect::SaveList { list } => {
                self.rt_handle.spawn(async move {
                    let (lnk, content) = {
                        let guard = list.read();
                        (guard.get_lnk(), toml::to_string_pretty(&*guard).unwrap())
                    };
                    if let Err(e) = lm.save_raw(&lnk, content).await {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::FailedListSave {
                                list: lnk,
                                error: e.to_string(),
                            }))
                            .await;
                    } else {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::ListSaved { list: lnk }))
                            .await;
                    }
                });
            }

            Effect::DuplicateList { list } => {
                self.rt_handle.spawn(async move {
                    let (lnk, content) = {
                        let guard = list.read();
                        (guard.get_lnk(), toml::to_string_pretty(&*guard).unwrap())
                    };

                    let _ = lm.save_raw(&lnk, content).await;

                    let (dup_lnk, dup_arc) = {
                        let guard = lm.copy(&lnk).await.unwrap();
                        (guard.get_lnk(), Arc::new(RwLock::new(guard)))
                    };

                    let _ = tx
                        .send(InternalEvent::ListDuplicated {
                            list: lnk,
                            dup_lnk,
                            dup_list: dup_arc,
                        })
                        .await;
                });
            }

            Effect::DeleteList { list } => {
                self.rt_handle.spawn(async move {
                    match lm.delete(&list).await {
                        Ok(_) => {
                            let _ = tx.send(InternalEvent::ListDeleted { list }).await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(InternalEvent::Standard(Event::FailedListDelete {
                                    list,
                                    error: e.to_string(),
                                }))
                                .await;
                        }
                    }
                });
            }

            Effect::ImportList { path } => {
                self.rt_handle.spawn(async move {
                    match lm.import_from_file(path.clone()).await {
                        Ok(list) => {
                            let lnk = list.get_lnk();
                            let shared = Arc::new(RwLock::new(list));
                            let _ = tx
                                .send(InternalEvent::ListImported {
                                    list_lnk: lnk,
                                    list: shared,
                                    path,
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(InternalEvent::Standard(Event::FailedListImport {
                                    path,
                                    error: e.to_string(),
                                }))
                                .await;
                        }
                    }
                });
            }

            Effect::ExportList { list, path } => {
                self.rt_handle.spawn(async move {
                    let (lnk, content) = {
                        let guard = list.read();
                        (guard.get_lnk(), toml::to_string_pretty(&*guard).unwrap())
                    };
                    let _ = lm.save_raw(&lnk, content).await;
                    if let Err(e) = lm.export_to_file(&lnk, path.clone()).await {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::FailedListExport {
                                list: lnk,
                                error: e.to_string(),
                            }))
                            .await;
                    } else {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::ListExported {
                                list: lnk,
                                path,
                            }))
                            .await;
                    }
                });
            }

            Effect::SelectProjectVersion {
                list,
                project,
                version_id,
            } => {
                self.rt_handle.spawn(async move {
                    let req = {
                        let guard = list.read();
                        let rt = guard
                            .get_project(&project)
                            .map(|p| p.resource_type)
                            .unwrap_or(ResourceType::Mod);
                        let config = guard.get_resource_type_config(&rt);
                        config.map(|c| {
                            (
                                guard.get_lnk(),
                                rt,
                                guard.get_game_version(),
                                c.loader.clone(),
                            )
                        })
                    };

                    let (lnk, rt, gv, loader) = match req {
                        Some(val) => val,
                        None => {
                            let lnk = list.read().get_lnk();
                            let _ = tx
                                .send(InternalEvent::Standard(Event::FailedProjectVersionSelect {
                                    list: lnk,
                                    project,
                                    version_id,
                                    error: "Missing type config".into(),
                                }))
                                .await;
                            return;
                        }
                    };

                    match api
                        .rt_project_pool
                        .get_versions_blocking(project.clone(), rt, gv, loader)
                        .await
                    {
                        Ok(Some(rt_versions)) => {
                            if let Some(target) =
                                rt_versions.into_iter().find(|v| v.version_id == version_id)
                            {
                                let mut dependency_data = Vec::new();
                                for prj in target.depended_on.iter().map(|d| d.project.clone()) {
                                    if list.read().has_project(&prj) {
                                        continue;
                                    }
                                    if let Some(rtpm) = api
                                        .rt_project_pool
                                        .get_metadata_blocking(prj.clone(), rt)
                                        .await
                                        .unwrap_or(None)
                                    {
                                        dependency_data.push((prj, rt, rtpm));
                                    }
                                }

                                let domain_v = ProjectVersion::new(
                                    true,
                                    target.version_id,
                                    target.artifact_id,
                                    target.artifact_hash,
                                    target.channel,
                                    target
                                        .depended_on
                                        .into_iter()
                                        .map(|d| {
                                            ProjectDependency::new(
                                                d.project,
                                                d.dependency_type,
                                                None,
                                                d.version_id,
                                            )
                                        })
                                        .collect(),
                                );

                                let _ = tx
                                    .send(InternalEvent::ProjectVersionSelected {
                                        list_lnk: lnk,
                                        project,
                                        version: domain_v,
                                        dependency_data,
                                    })
                                    .await;
                            }
                        }
                        _ => {
                            let _ = tx
                                .send(InternalEvent::Standard(Event::FailedProjectVersionSelect {
                                    list: lnk,
                                    project,
                                    version_id,
                                    error: "API Failure".into(),
                                }))
                                .await;
                        }
                    }
                });
            }

            Effect::FindFiles {
                directory,
                file_extension,
            } => {
                self.rt_handle.spawn(async move {
                    use std::fmt::Write as _;
                    let mut files = Vec::new();
                    if let Ok(mut dir) = tokio::fs::read_dir(&directory).await {
                        while let Ok(Some(entry)) = dir.next_entry().await {
                            let path = entry.path();

                            if let Some(ext) = path.extension().and_then(|s| s.to_str())
                                && file_extension.contains(&ext.to_string())
                            {
                                let hash_result: anyhow::Result<String> = async {
                                    let mut file = tokio::fs::File::open(&path).await?;
                                    let mut hasher = Sha1::new();
                                    let mut buffer = [0u8; 8192];

                                    loop {
                                        let n = file.read(&mut buffer).await?;
                                        if n == 0 {
                                            break;
                                        }
                                        hasher.update(&buffer[..n]);
                                    }

                                    let result = hasher.finalize();

                                    let mut s = String::with_capacity(40);
                                    for byte in result {
                                        write!(s, "{byte:02x}").expect("String write failed");
                                    }
                                    Ok(s)
                                }
                                .await;

                                if let Ok(sha1_hash) = hash_result {
                                    files.push((path, sha1_hash));
                                }
                            }
                        }
                    }
                    let _ = tx
                        .send(InternalEvent::FilesFound {
                            directory,
                            file_extension,
                            files,
                        })
                        .await;
                });
            }

            Effect::DownloadProjectArtifact {
                project,
                resource_type,
                version_id,
                artifact_id,
                target_destination,
                progress_callback,
            } => {
                let art_cache = Arc::clone(&api.artifact_cache);
                self.rt_handle.spawn(async move {
                    art_cache.queue_download(ArtifactRequest {
                        project,
                        resource_type,
                        version_id,
                        artifact_id,
                        target_destination,
                        progress_callback,
                    });
                });
            }

            Effect::ArchiveProjectFile { path, filename } => {
                self.rt_handle.spawn(async move {
                    let src = path.join(&filename);
                    let dest = path.join(format!("{filename}.archive"));

                    if !src.exists() && dest.exists() {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::FailedProjectFileArchive {
                                path,
                                error: format!("Failed to archive {filename}: Already archived"),
                                filename,
                            }))
                            .await;
                        return;
                    }

                    if let Err(e) = tokio::fs::rename(&src, dest).await {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::FailedProjectFileArchive {
                                path,
                                error: format!("Failed to archive {filename}: {e}"),
                                filename,
                            }))
                            .await;
                    } else {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::ProjectFileArchived {
                                path,
                                filename,
                            }))
                            .await;
                    }
                });
            }

            Effect::UnarchiveProjectFile { path, filename } => {
                self.rt_handle.spawn(async move {
                    let src = path.join(format!("{filename}.archive"));
                    let dest = path.join(&filename);

                    if !src.exists() && dest.exists() {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::FailedProjectFileArchive {
                                path,
                                error: format!(
                                    "Failed to unarchive {filename}: Already unarchived"
                                ),
                                filename,
                            }))
                            .await;
                        return;
                    }

                    if let Err(e) = tokio::fs::rename(&src, &dest).await {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::FailedProjectFileUnarchive {
                                path,
                                error: format!("Failed to unarchive {filename}: {e}"),
                                filename,
                            }))
                            .await;
                    } else {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::ProjectFileUnarchived {
                                path,
                                filename,
                            }))
                            .await;
                    }
                });
            }

            Effect::DeleteArtifact { path, filename } => {
                self.rt_handle.spawn(async move {
                    let full_path = path.join(&filename);
                    if let Err(e) = tokio::fs::remove_file(&full_path).await {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::FailedArtifactDelete {
                                path: full_path,
                                error: format!("Failed to delete {filename}: {e}"),
                                filename,
                            }))
                            .await;
                    } else {
                        let _ = tx
                            .send(InternalEvent::Standard(Event::ArtifactDeleted {
                                path: full_path,
                                filename,
                            }))
                            .await;
                    }
                });
            }
            Effect::OpenExplorer { path } => {
                self.rt_handle.spawn(async move {
                    #[cfg(target_os = "windows")]
                    {
                        let _ = Command::new("explorer").arg(&path).spawn();
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let _ = Command::new("open").arg(&path).spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = Command::new("xdg-open").arg(&path).spawn();
                    }
                });
            }
            Effect::ImportLegacyList {
                path,
                version,
                loader,
                download_dir,
            } => {
                self.rt_handle.spawn(async move {
                    match legacy
                        .import_legacy_list(
                            path.clone(),
                            &version,
                            &loader,
                            download_dir.clone(),
                            tx.clone(),
                        )
                        .await
                    {
                        Ok(new_list) => {
                            if let Err(e) = lm.save(&new_list).await {
                                let _ = tx
                                    .send(InternalEvent::Standard(Event::FailedLegacyListImport {
                                        path,
                                        version,
                                        loader,
                                        error: e.to_string(),
                                    }))
                                    .await;
                                return;
                            }

                            let lnk = new_list.get_lnk();
                            let shared = Arc::new(RwLock::new(new_list));

                            let _ = tx
                                .send(InternalEvent::LegacyListImported {
                                    path,
                                    list: lnk,
                                    list_data: shared,
                                    version,
                                    loader,
                                    download_dir,
                                    unresolved: vec![],
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(InternalEvent::Standard(Event::FailedLegacyListImport {
                                    path,
                                    version,
                                    loader,
                                    error: e.to_string(),
                                }))
                                .await;
                        }
                    }
                });
            }

            Effect::ExportLegacyList {
                list,
                version,
                loader,
                path,
            } => {
                self.rt_handle.spawn(async move {
                    let lnk = list.read().get_lnk();
                    match legacy
                        .export_legacy_list(path.clone(), list, tx.clone())
                        .await
                    {
                        Ok(unresolved) => {
                            let _ = tx
                                .send(InternalEvent::Standard(Event::LegacyListExported {
                                    list: lnk,
                                    path,
                                    version,
                                    loader,
                                    unresolved,
                                }))
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(InternalEvent::Standard(Event::FailedLegacyListExport {
                                    list: lnk,
                                    version,
                                    loader,
                                    error: e.to_string(),
                                }))
                                .await;
                        }
                    }
                });
            }

            Effect::ImportModrinthCollection { collection_id } => {
                self.rt_handle.spawn(async move {
                    match api.fetch_modrinth_collection(collection_id.clone()).await {
                        Ok((_, meta, projects)) => {
                            let mut contained_resource_ids = HashMap::new();
                            for (p_id, _, rt) in projects {
                                if let Some((ver_name, loader)) = meta.get(&rt) {
                                    let entry = contained_resource_ids.entry(rt).or_insert((
                                        GameVersion::from(ver_name),
                                        loader.clone(),
                                        Vec::new(),
                                    ));
                                    entry.2.push(p_id);
                                }
                            }
                            let _ = tx
                                .send(InternalEvent::Standard(Event::ModrinthCollectionImported {
                                    collection_id,
                                    contained_resource_ids,
                                }))
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(InternalEvent::Standard(
                                    Event::FailedModrinthCollectionImport {
                                        collection_id,
                                        error: e.to_string(),
                                    },
                                ))
                                .await;
                        }
                    }
                });
            }
        }
    }
}
