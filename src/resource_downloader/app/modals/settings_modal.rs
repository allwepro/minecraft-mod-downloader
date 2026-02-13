use crate::common::ui::structs::modal_window::ModalWindow;
use crate::resource_downloader::business::xcache::CacheType;
use crate::resource_downloader::business::{Effect, SharedRDState};
use egui::{Id, Ui};

#[derive(Clone)]
pub struct SettingsModal {
    state: SharedRDState,
    save_on_close: bool,
    default_list_name: String,
    default_list_group_name: String,
    show_advanced_options: bool,
    cache_types: (bool, bool, bool, bool, bool, bool, bool, bool),
}

impl SettingsModal {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state,
            save_on_close: false,
            default_list_name: String::new(),
            default_list_group_name: String::new(),
            show_advanced_options: false,
            cache_types: (false, false, false, false, false, false, false, false),
        }
    }
}

impl ModalWindow for SettingsModal {
    fn id(&self) -> Id {
        Id::new("settings")
    }

    fn title(&self) -> String {
        "Resource Downloader Settings".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.label("Default list name:");
        ui.text_edit_singleline(&mut self.default_list_name);

        ui.label("Default list group name:");
        ui.text_edit_singleline(&mut self.default_list_group_name);

        ui.add_space(10.0);

        if ui.button("💾 Save").clicked() {
            self.save_on_close = true;
            *open = false;
        }

        ui.add_space(20.0);
        if ui
            .button(format!(
                "{} Advanced",
                if self.show_advanced_options {
                    "🔽"
                } else {
                    "▶"
                }
            ))
            .clicked()
        {
            self.show_advanced_options = !self.show_advanced_options;
        }
        if self.show_advanced_options {
            ui.add_space(5.0);
            self.render_advanced_options(ui);
        }
        ui.add_space(5.0);
    }

    fn on_open(&mut self) {
        self.save_on_close = false;
        let state = self.state.read();
        let config = state.config.read();
        self.default_list_name = config.default_list_name.clone();
        self.default_list_group_name = config.default_list_group_name.clone();
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            return;
        }
        let state = self.state.write();
        {
            let mut config = state.config.write();
            config.default_list_name = self.default_list_name.clone();
            config.default_list_group_name = self.default_list_group_name.clone();
        }
        state.save_config();
    }
}

impl SettingsModal {
    fn render_advanced_options(&mut self, ui: &mut Ui) {
        ui.strong("Cache Management");
        ui.weak(
            "Cache contains search results, icons, versions, and more. Clearing \
         it can resolve loading issues but will require re-fetching data.",
        );
        ui.add_space(3.0);
        ui.checkbox(&mut self.cache_types.0, "Game Loader Cache");
        ui.checkbox(&mut self.cache_types.1, "Game Version Cache");
        ui.checkbox(&mut self.cache_types.2, "Slug Cache");
        ui.checkbox(&mut self.cache_types.3, "Metadata Cache");
        ui.checkbox(&mut self.cache_types.4, "Versions Cache");
        ui.checkbox(&mut self.cache_types.5, "Icons Cache");
        ui.checkbox(&mut self.cache_types.6, "Artifact Cache");
        ui.checkbox(&mut self.cache_types.7, "File Index Cache");
        ui.add_space(3.0);
        if self.cache_types.0 || self.cache_types.1 {
            ui.label("⚠ Clearing game version or loader cache requires restarting the app to take effect.");
            ui.add_space(3.0);
        }
        if ui.button("🗑 Clear Cache").clicked() {
            {
                let mut to_clear_wo_mem = Vec::new();
                if self.cache_types.0 {
                    to_clear_wo_mem.push(CacheType::GameLoaders);
                }
                if self.cache_types.1 {
                    to_clear_wo_mem.push(CacheType::GameVersions);
                }
                self.state
                    .read()
                    .api()
                    .clear_core_cache(to_clear_wo_mem, false);
            }
            {
                let mut to_clear_in_mem = Vec::new();
                if self.cache_types.2 {
                    to_clear_in_mem.push(CacheType::ProjectSlug);
                }
                if self.cache_types.3 {
                    to_clear_in_mem.push(CacheType::ProjectMetadata);
                }
                if self.cache_types.4 {
                    to_clear_in_mem.push(CacheType::ProjectVersions);
                }
                if self.cache_types.5 {
                    to_clear_in_mem.push(CacheType::ProjectIcons);
                    self.state.read().api().icon_pool.clear_gpu_cache();
                }
                self.state
                    .read()
                    .api()
                    .clear_core_cache(to_clear_in_mem, true);
            }
            if self.cache_types.6 {
                self.state.read().api().artifact_cache.delete_all();
            }
            if self.cache_types.7 {
                self.state.read().dispatch(Effect::ClearFileIndexCache);
            }
        }
    }
}
