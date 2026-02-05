use crate::resource_downloader::business::Effect;
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, MutationResult, ProjectList, ProjectLnk, ResourceType,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct ListPool {
    rt_handle: tokio::runtime::Handle,
    lists: Arc<RwLock<HashMap<ListLnk, Arc<RwLock<ProjectList>>>>>,
    effect_sx: mpsc::Sender<Effect>,
}

impl ListPool {
    pub fn new(rt_handle: tokio::runtime::Handle, effect_sx: mpsc::Sender<Effect>) -> Self {
        Self {
            rt_handle,
            lists: Arc::new(RwLock::new(HashMap::new())),
            effect_sx,
        }
    }

    pub fn map_filter<F, R>(&self, f: F) -> Vec<R>
    where
        F: Fn(&ProjectList) -> Option<R>,
    {
        let lists_guard = self.lists.read();
        lists_guard
            .values()
            .filter_map(|arc| {
                let list = arc.read();
                f(&list)
            })
            .collect()
    }

    pub fn get(&self, lnk: &ListLnk) -> Option<Arc<RwLock<ProjectList>>> {
        self.lists.read().get(lnk).cloned()
    }

    pub fn create_list(
        &self,
        name: String,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
        projects: Vec<String>,
    ) {
        let _ = self.effect_sx.try_send(Effect::CreateList {
            name,
            resource_type,
            version,
            loader,
            download_dir,
            projects,
        });
    }

    pub fn save(&self, lnk: &ListLnk) {
        self.send_list_effect(lnk, |arc| Effect::SaveList { list: arc });
    }
    pub fn duplicate(&self, lnk: &ListLnk) {
        self.send_list_effect(lnk, |arc| Effect::DuplicateList { list: arc });
    }
    pub fn delete(&self, lnk: &ListLnk) {
        let _ = self
            .effect_sx
            .try_send(Effect::DeleteList { list: lnk.clone() });
    }
    pub fn import(&self, path: PathBuf) {
        let _ = self.effect_sx.try_send(Effect::ImportList { path });
    }
    pub fn export(&self, lnk: &ListLnk, path: PathBuf) {
        self.send_list_effect(lnk, |arc| Effect::ExportList { list: arc, path });
    }

    #[allow(dead_code)]
    pub fn select_version(&self, lnk: &ListLnk, project: ProjectLnk, version_id: String) {
        self.send_list_effect(lnk, |arc| Effect::SelectProjectVersion {
            list: arc,
            project,
            version_id,
        });
    }

    pub fn import_legacy(
        &self,
        path: PathBuf,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
    ) {
        let _ = self.effect_sx.try_send(Effect::ImportLegacyList {
            path,
            version,
            loader,
            download_dir,
        });
    }
    pub fn export_legacy(
        &self,
        lnk: &ListLnk,
        path: PathBuf,
        version: GameVersion,
        loader: GameLoader,
    ) {
        self.send_list_effect(lnk, |arc| Effect::ExportLegacyList {
            list: arc,
            path,
            version,
            loader,
        });
    }

