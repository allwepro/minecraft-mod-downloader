use crate::resource_downloader::business::{Effect, SharedRDState};
use crate::resource_downloader::domain::{
    GameVersion, InstanceSettings, ListGroup, ListGroupLnk, ListLnk, SidebarItem,
};
use std::time::{SystemTime, UNIX_EPOCH};

use super::list_actions::ListActions;

pub struct ListGroupActions;

impl ListGroupActions {
    fn generate_list_group_id() -> ListGroupLnk {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        ListGroupLnk::new(format!("list_group_{}", timestamp))
    }

    pub fn create_list_group_with_parent(
        state: SharedRDState,
        name: String,
        parent_id: Option<ListGroupLnk>,
    ) -> ListGroupLnk {
        let lg_id = Self::generate_list_group_id();

        {
            let s = state.read();
            let mut config = s.config.write();
            config.list_groups.push(ListGroup {
                lnk: lg_id.clone(),
                name,
                collapsed: false,
                parent_id,
                is_instance: false,
                instance_settings: None,
            });
        }

        state
            .read()
            .insert_sidebar_item(SidebarItem::ListGroup(lg_id.clone()), false);

        {
            let mut state_guard = state.write();
            state_guard.pending_sidebar_scroll = Some(SidebarItem::from(&lg_id));
        }

        lg_id
    }

    pub fn rename_list_group(state: SharedRDState, lg_lnk: ListGroupLnk, new_name: String) {
        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();
            if let Some(list_group) = config.list_groups.iter_mut().find(|f| f.lnk == lg_lnk) {
                list_group.name = new_name;
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn delete_list_group(state: SharedRDState, lg_lnk: ListGroupLnk) {
        ListActions::delete_items(state, vec![SidebarItem::ListGroup(lg_lnk)]);
    }

    pub fn duplicate_list_group(state: SharedRDState, lg_lnk: ListGroupLnk) {
        let parent_id = {
            let s = state.read();
            let config = s.config.read();
            config
                .list_groups
                .iter()
                .find(|f| f.lnk == lg_lnk)
                .and_then(|f| f.parent_id.clone())
        };
        Self::duplicate_list_group_recursive(state, lg_lnk, parent_id);
    }

    fn duplicate_list_group_recursive(
        state: SharedRDState,
        lg_lnk: ListGroupLnk,
        new_parent_id: Option<ListGroupLnk>,
    ) -> ListGroupLnk {
        let new_lg_id = Self::generate_list_group_id();
        let lists_to_duplicate: Vec<ListLnk>;
        let sublist_groups_to_duplicate: Vec<ListGroupLnk>;

        {
            let mut state_guard = state.write();
            let mut config = state_guard.config.write();

            if let Some(original) = config.list_groups.iter().find(|f| f.lnk == lg_lnk) {
                let new_list_group = ListGroup {
                    lnk: new_lg_id.clone(),
                    name: format!("{} (Copy)", original.name),
                    collapsed: original.collapsed,
                    is_instance: original.is_instance,
                    parent_id: new_parent_id.clone(),
                    instance_settings: original.instance_settings.clone(),
                };
                config.list_groups.push(new_list_group);

                let item_to_insert = SidebarItem::from(&new_lg_id);
                if let Some(pos) = config
                    .sidebar_ui_order
                    .iter()
                    .position(|id| id.match_list_group(&lg_lnk))
                {
                    config.sidebar_ui_order.insert(pos + 1, item_to_insert);
                } else {
                    config.sidebar_ui_order.insert(0, item_to_insert);
                }

                lists_to_duplicate = config
                    .list_group_assignments
                    .iter()
                    .filter(|(_, fid)| **fid == lg_lnk)
                    .map(|(list_id, _)| list_id.clone())
                    .collect();

                sublist_groups_to_duplicate = config
                    .list_groups
                    .iter()
                    .filter(|f| f.parent_id.as_ref() == Some(&lg_lnk))
                    .map(|f| f.lnk.clone())
                    .collect();

                drop(config);
                state_guard.pending_sidebar_scroll = Some(SidebarItem::from(&new_lg_id));
            } else {
                return lg_lnk;
            }
        }

        for list_lnk in lists_to_duplicate {
            state
                .read()
                .list_pool
                .duplicate(&list_lnk, Some(new_lg_id.clone()));
        }

        for sub_lg_lnk in sublist_groups_to_duplicate {
            Self::duplicate_list_group_recursive(
                state.clone(),
                sub_lg_lnk,
                Some(new_lg_id.clone()),
            );
        }

        {
            state.read().save_config();
        }

        new_lg_id
    }

    pub fn toggle_list_group_collapsed(state: SharedRDState, lg_lnk: ListGroupLnk) {
        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();
            if let Some(list_group) = config.list_groups.iter_mut().find(|f| f.lnk == lg_lnk) {
                list_group.collapsed = !list_group.collapsed;
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn move_list_to_list_group(
        state: SharedRDState,
        list_id: ListLnk,
        lg_lnk: Option<ListGroupLnk>,
    ) {
        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();

            if let Some(list_group) = lg_lnk {
                config.list_group_assignments.insert(list_id, list_group);
            } else {
                config.list_group_assignments.remove(&list_id);
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn is_instance_mode(state: SharedRDState, lg_lnk: ListGroupLnk) -> bool {
        let state_guard = state.read();
        let config = state_guard.config.read();
        config
            .list_groups
            .iter()
            .find(|f| f.lnk == lg_lnk)
            .map(|lg| lg.is_instance)
            .unwrap_or(false)
    }

    pub fn toggle_instance_mode(state: SharedRDState, lg_lnk: ListGroupLnk) {
        {
            let s = state.write();
            let mut config = s.config.write();
            if let Some(list_group) = config.list_groups.iter_mut().find(|f| f.lnk == lg_lnk) {
                list_group.is_instance = !list_group.is_instance;
                if list_group.is_instance && list_group.instance_settings.is_none() {
                    // Initialize with defaults if turning on
                    if let Some((_, default_dir)) = s.default_dirs.iter().next() {
                        let parent = std::path::PathBuf::from(default_dir)
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default();
                        list_group.instance_settings = Some(InstanceSettings {
                            download_directory: parent,
                            game_version: GameVersion::release("1.20.1".to_string()), // Default fallback
                        });
                    }
                }
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn update_instance_settings(
        state: SharedRDState,
        lg_lnk: ListGroupLnk,
        directory: String,
        version: GameVersion,
    ) {
        {
            let s = state.write();
            let mut config = s.config.write();
            if let Some(list_group) = config.list_groups.iter_mut().find(|f| f.lnk == lg_lnk) {
                list_group.instance_settings = Some(InstanceSettings {
                    download_directory: directory,
                    game_version: version,
                });
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }
}
