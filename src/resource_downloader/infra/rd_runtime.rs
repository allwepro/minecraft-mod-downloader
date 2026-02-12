use crate::resource_downloader::business::cache::ArtifactRequest;
use crate::resource_downloader::business::services::ApiService;
use crate::resource_downloader::business::{Effect, Event};
use crate::resource_downloader::business::{FolderImportCandidate, InternalEvent};
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, Project, ProjectDependency, ProjectList, ProjectLnk,
    ProjectTypeConfig, ProjectVersion, RESOURCE_TYPES, ResourceType,
};
use crate::resource_downloader::infra::cache::file_index;
use crate::resource_downloader::infra::cache::file_index::{FileIndexCache, FileIndexEntry};
use crate::resource_downloader::infra::{
    ConfigManager, GameDetection, LegacyListService, ListFileManager, ResourceDetector,
};
use parking_lot::RwLock;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
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
    file_index_cache: Arc<RwLock<FileIndexCache>>,
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
            file_index_cache: Arc::new(RwLock::new(FileIndexCache::default())),
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

    async fn hash_file(path: std::path::PathBuf) -> anyhow::Result<String> {
        tokio::task::spawn_blocking(move || {
            use std::fmt::Write as _;
            use std::io::Read;
            let mut file = std::fs::File::open(&path)?;
            let mut hasher = Sha1::new();
            let mut buffer = [0u8; 8192];

            loop {
                let n = file.read(&mut buffer)?;
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
        })
        .await?
    }

    async fn handle_effect(&self, effect: Effect) {
        let api = self.api_service.clone();
        let cm = self.config_manager.clone();
        let lm = self.list_manager.clone();
        let legacy = self.legacy_list_manager.clone();
        let gd = self.game_detection.clone();
        let tx = self.event_tx.clone();
        let fic = self.file_index_cache.clone();
        match effect {
            Effect::Initialize => {
                self.rt_handle.spawn(async move {
                    let status_ping = api.ping();

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

                    let cache_path = cm.get_cache_dir().join("file_index.json");
                    if let Ok(loaded_cache) = FileIndexCache::load(&cache_path).await {
                        let mut cache = fic.write();
                        *cache = loaded_cache;
                    }

                    let config = cm.load_config().await.unwrap_or_default();

                    let mut lists_with_lnks = Vec::new();
                    let available_lnks = lm.get_available_lists().await;
                    let mut join_set = tokio::task::JoinSet::new();

                    for lnk in available_lnks {
                        let lm_clone = lm.clone();
                        join_set.spawn(async move {
                            let list = lm_clone.load(&lnk).await?;
                            Ok::<(ListLnk, Arc<RwLock<ProjectList>>), anyhow::Error>((
                                lnk,
                                Arc::new(RwLock::new(list)),
                            ))
                        });
                    }

                    while let Some(res) = join_set.join_next().await {
                        if let Ok(Ok(list_entry)) = res {
                            lists_with_lnks.push(list_entry);
                        }
                    }

                    let mut join_set2: tokio::task::JoinSet<anyhow::Result<DefaultReason>> =
                        tokio::task::JoinSet::new();
                    struct DefaultReason;

                    let api_v = api.clone();
                    join_set2.spawn(async move {
                        api_v.game_version_pool.get_versions_blocking().await?;
                        Ok(DefaultReason)
                    });

                    for rt in RESOURCE_TYPES {
                        let api_l = api.clone();
                        join_set2.spawn(async move {
                            api_l.game_loader_pool.get_loaders_blocking(rt).await?;
                            Ok(DefaultReason)
                        });
                    }

                    while join_set2.join_next().await.is_some() {}

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

                    lm.set_default_download_dirs(default_download_dir_by_type.clone())
                        .await;

                    let offline_mode = status_ping.await.is_err();
                    let _ = tx
                        .send(InternalEvent::Initialized {
                            config: config.clone(),
                            lists: lists_with_lnks,
                            default_download_dir_by_type,
                            offline_mode,
                        })
                        .await;

                    let tx_connectivity = tx.clone();
                    let api_connectivity = api.clone();
                    tokio::spawn(async move {
                        let mut last_status = !offline_mode;
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                            let status = api_connectivity.ping().await.is_ok();
                            if status != last_status {
                                last_status = status;
                                let _ = tx_connectivity
                                    .send(InternalEvent::ConnectivityChanged { offline: !status })
                                    .await;
                            }
                        }
                    });
                });
            }

            Effect::SaveConfig { config } => {
                self.rt_handle.spawn(async move {
                    let _ = cm.save_config(&config).await;
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
                        .get_versions_best_blocking(project.clone(), rt, gv, loader)
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
                                    if let Ok(Some(rtpm)) = api
                                        .rt_project_pool
                                        .get_metadata_blocking(prj.clone(), rt)
                                        .await
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
                let fic = self.file_index_cache.clone();
                let cm = self.config_manager.clone();
                let tx = self.event_tx.clone();
                self.rt_handle.spawn(async move {
                    let mut files = Vec::new();
                    let mut files_to_hash = Vec::new();

                    let mut metadata_join_set: tokio::task::JoinSet<
                        anyhow::Result<Option<(std::path::PathBuf, u64, u64)>>,
                    > = tokio::task::JoinSet::new();

                    if let Ok(mut dir) = tokio::fs::read_dir(&directory).await {
                        while let Ok(Some(entry)) = dir.next_entry().await {
                            let path = entry.path();
                            let ext_list = file_extension.clone();

                            metadata_join_set.spawn(async move {
                                let file_name = path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or_default();
                                let mut matches = false;
                                for target_ext in &ext_list {
                                    if file_name.ends_with(target_ext) {
                                        matches = true;
                                        break;
                                    }
                                }

                                if !matches {
                                    return Ok(None);
                                }

                                let metadata = tokio::fs::metadata(&path).await?;
                                if !metadata.is_file() {
                                    return Ok(None);
                                }

                                let size = metadata.len();
                                let modified = file_index::get_system_time_secs(
                                    metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
                                );

                                Ok(Some((path, size, modified)))
                            });
                        }
                    }

                    let mut scan_results = Vec::new();
                    while let Some(res) = metadata_join_set.join_next().await {
                        if let Ok(Ok(Some(item))) = res {
                            scan_results.push(item);
                        }
                    }

                    {
                        let cache = fic.read();
                        for (path, size, modified) in scan_results {
                            let cached_hash = cache.get(&path).and_then(|e| {
                                if e.size == size && e.modified == modified {
                                    Some(e.hash.clone())
                                } else {
                                    None
                                }
                            });

                            if let Some(h) = cached_hash {
                                files.push((path, h));
                            } else {
                                files.push((path.clone(), String::new()));
                                files_to_hash.push((path, size, modified));
                            }
                        }
                    }

                    let _ = tx
                        .send(InternalEvent::FilesFound {
                            directory: directory.clone(),
                            file_extension: file_extension.clone(),
                            files: files.clone(),
                        })
                        .await;

                    if files_to_hash.is_empty() {
                        return;
                    }

                    let mut hashing_join_set: tokio::task::JoinSet<
                        anyhow::Result<(std::path::PathBuf, String, u64, u64)>,
                    > = tokio::task::JoinSet::new();

                    let hashing_semaphore = Arc::new(tokio::sync::Semaphore::new(4));

                    for (path, size, modified) in files_to_hash {
                        let path_clone = path.clone();
                        let sem_clone = hashing_semaphore.clone();
                        hashing_join_set.spawn(async move {
                            let _permit = sem_clone.acquire().await;
                            let sha1_hash = RDRuntime::hash_file(path_clone).await?;
                            Ok((path, sha1_hash, size, modified))
                        });
                    }

                    let mut cache_changed = false;
                    let mut files_updated = 0;

                    while let Some(res) = hashing_join_set.join_next().await {
                        if let Ok(Ok((p, h, s, m))) = res {
                            if let Some(entry) = files.iter_mut().find(|(path, _)| *path == p) {
                                entry.1 = h.clone();
                            }
                            {
                                let mut cache = fic.write();
                                cache.insert(
                                    p,
                                    FileIndexEntry {
                                        hash: h,
                                        size: s,
                                        modified: m,
                                    },
                                );
                            }
                            cache_changed = true;
                            files_updated += 1;

                            if files_updated % 10 == 0 {
                                let _ = tx
                                    .send(InternalEvent::FilesFound {
                                        directory: directory.clone(),
                                        file_extension: file_extension.clone(),
                                        files: files.clone(),
                                    })
                                    .await;
                            }
                        }
                    }

                    if cache_changed {
                        let cache_to_save = fic.read().clone();
                        let cache_path = cm.get_cache_dir().join("file_index.json");
                        let _ = cache_to_save.save(&cache_path).await;
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
                list_name,
                version,
                loader,
                download_dir,
            } => {
                self.rt_handle.spawn(async move {
                    match legacy
                        .import_legacy_list(
                            path.clone(),
                            list_name.clone(),
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

            Effect::ScanFolderImport {
                path,
                resource_type,
            } => {
                let api = api.clone();
                let tx = self.event_tx.clone();

                let _ = tx
                    .send(InternalEvent::FolderImportProgress {
                        total: 4,
                        current: 1,
                        message: "Scanning folder".to_string(),
                    })
                    .await;

                self.rt_handle.spawn(async move {
                    // 1. Fetch available versions and loaders
                    let versions_res = api.game_version_pool.get_versions_blocking().await;
                    let loaders_res = api
                        .game_loader_pool
                        .get_loaders_blocking(resource_type)
                        .await;

                    let (versions, loaders) = match (versions_res, loaders_res) {
                        (Ok(Some(v)), Ok(Some(l))) => (v, l),
                        _ => {
                            let _ = tx
                                .send(InternalEvent::FailedFolderScan {
                                    path,
                                    resource_type,
                                    error: "Failed to fetch versions/loaders".into(),
                                })
                                .await;
                            return;
                        }
                    };

                    // 2. Detect resources
                    let detector = ResourceDetector;
                    let v_vec: Vec<GameVersion> = versions.to_vec();
                    let l_vec: Vec<GameLoader> = loaders.to_vec();

                    let _ = tx
                        .send(InternalEvent::FolderImportProgress {
                            total: 4,
                            current: 2,
                            message: "Parsing files".to_string(),
                        })
                        .await;
                    let (results, best_ver, best_loader) = detector.detect_resources_from_dir(
                        path.clone(),
                        resource_type,
                        l_vec,
                        v_vec,
                    );

                    let _ = tx
                        .send(InternalEvent::FolderImportProgress {
                            total: 4,
                            current: 3,
                            message: "Converting data".to_string(),
                        })
                        .await;
                    let mut candidates = Vec::new();
                    for (filename, cleaned_name, ver, loader) in results {
                        candidates.push(FolderImportCandidate {
                            original_filename: filename,
                            cleaned_name,
                            detected_version: ver,
                            detected_loader: loader,
                            search_results: None,
                        });
                    }

                    let _ = tx
                        .send(InternalEvent::FolderImportProgress {
                            total: 4,
                            current: 4,
                            message: "Finalizing data".to_string(),
                        })
                        .await;
                    let _ = tx
                        .send(InternalEvent::FolderImportScanned {
                            path,
                            resource_type,
                            candidates,
                            suggested_version: best_ver,
                            suggested_loader: best_loader,
                        })
                        .await;
                });
            }

            Effect::SearchFolderImportCandidates {
                file_names,
                resource_type,
                version,
                loader,
            } => {
                let api = api.clone();
                let tx = self.event_tx.clone();

                self.rt_handle.spawn(async move {
                    let mut results_map = HashMap::new();

                    for cleaned_name in file_names.into_iter() {
                        let search_res = api
                            .rt_project_pool
                            .search_blocking(
                                cleaned_name.clone(),
                                resource_type,
                                Some(version.clone()),
                                Some(loader.clone()),
                            )
                            .await;

                        let matches_with_names: Vec<(ProjectLnk, String)> = match search_res {
                            Ok(Some(results)) => {
                                let mut named_results = Vec::new();
                                for proj_lnk in results {
                                    if let Ok(Some(meta)) = api
                                        .rt_project_pool
                                        .get_metadata_blocking(proj_lnk.clone(), resource_type)
                                        .await
                                    {
                                        named_results.push((proj_lnk, meta.name));
                                    } else {
                                        let name = proj_lnk
                                            .to_context_id()
                                            .unwrap_or("Unknown".to_string());
                                        named_results.push((proj_lnk, name));
                                    }
                                }
                                named_results
                            }
                            Ok(None) => Vec::new(),
                            Err(_) => Vec::new(),
                        };

                        results_map.insert(cleaned_name, matches_with_names);
                    }

                    let _ = tx
                        .send(InternalEvent::FolderImportCandidatesFound {
                            results: results_map,
                        })
                        .await;
                });
            }

            Effect::ImportFolderList {
                name,
                resource_type,
                version,
                loader,
                download_dir,
                projects,
            } => {
                let lm = self.list_manager.clone();
                let api_clone = api.clone();
                let tx = self.event_tx.clone();

                self.rt_handle.spawn(async move {
                    let project_strings: Vec<String> =
                        projects.iter().filter_map(|p| p.to_context_id()).collect();

                    let new_id = ProjectList::generate_id();
                    let mut new_list =
                        ProjectList::new(new_id.clone(), name.clone(), version.clone());
                    new_list.set_resource_type(
                        resource_type,
                        ProjectTypeConfig::new(loader.clone(), download_dir.clone()),
                    );

                    for proj_lnk in &projects {
                        if let Some(rtpm) = api_clone
                            .rt_project_pool
                            .get_metadata_blocking(proj_lnk.clone(), resource_type)
                            .await
                            .unwrap_or(None)
                            && let Some(id) = proj_lnk.to_context_id()
                        {
                            new_list.add_project(Project::new(
                                id,
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
                                    projects: project_strings,
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
                                    projects: project_strings,
                                    error: e.to_string(),
                                }))
                                .await;
                        }
                    }
                });
            }
        }
    }
}
