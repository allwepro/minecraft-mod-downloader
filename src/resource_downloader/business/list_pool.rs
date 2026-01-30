use crate::resource_downloader::business::Effect;
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, ProjectList, ProjectLnk, ResourceType,
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
