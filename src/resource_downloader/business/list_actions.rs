use crate::resource_downloader::business::{Effect, SharedRDState};
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, ResourceType, SidebarItem,
};
use std::path::PathBuf;

pub struct ListActions;

impl ListActions {
    pub fn get_list_resource_type(state: &SharedRDState, list_lnk: &ListLnk) -> ResourceType {
        state
            .read()
            .list_pool
            .get(list_lnk)
            .map(|l| {
                l.read()
                    .get_resource_types()
                    .first()
                    .cloned()
                    .unwrap_or(ResourceType::Mod)
            })
            .unwrap_or(ResourceType::Mod)
    }

    pub fn create_list(
        state: SharedRDState,
        name: String,
        resource_type: ResourceType,
        version: GameVersion,
        loader: GameLoader,
        download_dir: String,
    ) {
        state.read().list_pool.create_list(
            name,
            resource_type,
            version,
            loader,
            download_dir,
            vec![],
        );
    }

    pub fn set_sidebar_ui_order(state: SharedRDState, new_order: Vec<SidebarItem>) {
        let state_guard = state.write();
        state_guard.config.write().sidebar_ui_order = new_order;
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn toggle_open_list(state: SharedRDState, list: &ListLnk) {
        let current = state.read().open_list.clone();
        if current.as_ref() == Some(list) {
            state.write().set_open_list(None);
        } else {
            {
                let mut s = state.write();
                if s.open_list_group.is_some() {
                    s.open_list_group = None;
                }
            }
            state.write().set_open_list(Some(list.clone()));
        }
    }

    pub fn rename_list(state: SharedRDState, list_lnk: ListLnk, new_name: String) {
        if let Some(list_arc) = state.read().list_pool.get(&list_lnk) {
            let mut list = list_arc.write();
            list.set_list_name(new_name);
            drop(list);
            state.read().list_pool.save(&list_lnk);
        }
    }

    pub fn delete_list(state: SharedRDState, list_lnk: ListLnk) {
        {
            let mut s = state.write();
            if s.open_list.as_ref() == Some(&list_lnk) {
                s.set_open_list_no_save(None);
            }
        }
        state.read().list_pool.delete(&list_lnk);
    }

    pub fn duplicate_list(state: SharedRDState, list_lnk: ListLnk) {
        state.read().list_pool.duplicate(&list_lnk, None);
    }

    pub fn open_folder(state: SharedRDState, list_lnk: ListLnk) {
        if let Some(list_arc) = state.read().list_pool.get(&list_lnk) {
            let list = list_arc.read();
            if let Some(rt) = list.get_resource_types().first()
                && let Some(config) = list.get_resource_type_config(rt)
            {
                let dir = config.download_dir.clone();
                state.read().open_explorer(dir.into());
            }
        }
    }

    pub fn import_list(state: SharedRDState, path: PathBuf) {
        state.read().list_pool.import(path);
    }

    pub fn export_list(state: SharedRDState, list_lnk: ListLnk, path: PathBuf) {
        state.read().list_pool.export(&list_lnk, path);
    }

    pub fn export_legacy_list(
        state: SharedRDState,
        list_lnk: ListLnk,
        path: PathBuf,
        version: GameVersion,
        loader: GameLoader,
    ) {
        state
            .read()
            .list_pool
            .export_legacy(&list_lnk, path, version, loader);
    }

    pub fn refresh_dependencies(state: SharedRDState, list_lnk: ListLnk) {
        state
            .read()
            .list_pool
            .mutate(&list_lnk, |list| list.recalculate_dependents());
    }
}
