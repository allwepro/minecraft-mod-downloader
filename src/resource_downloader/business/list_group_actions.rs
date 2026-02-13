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
}
