use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::domain::ResourceType::Mod as RTMod;
use parking_lot::RwLock;
use std::sync::Arc;

pub type SharedRMAPI = Arc<RwLock<RMAPI>>;

#[allow(dead_code)]
pub struct List {
    pub id: String,
    pub name: String,
    pub count: usize,
    pub loader: String,
    pub download_dir: String,
    pub mods: Vec<Mod>,
}

#[allow(dead_code)]
pub struct Mod {
    pub id: String,
    pub mod_name: String,
    pub compatibility_override: bool,
}

#[allow(clippy::upper_case_acronyms)]
pub struct RMAPI {
    state: Option<SharedRDState>,
}

impl RMAPI {
    pub fn new() -> Self {
        Self { state: None }
    }

    pub fn set_state(&mut self, state: SharedRDState) {
        self.state = Some(state);
    }

    pub fn all_lists(&self) -> Vec<List> {
        if let Some(state) = self.state.as_ref() {
            state
                .read()
                .list_pool
                .map_filter(|list| {
                    if ListActions::get_list_target_resource_type(list) == RTMod {
                        Some(List {
                            id: list.get_id(),
                            name: list.get_name(),
                            count: list.count_manual_projects_by_type(RTMod),
                            loader: list
                                .get_resource_type_config(&RTMod)
                                .unwrap()
                                .loader
                                .name
                                .clone(),
                            download_dir: list
                                .get_resource_type_config(&RTMod)
                                .unwrap()
                                .download_dir
                                .clone(),
                            mods: list
                                .manual_projects_by_type(RTMod)
                                .iter()
                                .map(|p| Mod {
                                    id: p.get_id(),
                                    mod_name: p.get_name().clone(),
                                    compatibility_override: p.is_compatibility_overruled(),
                                })
                                .collect(),
                        })
                    } else {
                        None
                    }
                })
                .into_iter()
                .collect()
        } else {
            vec![]
        }
    }

    pub fn current_list_id(&self) -> Option<String> {
        if let Some(state) = self.state.as_ref() {
            state.read().open_list.as_ref().map(|l| l.to_context_id())
        } else {
            None
        }
    }
}
