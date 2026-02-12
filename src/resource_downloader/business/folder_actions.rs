use crate::resource_downloader::business::{Effect, SharedRDState};
use crate::resource_downloader::domain::{Folder, FolderLnk};
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

    pub fn create_folder(state: SharedRDState, name: String) -> FolderLnk {
        let folder_id = Self::generate_folder_id();
        let folder = Folder {
            id: folder_id.clone(),
            name,
            collapsed: false,
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
        let new_folder_id = Self::generate_folder_id();

        {
            let state_guard = state.write();
            let mut config = state_guard.config.write();

            if let Some(original) = config.folders.iter().find(|f| f.id == folder_lnk.id()) {
                let new_folder = Folder {
                    id: new_folder_id.clone(),
                    name: format!("{} (Copy)", original.name),
                    collapsed: original.collapsed,
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
            }
        }

        let state_guard = state.read();
        state_guard.dispatch(Effect::SaveConfig {
            config: state_guard.config.read().clone(),
        });
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
}
