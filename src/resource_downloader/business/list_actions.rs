use crate::resource_downloader::business::{Effect, SharedRDState};
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, ProjectList, ResourceType, SidebarItem,
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
                let list = l.read();
                list.get_resource_types()
                    .first()
                    .cloned()
                    .unwrap_or(ResourceType::Mod)
            })
            .unwrap_or(ResourceType::Mod)
    }
    pub fn get_list_target_resource_type(list: &ProjectList) -> ResourceType {
        list.get_resource_types()
            .first()
            .cloned()
            .unwrap_or(ResourceType::Mod)
    }

    pub fn get_effective_game_version(
        state: &SharedRDState,
        list_lnk: &ListLnk,
        original_version: &GameVersion,
    ) -> GameVersion {
        let s = state.read();
        let config = s.config.read();

        if let Some(lg_lnk) = config.list_group_assignments.get(list_lnk)
            && let Some(list_group) = config.list_groups.iter().find(|lg| &lg.lnk == lg_lnk)
            && list_group.is_instance
            && let Some(instance_settings) = &list_group.instance_settings
        {
            return instance_settings.game_version.clone();
        }
        original_version.clone()
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
        Self::delete_items(state, vec![SidebarItem::List(list_lnk)]);
    }

    pub fn delete_items(state: SharedRDState, items: Vec<SidebarItem>) {
        let mut config_changed = false;
        {
            let mut s = state.write();
            let config_arc = s.config.clone();
            let mut config = config_arc.write();

            for item in &items {
                match item {
                    SidebarItem::List(list_lnk) => {
                        if s.open_list.as_ref() == Some(list_lnk) {
                            s.open_list = None;
                            config.last_open_list_id = None;
                        }
                        config.sidebar_ui_order.retain(|i| !i.match_list(list_lnk));
                        config.list_group_assignments.remove(list_lnk);
                        config_changed = true;
                        s.list_pool.delete(list_lnk);
                    }
                    SidebarItem::ListGroup(lg_lnk) => {
                        if s.open_list_group.as_ref() == Some(lg_lnk) {
                            s.open_list_group = None;
                        }

                        let parent_id = config
                            .list_groups
                            .iter()
                            .find(|f| &f.lnk == lg_lnk)
                            .and_then(|f| f.parent_id.clone());

                        for group in config.list_groups.iter_mut() {
                            if group.parent_id.as_ref() == Some(lg_lnk) {
                                group.parent_id = parent_id.clone();
                            }
                        }

                        if let Some(target_parent) = &parent_id {
                            for fid in config.list_group_assignments.values_mut() {
                                if fid == lg_lnk {
                                    *fid = target_parent.clone();
                                }
                            }
                        } else {
                            config.list_group_assignments.retain(|_, fid| fid != lg_lnk);
                        }

                        config.list_groups.retain(|f| &f.lnk != lg_lnk);
                        config
                            .sidebar_ui_order
                            .retain(|i| !i.match_list_group(lg_lnk));
                        config_changed = true;
                    }
                }
            }
        }

        if config_changed {
            let state_guard = state.read();
            state_guard.dispatch(Effect::SaveConfig {
                config: state_guard.config.read().clone(),
            });
            drop(state_guard);
            state.write().request_full_refresh();
        }
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
                let original_dir = config.download_dir.clone();
                let effective_dir =
                    Self::get_effective_download_dir(&state, &list_lnk, *rt, &original_dir);
                println!("Opening folder: {}", effective_dir);
                state.read().open_explorer(effective_dir.into());
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

    pub fn get_effective_download_dir(
        state: &SharedRDState,
        list_lnk: &ListLnk,
        resource_type: ResourceType,
        original_download_dir: &str,
    ) -> String {
        let s = state.read();
        let config = s.config.read();

        if let Some(lg_lnk) = config.list_group_assignments.get(list_lnk)
            && let Some(list_group) = config.list_groups.iter().find(|lg| &lg.lnk == lg_lnk)
            && list_group.is_instance
            && matches!(
                resource_type,
                ResourceType::Mod | ResourceType::ResourcePack | ResourceType::Shader
            )
            && let Some(instance_settings) = &list_group.instance_settings
        {
            let subfolder = resource_type.game_folder();
            let path = PathBuf::from(instance_settings.download_directory.clone()).join(subfolder);
            return path.to_str().unwrap().to_string();
        }

        original_download_dir.to_string()
    }
}
