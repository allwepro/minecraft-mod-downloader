use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::business::{Effect, SharedRDState};
use crate::resource_downloader::domain::{ListGroup, ListGroupLnk, ListLnk, SidebarItem};
use std::time::{SystemTime, UNIX_EPOCH};

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
        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();

            let parent_id = config
                .list_groups
                .iter()
                .find(|f| f.lnk == lg_lnk)
                .and_then(|f| f.parent_id.clone());

            for group in config.list_groups.iter_mut() {
                if group.parent_id.as_ref() == Some(&lg_lnk) {
                    group.parent_id = parent_id.clone();
                }
            }

            if let Some(target_parent) = &parent_id {
                for fid in config.list_group_assignments.values_mut() {
                    if *fid == lg_lnk {
                        *fid = target_parent.clone();
                    }
                }
            } else {
                config
                    .list_group_assignments
                    .retain(|_, fid| *fid != lg_lnk);
            }

            config.list_groups.retain(|f| f.lnk != lg_lnk);
            config
                .sidebar_ui_order
                .retain(|i| !i.match_list_group(&lg_lnk));
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn duplicate_list_group(state: SharedRDState, lg_lnk: ListGroupLnk) {
        Self::duplicate_list_group_recursive(state, lg_lnk, None);
    }

    fn duplicate_list_group_recursive(
        state: SharedRDState,
        lg_lnk: ListGroupLnk,
        new_parent_id: Option<ListGroupLnk>,
    ) -> ListGroupLnk {
        let new_lg_id = Self::generate_list_group_id();
        let lists_to_duplicate: Vec<ListLnk>;
        let existing_list_ids: Vec<String>;
        let sublist_groups_to_duplicate: Vec<(ListGroupLnk, String)>;

        {
            let mut state_guard = state.write();
            let mut config = state_guard.config.write();

            if let Some(original) = config.list_groups.iter().find(|f| f.lnk == lg_lnk) {
                let new_list_group = ListGroup {
                    lnk: new_lg_id.clone(),
                    name: format!("{} (Copy)", original.name),
                    collapsed: original.collapsed,
                    parent_id: new_parent_id.clone(),
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

                let list_ids: Vec<ListLnk> = config
                    .list_group_assignments
                    .iter()
                    .filter(|(_, fid)| **fid == lg_lnk)
                    .map(|(list_id, _)| list_id.clone())
                    .collect();

                lists_to_duplicate = state_guard.list_pool.map_filter(|list| {
                    if list_ids.contains(&list.get_lnk()) {
                        Some(list.get_lnk())
                    } else {
                        None
                    }
                });

                existing_list_ids = state_guard
                    .list_pool
                    .map_filter(|list| Some(list.get_lnk().to_string()));

                sublist_groups_to_duplicate = config
                    .list_groups
                    .iter()
                    .filter(|f| f.parent_id.as_ref() == Some(&lg_lnk))
                    .map(|f| (f.lnk.clone(), f.name.clone()))
                    .collect();

                drop(config);

                state_guard.pending_sidebar_scroll = Some(SidebarItem::from(&new_lg_id));
            } else {
                lists_to_duplicate = Vec::new();
                existing_list_ids = Vec::new();
                sublist_groups_to_duplicate = Vec::new();
            }
        }

        {
            let state_guard = state.read();
            state_guard.dispatch(Effect::SaveConfig {
                config: state_guard.config.read().clone(),
            });
        }

        for (sublist_group_id, _sublist_group_name) in sublist_groups_to_duplicate {
            let sublist_group_lnk = sublist_group_id;
            Self::duplicate_list_group_recursive(
                state.clone(),
                sublist_group_lnk,
                Some(new_lg_id.clone()),
            );
        }

        if lists_to_duplicate.is_empty() {
            return new_lg_id;
        }

        let state_clone = state.clone();
        let new_list_group_id_clone = new_lg_id.clone();
        let num_lists_to_duplicate = lists_to_duplicate.len();

        std::thread::spawn(move || {
            for list_lnk in &lists_to_duplicate {
                ListActions::duplicate_list(state_clone.clone(), list_lnk.clone());
            }

            let mut newly_created: Vec<ListLnk> = Vec::new();
            let max_attempts = 20;

            for _attempt in 0..max_attempts {
                std::thread::sleep(std::time::Duration::from_millis(250));

                let state_guard = state_clone.read();
                let all_current_lists: Vec<ListLnk> = state_guard
                    .list_pool
                    .map_filter(|list| Some(list.get_lnk()));

                newly_created = all_current_lists
                    .into_iter()
                    .filter(|lnk| !existing_list_ids.contains(&lnk.to_string()))
                    .collect();

                if newly_created.len() >= num_lists_to_duplicate {
                    break;
                }
            }

            {
                let state_guard = state_clone.write();
                let mut config = state_guard.config.write();

                newly_created.truncate(num_lists_to_duplicate);

                for new_list_lnk in newly_created {
                    config
                        .list_group_assignments
                        .insert(new_list_lnk, new_list_group_id_clone.clone());
                }
            }

            let state_guard = state_clone.read();
            state_guard.dispatch(Effect::SaveConfig {
                config: state_guard.config.read().clone(),
            });
        });

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
}
