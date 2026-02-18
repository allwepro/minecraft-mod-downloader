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

    pub fn toggle_open_group_list(state: SharedRDState, lg_lnk: &ListGroupLnk) {
        let current = { state.read().open_list_group.clone() };
        if current.as_ref() == Some(lg_lnk) {
            state.write().set_open_list_group(None);
        } else {
            state.write().set_open_list_group(Some(lg_lnk.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ui::helper::modal_manager::SharedModalManager;
    use crate::common::ui::helper::notification_manager::SharedNotificationManager;
    use crate::common::ui::helper::pop_up_manager::SharedPopupManager;
    use crate::resource_downloader::business::services::ApiService;
    use crate::resource_downloader::business::{InternalEvent, RMState};
    use crate::resource_downloader::domain::{
        AppConfig, GameVersion, InstanceSettings, ListGroup, ListGroupLnk, ListLnk, ResourceType,
        SidebarItem,
    };
    use parking_lot::RwLock;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    fn setup_test_state() -> SharedRDState {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.handle().clone();

        let modal_manager = SharedModalManager::default();
        let popup_manager = SharedPopupManager::default();
        let notification_manager = SharedNotificationManager::new();

        let (_effect_sx, _effect_rx) = mpsc::channel::<Effect>(1024);
        let (_event_sx, event_rx) = mpsc::channel::<InternalEvent>(1024);

        let cache_dir = std::env::temp_dir().join("flux_test_cache");
        let (api_service, _, _, _) = ApiService::new(&handle, cache_dir);
        let api_service = Arc::new(api_service);

        let (effect_sx, _) = mpsc::channel::<Effect>(1024);

        let mut state = RMState::new(
            handle.clone(),
            modal_manager,
            popup_manager,
            notification_manager,
            api_service,
            event_rx,
            effect_sx,
        );

        let config = AppConfig {
            list_groups: vec![
                ListGroup {
                    lnk: ListGroupLnk::new("group_1".to_string()),
                    name: "Group 1".to_string(),
                    collapsed: false,
                    parent_id: None,
                    is_instance: false,
                    instance_settings: None,
                },
                ListGroup {
                    lnk: ListGroupLnk::new("group_2".to_string()),
                    name: "Group 2".to_string(),
                    collapsed: true,
                    parent_id: None,
                    is_instance: true,
                    instance_settings: Some(InstanceSettings {
                        download_directory: "/test/dir".to_string(),
                        game_version: GameVersion::release("1.20.1".to_string()),
                    }),
                },
            ],
            sidebar_ui_order: vec![
                SidebarItem::ListGroup(ListGroupLnk::new("group_1".to_string())),
                SidebarItem::ListGroup(ListGroupLnk::new("group_2".to_string())),
            ],
            ..Default::default()
        };

        state.config = Arc::new(RwLock::new(config));

        Arc::new(RwLock::new(state))
    }

    #[test]
    fn test_create_list_group_with_parent() {
        let state = setup_test_state();
        let parent_id = Some(ListGroupLnk::new("group_1".to_string()));

        let new_lg_id = ListGroupActions::create_list_group_with_parent(
            state.clone(),
            "New Group".to_string(),
            parent_id.clone(),
        );

        let state_guard = state.read();
        let config = state_guard.config.read();
        let created_group = config.list_groups.iter().find(|g| g.lnk == new_lg_id);

        assert!(created_group.is_some());
        let group = created_group.unwrap();
        assert_eq!(group.name, "New Group");
        assert_eq!(group.parent_id, parent_id);
        assert!(!group.collapsed);
        assert!(!group.is_instance);
    }

    #[test]
    fn test_rename_list_group() {
        let state = setup_test_state();
        let lg_lnk = ListGroupLnk::new("group_1".to_string());

        ListGroupActions::rename_list_group(
            state.clone(),
            lg_lnk.clone(),
            "Renamed Group".to_string(),
        );

        let state_guard = state.read();
        let config = state_guard.config.read();
        let group = config.list_groups.iter().find(|g| g.lnk == lg_lnk).unwrap();

        assert_eq!(group.name, "Renamed Group");
    }

    #[test]
    fn test_toggle_list_group_collapsed() {
        let state = setup_test_state();
        let lg_lnk = ListGroupLnk::new("group_1".to_string());

        {
            let state_guard = state.read();
            let config = state_guard.config.read();
            let group = config.list_groups.iter().find(|g| g.lnk == lg_lnk).unwrap();
            assert!(!group.collapsed);
        }

        ListGroupActions::toggle_list_group_collapsed(state.clone(), lg_lnk.clone());

        {
            let state_guard = state.read();
            let config = state_guard.config.read();
            let group = config.list_groups.iter().find(|g| g.lnk == lg_lnk).unwrap();
            assert!(group.collapsed);
        }

        ListGroupActions::toggle_list_group_collapsed(state.clone(), lg_lnk.clone());

        {
            let state_guard = state.read();
            let config = state_guard.config.read();
            let group = config.list_groups.iter().find(|g| g.lnk == lg_lnk).unwrap();
            assert!(!group.collapsed);
        }
    }

    #[test]
    fn test_move_list_to_list_group() {
        let state = setup_test_state();
        let list_id = ListLnk::new("list_1".to_string());
        let lg_lnk = ListGroupLnk::new("group_1".to_string());

        ListGroupActions::move_list_to_list_group(
            state.clone(),
            list_id.clone(),
            Some(lg_lnk.clone()),
        );

        {
            let state_guard = state.read();
            let config = state_guard.config.read();
            assert_eq!(config.list_group_assignments.get(&list_id), Some(&lg_lnk));
        }

        ListGroupActions::move_list_to_list_group(state.clone(), list_id.clone(), None);

        {
            let state_guard = state.read();
            let config = state_guard.config.read();
            assert_eq!(config.list_group_assignments.get(&list_id), None);
        }
    }

    #[test]
    fn test_is_instance_mode() {
        let state = setup_test_state();
        let lg_lnk_1 = ListGroupLnk::new("group_1".to_string());
        let lg_lnk_2 = ListGroupLnk::new("group_2".to_string());

        assert!(!ListGroupActions::is_instance_mode(state.clone(), lg_lnk_1));
        assert!(ListGroupActions::is_instance_mode(state.clone(), lg_lnk_2));
    }

    #[test]
    fn test_toggle_instance_mode() {
        let state = setup_test_state();
        let lg_lnk = ListGroupLnk::new("group_1".to_string());

        {
            let mut s = state.write();
            s.default_dirs
                .insert(ResourceType::Mod, "/default/mods".to_string());
        }

        assert!(!ListGroupActions::is_instance_mode(
            state.clone(),
            lg_lnk.clone()
        ));

        ListGroupActions::toggle_instance_mode(state.clone(), lg_lnk.clone());

        {
            let state_guard = state.read();
            let config = state_guard.config.read();
            let group = config.list_groups.iter().find(|g| g.lnk == lg_lnk).unwrap();
            assert!(group.is_instance);
            assert!(group.instance_settings.is_some());
        }

        ListGroupActions::toggle_instance_mode(state.clone(), lg_lnk.clone());

        {
            let state_guard = state.read();
            let config = state_guard.config.read();
            let group = config.list_groups.iter().find(|g| g.lnk == lg_lnk).unwrap();
            assert!(!group.is_instance);
        }
    }

    #[test]
    fn test_update_instance_settings() {
        let state = setup_test_state();
        let lg_lnk = ListGroupLnk::new("group_2".to_string());
        let new_dir = "/new/test/dir".to_string();
        let new_version = GameVersion::release("1.21".to_string());

        ListGroupActions::update_instance_settings(
            state.clone(),
            lg_lnk.clone(),
            new_dir.clone(),
            new_version.clone(),
        );

        let state_guard = state.read();
        let config = state_guard.config.read();
        let group = config.list_groups.iter().find(|g| g.lnk == lg_lnk).unwrap();

        assert!(group.instance_settings.is_some());
        let settings = group.instance_settings.as_ref().unwrap();
        assert_eq!(settings.download_directory, new_dir);
        assert_eq!(settings.game_version.name, new_version.name);
    }

    #[test]
    fn test_toggle_open_group_list() {
        let state = setup_test_state();
        let lg_lnk = ListGroupLnk::new("group_1".to_string());

        assert!(state.read().open_list_group.is_none());

        ListGroupActions::toggle_open_group_list(state.clone(), &lg_lnk);
        assert_eq!(state.read().open_list_group, Some(lg_lnk.clone()));

        ListGroupActions::toggle_open_group_list(state.clone(), &lg_lnk);
        assert!(state.read().open_list_group.is_none());
    }

    #[test]
    fn test_duplicate_list_group_creates_copy() {
        let state = setup_test_state();
        let lg_lnk = ListGroupLnk::new("group_1".to_string());

        let original_count = {
            let state_guard = state.read();
            let config = state_guard.config.read();
            config.list_groups.len()
        };

        ListGroupActions::duplicate_list_group(state.clone(), lg_lnk.clone());

        let state_guard = state.read();
        let config = state_guard.config.read();
        assert_eq!(config.list_groups.len(), original_count + 1);

        let duplicated = config
            .list_groups
            .iter()
            .find(|g| g.name == "Group 1 (Copy)");
        assert!(duplicated.is_some());

        let dup = duplicated.unwrap();
        assert_ne!(dup.lnk, lg_lnk);
        assert_eq!(dup.name, "Group 1 (Copy)");
    }

    #[test]
    fn test_generate_list_group_id_is_unique() {
        let id1 = ListGroupActions::generate_list_group_id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = ListGroupActions::generate_list_group_id();

        assert_ne!(id1, id2);
    }
}