    pub fn mutate<F>(&self, lnk: &ListLnk, mutator: F)
    where
        F: FnOnce(&mut ProjectList) -> MutationResult + Send + 'static,
    {
        let pool = Arc::clone(&self.lists);
        let sx = self.effect_sx.clone();
        let l = lnk.clone();
        self.rt_handle.spawn(async move {
            let list_arc = { pool.read().get(&l).cloned() };
            if let Some(arc) = list_arc {
                let (result, potential_deletions, potential_archival_changes) = {
                    let mut list = arc.write();

                    let mut pre_archived_status = HashMap::new();
                    for p in list.get_target_projects() {
                        let p_lnk = p.get_lnk();
                        pre_archived_status.insert(p_lnk.clone(), list.is_project_archived(&p_lnk));
                    }

                    let result = mutator(&mut list);

                    let mut potential_deletions = Vec::new();
                    let mut potential_archival_changes = Vec::new();

                    if result.is_success() {
                        for p in result.deleted_projects() {
                            if let Some(tc) = list.get_resource_type_config(&p.resource_type) {
                                potential_deletions.push((
                                    PathBuf::from(&tc.download_dir),
                                    p.get_safe_filename(),
                                ));
                            }
                        }

                        for p in list.get_target_projects() {
                            let p_lnk = p.get_lnk();
                            let is_archived_now = list.is_project_archived(&p_lnk);
                            let was_archived_before =
                                pre_archived_status.get(&p_lnk).copied().unwrap_or(false);

                            if is_archived_now != was_archived_before {
                                if let Some(tc) = list.get_resource_type_config(&p.resource_type) {
                                    potential_archival_changes.push((
                                        PathBuf::from(&tc.download_dir),
                                        p.get_safe_filename(),
                                        is_archived_now,
                                    ));
                                }
                            }
                        }
                    }
                    (
                        result,
                        potential_deletions,
                        potential_archival_changes,
                    )
                };

                let mut deleted_safe = Vec::new();
                if result.is_success() {
                    for (dir, filename) in potential_deletions {
                        let path = dir.join(&filename);
                        if tokio::fs::metadata(&path).await.is_ok() {
                            deleted_safe.push((dir.clone(), filename.clone()));
                        }
                        let archive_filename = format!("{}.archive", filename);
                        let archive_path = dir.join(&archive_filename);
                        if tokio::fs::metadata(&archive_path).await.is_ok() {
                            deleted_safe.push((dir.clone(), archive_filename));
                        }
                    }

                    let mut effective_archival_changes = Vec::new();
                    for (dir, filename, is_archived_now) in potential_archival_changes {
                        let file_to_move_from_name = if is_archived_now {
                            filename.clone()
                        } else {
                            format!("{}.archive", filename)
                        };

                        let path_to_check = dir.join(&file_to_move_from_name);

                        if tokio::fs::metadata(&path_to_check).await.is_ok() {
                            effective_archival_changes.push((dir, filename, is_archived_now));
                        }
                    }

                    for (path, filename) in deleted_safe {
                        let _ = sx.send(Effect::DeleteArtifact { path, filename }).await;
                    }

                    for (path, filename, is_archived) in effective_archival_changes {
                        if is_archived {
                            let _ = sx
                                .send(Effect::ArchiveProjectFile { path, filename })
                                .await;
                        } else {
                            let _ = sx
                                .send(Effect::UnarchiveProjectFile { path, filename })
                                .await;
                        }
                    }

                    let _ = sx.send(Effect::SaveList { list: arc.clone() }).await;

                    let refresh_requests = {
                        let list = arc.read();
                        let mut reqs = Vec::new();
                        for rt in list.get_resource_types() {
                            if let Some(tc) = list.get_resource_type_config(&rt) {
                                reqs.push((
                                    tc.download_dir.clone().into(),
                                    vec![
                                        rt.file_extension(),
                                        format!("{}.archive", rt.file_extension()),
                                    ],
                                ));
                            }
                        }
                        reqs
                    };

                    for (directory, file_extension) in refresh_requests {
                        let _ = sx
                            .send(Effect::FindFiles {
                                directory,
                                file_extension,
                            })
                            .await;
                    }
                }
            }
        });
    }

    // --- INTERNAL HELPERS ---

    fn send_list_effect(
        &self,
        lnk: &ListLnk,
        f: impl FnOnce(Arc<RwLock<ProjectList>>) -> Effect + Send + 'static,
    ) {
        let pool = Arc::clone(&self.lists);
        let sx = self.effect_sx.clone();
        let l = lnk.clone();
        self.rt_handle.spawn(async move {
            let list_to_send = { pool.read().get(&l).cloned() };
            if let Some(arc) = list_to_send {
                let _ = sx.send(f(arc)).await;
            }
        });
    }

    pub(crate) fn insert_arc(&self, list_arc: Arc<RwLock<ProjectList>>) {
        let pool = Arc::clone(&self.lists);
        self.rt_handle.spawn(async move {
            let lnk = list_arc.read().get_lnk();
            pool.write().insert(lnk, list_arc);
        });
    }

    pub(crate) fn remove_sync(&self, lnk: &ListLnk) {
        let pool = Arc::clone(&self.lists);
        let l = lnk.clone();
        self.rt_handle.spawn(async move {
            pool.write().remove(&l);
        });
    }
}
