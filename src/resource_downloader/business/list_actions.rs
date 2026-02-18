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
        let current = { state.read().open_list.clone() };
        if current.as_ref() == Some(list) {
            state.write().set_open_list(None);
        } else {
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
                if config.last_open_item.as_ref() == Some(item) {
                    config.last_open_item = None;
                    config_changed = true;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ui::helper::modal_manager::SharedModalManager;
    use crate::common::ui::helper::notification_manager::SharedNotificationManager;
    use crate::common::ui::helper::pop_up_manager::SharedPopupManager;
    use crate::resource_downloader::business::services::ApiService;
    use crate::resource_downloader::business::{InternalEvent, RMState};
    use crate::resource_downloader::domain::{
        AppConfig, GameLoader, GameVersion, InstanceSettings, ListGroup, ListGroupLnk, ListLnk,
        ProjectList, ProjectTypeConfig, ResourceType, SidebarItem,
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

        let cache_dir = std::env::temp_dir().join("flux_test_cache_lists");
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

        let list_group_lnk = ListGroupLnk::new("test_group".to_string());

        let config = AppConfig {
            list_groups: vec![ListGroup {
                lnk: list_group_lnk.clone(),
                name: "Test Group".to_string(),
                collapsed: false,
                parent_id: None,
                is_instance: true,
                instance_settings: Some(InstanceSettings {
                    download_directory: "/test/instance".to_string(),
                    game_version: GameVersion::release("1.21".to_string()),
                }),
            }],
            ..Default::default()
        };

        state.config = Arc::new(RwLock::new(config));
        state
            .default_dirs
            .insert(ResourceType::Mod, "/default/mods".to_string());

        Arc::new(RwLock::new(state))
    }

    fn create_test_list(name: &str, resource_type: ResourceType) -> ProjectList {
        let mut list = ProjectList::new(
            format!("list_{}", name),
            name.to_string(),
            GameVersion::release("1.20.1".to_string()),
        );
        list.set_resource_type(
            resource_type,
            ProjectTypeConfig::new(
                GameLoader {
                    id: "fabric".to_string(),
                    name: "Fabric".to_string(),
                },
                "/test/mods".to_string(),
            ),
        );
        list
    }

    #[test]
    fn test_get_list_resource_type_default() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("nonexistent_list".to_string());

        let rt = ListActions::get_list_resource_type(&state, &list_lnk);
        assert_eq!(rt, ResourceType::Mod);
    }

    #[test]
    fn test_get_list_target_resource_type() {
        let list = create_test_list("test", ResourceType::Shader);
        let rt = ListActions::get_list_target_resource_type(&list);
        assert_eq!(rt, ResourceType::Shader);
    }

    #[test]
    fn test_get_list_target_resource_type_default() {
        let list = ProjectList::new(
            "empty_list".to_string(),
            "Empty".to_string(),
            GameVersion::release("1.20.1".to_string()),
        );
        let rt = ListActions::get_list_target_resource_type(&list);
        assert_eq!(rt, ResourceType::Mod); // Default
    }

    #[test]
    fn test_get_effective_game_version_without_instance() {
        let state = setup_test_state();
        let original_version = GameVersion::release("1.20.1".to_string());
        let list_lnk = ListLnk::new("test_list".to_string());

        let effective_version =
            ListActions::get_effective_game_version(&state, &list_lnk, &original_version);
        assert_eq!(effective_version.name, "1.20.1");
    }

    #[test]
    fn test_get_effective_game_version_with_instance() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("test_list".to_string());
        let list_group_lnk = ListGroupLnk::new("test_group".to_string());

        {
            let s = state.read();
            let mut config = s.config.write();
            config
                .list_group_assignments
                .insert(list_lnk.clone(), list_group_lnk);
        }

        let original_version = GameVersion::release("1.20.1".to_string());
        let effective_version =
            ListActions::get_effective_game_version(&state, &list_lnk, &original_version);

        assert_eq!(effective_version.name, "1.21");
    }

    #[test]
    fn test_toggle_open_list() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("test_list".to_string());

        assert!(state.read().open_list.is_none());

        ListActions::toggle_open_list(state.clone(), &list_lnk);
        assert_eq!(state.read().open_list, Some(list_lnk.clone()));

        ListActions::toggle_open_list(state.clone(), &list_lnk);
        assert!(state.read().open_list.is_none());
    }

    #[test]
    fn test_rename_list_nonexistent() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("nonexistent".to_string());

        ListActions::rename_list(state.clone(), list_lnk.clone(), "New Name".to_string());
    }

    #[test]
    fn test_set_sidebar_ui_order() {
        let state = setup_test_state();
        let item1 = SidebarItem::List(ListLnk::new("list1".to_string()));
        let item2 = SidebarItem::ListGroup(ListGroupLnk::new("group1".to_string()));
        let new_order = vec![item1.clone(), item2.clone()];

        ListActions::set_sidebar_ui_order(state.clone(), new_order.clone());

        let s = state.read();
        let config = s.config.read();
        assert_eq!(config.sidebar_ui_order.len(), 2);
        assert!(config.sidebar_ui_order.contains(&item1));
        assert!(config.sidebar_ui_order.contains(&item2));
    }

    #[test]
    fn test_delete_list_removes_from_config() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("test_list".to_string());

        {
            let s = state.read();
            let mut config = s.config.write();
            config
                .sidebar_ui_order
                .push(SidebarItem::List(list_lnk.clone()));
        }

        ListActions::delete_list(state.clone(), list_lnk.clone());

        let s = state.read();
        let config = s.config.read();
        assert!(
            !config
                .sidebar_ui_order
                .iter()
                .any(|item| item.match_list(&list_lnk))
        );
    }

    #[test]
    fn test_delete_list_closes_if_open() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("test_list".to_string());

        state.write().open_list = Some(list_lnk.clone());
        assert!(state.read().open_list.is_some());

        ListActions::delete_list(state.clone(), list_lnk.clone());

        assert!(state.read().open_list.is_none());
    }

    #[test]
    fn test_delete_list_group_reassigns_children() {
        let state = setup_test_state();
        let parent_group = ListGroupLnk::new("parent".to_string());
        let child_group = ListGroupLnk::new("child".to_string());
        let grandchild_group = ListGroupLnk::new("grandchild".to_string());

        {
            let s = state.read();
            let mut config = s.config.write();

            config.list_groups.push(ListGroup {
                lnk: parent_group.clone(),
                name: "Parent".to_string(),
                collapsed: false,
                parent_id: None,
                is_instance: false,
                instance_settings: None,
            });

            config.list_groups.push(ListGroup {
                lnk: child_group.clone(),
                name: "Child".to_string(),
                collapsed: false,
                parent_id: Some(parent_group.clone()),
                is_instance: false,
                instance_settings: None,
            });

            config.list_groups.push(ListGroup {
                lnk: grandchild_group.clone(),
                name: "Grandchild".to_string(),
                collapsed: false,
                parent_id: Some(child_group.clone()),
                is_instance: false,
                instance_settings: None,
            });
        }

        ListActions::delete_items(
            state.clone(),
            vec![SidebarItem::ListGroup(child_group.clone())],
        );

        let s = state.read();
        let config = s.config.read();
        let grandchild = config
            .list_groups
            .iter()
            .find(|g| g.lnk == grandchild_group)
            .unwrap();
        assert_eq!(grandchild.parent_id, Some(parent_group));
    }

    #[test]
    fn test_get_effective_download_dir_without_instance() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("test_list".to_string());
        let original_dir = "/original/mods";

        let effective_dir = ListActions::get_effective_download_dir(
            &state,
            &list_lnk,
            ResourceType::Mod,
            original_dir,
        );

        assert_eq!(effective_dir, original_dir);
    }

    #[test]
    fn test_get_effective_download_dir_with_instance() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("test_list".to_string());
        let list_group_lnk = ListGroupLnk::new("test_group".to_string());

        {
            let s = state.read();
            let mut config = s.config.write();
            config
                .list_group_assignments
                .insert(list_lnk.clone(), list_group_lnk);
        }

        let original_dir = "/original/mods";
        let effective_dir = ListActions::get_effective_download_dir(
            &state,
            &list_lnk,
            ResourceType::Mod,
            original_dir,
        );

        assert!(effective_dir.contains("/test/instance"));
        assert!(effective_dir.contains("mods"));
    }

    #[test]
    fn test_get_effective_download_dir_instance_only_for_supported_types() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("test_list".to_string());
        let list_group_lnk = ListGroupLnk::new("test_group".to_string());

        {
            let s = state.read();
            let mut config = s.config.write();
            config
                .list_group_assignments
                .insert(list_lnk.clone(), list_group_lnk);
        }

        let original_dir = "/original/datapacks";

        let effective_dir = ListActions::get_effective_download_dir(
            &state,
            &list_lnk,
            ResourceType::Datapack,
            original_dir,
        );

        assert_eq!(effective_dir, original_dir);
    }

    #[test]
    fn test_get_effective_download_dir_shader_uses_instance() {
        let state = setup_test_state();
        let list_lnk = ListLnk::new("test_list".to_string());
        let list_group_lnk = ListGroupLnk::new("test_group".to_string());

        {
            let s = state.read();
            let mut config = s.config.write();
            config
                .list_group_assignments
                .insert(list_lnk.clone(), list_group_lnk);
        }

        let original_dir = "/original/shaderpacks";
        let effective_dir = ListActions::get_effective_download_dir(
            &state,
            &list_lnk,
            ResourceType::Shader,
            original_dir,
        );

        assert!(effective_dir.contains("/test/instance"));
        assert!(effective_dir.contains("shaderpacks"));
    }

    #[test]
    fn test_delete_multiple_items() {
        let state = setup_test_state();
        let list1 = ListLnk::new("list1".to_string());
        let list2 = ListLnk::new("list2".to_string());
        let group1 = ListGroupLnk::new("group1".to_string());

        {
            let s = state.read();
            let mut config = s.config.write();
            config
                .sidebar_ui_order
                .push(SidebarItem::List(list1.clone()));
            config
                .sidebar_ui_order
                .push(SidebarItem::List(list2.clone()));
            config
                .sidebar_ui_order
                .push(SidebarItem::ListGroup(group1.clone()));

            config.list_groups.push(ListGroup {
                lnk: group1.clone(),
                name: "Group 1".to_string(),
                collapsed: false,
                parent_id: None,
                is_instance: false,
                instance_settings: None,
            });
        }

        let items = vec![
            SidebarItem::List(list1.clone()),
            SidebarItem::ListGroup(group1.clone()),
        ];

        ListActions::delete_items(state.clone(), items);

        let s = state.read();
        let config = s.config.read();

        assert!(
            !config
                .sidebar_ui_order
                .iter()
                .any(|item| item.match_list(&list1))
        );
        assert!(
            !config
                .sidebar_ui_order
                .iter()
                .any(|item| item.match_list_group(&group1))
        );

        assert!(
            config
                .sidebar_ui_order
                .iter()
                .any(|item| item.match_list(&list2))
        );
    }
}
