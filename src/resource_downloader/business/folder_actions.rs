use crate::resource_downloader::business::{Effect, SharedRDState};
use crate::resource_downloader::domain::{Folder, FolderLnk, ListLnk};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct FolderActions;

impl FolderActions {
    fn generate_folder_id() -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("folder_{}", timestamp)
    }

    #[allow(dead_code)]
    pub fn create_folder(state: SharedRDState, name: String) -> FolderLnk {
        Self::create_folder_with_parent(state, name, None)
    }

    pub fn create_folder_with_parent(
        state: SharedRDState,
        name: String,
        parent_id: Option<String>,
    ) -> FolderLnk {
        let folder_id = Self::generate_folder_id();
        let folder = Folder {
            id: folder_id.clone(),
            name,
            collapsed: false,
            parent_id,
        };

        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();
            config.folders.push(folder);
            config.folder_order.push(folder_id.clone());
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });

        FolderLnk::new(folder_id)
    }

    pub fn rename_folder(state: SharedRDState, folder_lnk: FolderLnk, new_name: String) {
        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();
            if let Some(folder) = config.folders.iter_mut().find(|f| f.id == folder_lnk.id()) {
                folder.name = new_name;
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn delete_folder(state: SharedRDState, folder_lnk: FolderLnk) {
        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();

            // Remove folder
            config.folders.retain(|f| f.id != folder_lnk.id());
            config.folder_order.retain(|id| id != folder_lnk.id());

            // Remove all assignments to this folder
            config
                .folder_assignments
                .retain(|_, fid| fid != folder_lnk.id());
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn duplicate_folder(state: SharedRDState, folder_lnk: FolderLnk) {
        Self::duplicate_folder_recursive(state, folder_lnk, None);
    }

    fn duplicate_folder_recursive(
        state: SharedRDState,
        folder_lnk: FolderLnk,
        new_parent_id: Option<String>,
    ) -> String {
        use crate::resource_downloader::business::list_actions::ListActions;

        let new_folder_id = Self::generate_folder_id();
        let lists_to_duplicate: Vec<ListLnk>;
        let existing_list_ids: Vec<String>;
        let subfolders_to_duplicate: Vec<(String, String)>; // (subfolder_id, name)

        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();

            if let Some(original) = config.folders.iter().find(|f| f.id == folder_lnk.id()) {
                let new_folder = Folder {
                    id: new_folder_id.clone(),
                    name: format!("{} (Copy)", original.name),
                    collapsed: original.collapsed,
                    parent_id: new_parent_id.clone(),
                };
                config.folders.push(new_folder);

                // Find position of original folder in order
                if let Some(pos) = config
                    .folder_order
                    .iter()
                    .position(|id| id == folder_lnk.id())
                {
                    config.folder_order.insert(pos + 1, new_folder_id.clone());
                } else {
                    config.folder_order.push(new_folder_id.clone());
                }

                let list_ids: Vec<String> = config
                    .folder_assignments
                    .iter()
                    .filter(|(_, fid)| *fid == folder_lnk.id())
                    .map(|(list_id, _)| list_id.clone())
                    .collect();

                lists_to_duplicate = state_guard.list_pool.map_filter(|list| {
                    let list_id_str = list.get_lnk().to_string();
                    if list_ids.contains(&list_id_str) {
                        Some(list.get_lnk())
                    } else {
                        None
                    }
                });

                // Store existing list IDs before duplication
                existing_list_ids = state_guard
                    .list_pool
                    .map_filter(|list| Some(list.get_lnk().to_string()));

                subfolders_to_duplicate = config
                    .folders
                    .iter()
                    .filter(|f| f.parent_id.as_ref() == Some(&folder_lnk.id().to_string()))
                    .map(|f| (f.id.clone(), f.name.clone()))
                    .collect();
            } else {
                lists_to_duplicate = Vec::new();
                existing_list_ids = Vec::new();
                subfolders_to_duplicate = Vec::new();
            }
        }

        {
            let state_guard = state.read();
            state_guard.dispatch(Effect::SaveConfig {
                config: state_guard.config.read().clone(),
            });
        }

        for (subfolder_id, _subfolder_name) in subfolders_to_duplicate {
            let subfolder_lnk = FolderLnk::new(subfolder_id);
            Self::duplicate_folder_recursive(
                state.clone(),
                subfolder_lnk,
                Some(new_folder_id.clone()),
            );
        }

        if lists_to_duplicate.is_empty() {
            return new_folder_id;
        }

        let state_clone = state.clone();
        let new_folder_id_clone = new_folder_id.clone();
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
                        .folder_assignments
                        .insert(new_list_lnk.to_string(), new_folder_id_clone.clone());
                }
            }

            let state_guard = state_clone.read();
            state_guard.dispatch(Effect::SaveConfig {
                config: state_guard.config.read().clone(),
            });
        });

        new_folder_id
    }

    pub fn toggle_folder_collapsed(state: SharedRDState, folder_lnk: FolderLnk) {
        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();
            if let Some(folder) = config.folders.iter_mut().find(|f| f.id == folder_lnk.id()) {
                folder.collapsed = !folder.collapsed;
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn move_list_to_folder(
        state: SharedRDState,
        list_id: String,
        folder_lnk: Option<FolderLnk>,
    ) {
        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();

            if let Some(folder) = folder_lnk {
                config
                    .folder_assignments
                    .insert(list_id, folder.id().to_string());
            } else {
                config.folder_assignments.remove(&list_id);
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn set_folder_order(state: SharedRDState, new_order: Vec<String>) {
        {
            let state_guard = state.write();
            state_guard.config.write().folder_order = new_order;
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    pub fn move_folder_to_parent(
        state: SharedRDState,
        folder_id: String,
        parent_folder_id: Option<String>,
    ) {
        if let Some(parent_id) = &parent_folder_id {
            if parent_id == &folder_id {
                return;
            }

            // Check if parent is a descendant of folder
            let state_guard = state.read();
            let config = state_guard.config.read();
            let mut current_parent = parent_id.clone();
            loop {
                if current_parent == folder_id {
                    return;
                }
                if let Some(folder) = config.folders.iter().find(|f| f.id == current_parent) {
                    if let Some(next_parent) = &folder.parent_id {
                        current_parent = next_parent.clone();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();

            if let Some(folder) = config.folders.iter_mut().find(|f| f.id == folder_id) {
                folder.parent_id = parent_folder_id;
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }

    #[allow(dead_code)]
    pub fn delete_folder_recursive(state: SharedRDState, folder_lnk: FolderLnk) {
        let mut folders_to_delete = vec![folder_lnk.id().to_string()];

        {
            let state_guard = state.read();
            let config = state_guard.config.read();
            let mut i = 0;
            while i < folders_to_delete.len() {
                let current_id = folders_to_delete[i].clone();
                for folder in &config.folders {
                    if folder.parent_id.as_ref() == Some(&current_id) {
                        folders_to_delete.push(folder.id.clone());
                    }
                }
                i += 1;
            }
        }

        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();

            config
                .folders
                .retain(|f| !folders_to_delete.contains(&f.id));
            config
                .folder_order
                .retain(|id| !folders_to_delete.contains(id));

            config
                .folder_assignments
                .retain(|_, fid| !folders_to_delete.contains(fid));
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
    }
}
