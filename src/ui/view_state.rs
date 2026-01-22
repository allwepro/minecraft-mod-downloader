use crate::app::{FilterMode, OrderMode, SortMode};
use crate::domain::{ModInfo, ModList, ProjectType};
use std::sync::Arc;

pub struct ViewState {
    // Launcher UI state
    pub launcher_open: bool,

    // List management UI state
    pub list_search_query: String,
    pub show_rename_input: bool,
    pub rename_list_input: String,

    // Mod search UI state
    pub search_query: String,
    pub selected_mod: Option<usize>,

    // Window states
    pub search_window_open: bool,
    pub search_window_query: String,
    pub is_searching: bool,
    pub settings_window_open: bool,
    pub import_window_open: bool,
    pub create_list_window_open: bool,
    pub list_settings_open: bool,
    pub legacy_import_settings_open: bool,

    // Import/Export state
    pub import_name_input: String,
    pub active_action: crate::app::ListAction,
    pub pending_import_list: Option<ModList>,

    // Sort and filter state
    pub sort_menu_open: bool,
    pub current_sort_mode: SortMode,
    pub current_filter_mode: FilterMode,
    pub current_order_mode: OrderMode,
    pub sort_btn_rect: egui::Rect,
    pub sort_popup_rect: egui::Rect,
    pub show_archived: bool,
    pub show_unknown_mods: bool,
    pub scroll_to_mod_id: Option<String>,

    // List settings inputs
    pub list_settings_initial_load: bool,
    pub list_settings_version: String,
    pub list_settings_loader: String,
    pub list_settings_dir: String,

    // App settings inputs
    pub app_settings_default_name: String,

    // Create list inputs
    pub new_list_name: String,
    pub new_list_type: ProjectType,
    pub new_list_version: String,
    pub new_list_loader: String,
    pub new_list_dir: String,
    pub new_list_dir_edited: bool,

    // Legacy import/export
    pub legacy_import_version: String,
    pub legacy_import_loader: String,
    pub legacy_import_dir: String,
    pub legacy_import_mods: Option<Vec<Arc<ModInfo>>>,
    pub legacy_import_name: String,

    // Collection import
    pub collection_import_window_open: bool,
    pub collection_import_input: String,
    pub collection_import_loading: bool,
    pub collection_import_finalizing: bool,
    pub collection_import_error: Option<String>,
    pub collection_import_selected_type: ProjectType,

    // Import popup state
    pub import_popup_open: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            launcher_open: false,
            list_search_query: String::new(),
            show_rename_input: false,
            rename_list_input: String::new(),
            search_query: String::new(),
            selected_mod: None,
            search_window_open: false,
            search_window_query: String::new(),
            is_searching: false,
            settings_window_open: false,
            import_window_open: false,
            create_list_window_open: false,
            list_settings_open: false,
            legacy_import_settings_open: false,
            import_name_input: String::new(),
            active_action: Default::default(),
            pending_import_list: None,
            sort_menu_open: false,
            current_sort_mode: SortMode::default(),
            current_filter_mode: FilterMode::default(),
            current_order_mode: OrderMode::default(),
            sort_btn_rect: egui::Rect::NOTHING,
            sort_popup_rect: egui::Rect::NOTHING,
            show_archived: false,
            show_unknown_mods: false,
            scroll_to_mod_id: None,
            list_settings_initial_load: true,
            list_settings_version: String::new(),
            list_settings_loader: String::new(),
            list_settings_dir: String::new(),
            new_list_dir_edited: false,
            app_settings_default_name: String::new(),
            new_list_name: String::new(),
            new_list_type: ProjectType::default(),
            new_list_version: String::new(),
            new_list_loader: String::new(),
            new_list_dir: String::new(),
            legacy_import_version: String::new(),
            legacy_import_loader: String::new(),
            legacy_import_dir: String::new(),
            legacy_import_mods: None,
            legacy_import_name: String::new(),
            collection_import_window_open: false,
            collection_import_input: String::new(),
            collection_import_loading: false,
            collection_import_finalizing: false,
            collection_import_error: None,
            collection_import_selected_type: ProjectType::Mod,
            import_popup_open: false,
        }
    }
}

impl ViewState {
    pub fn close_all_windows(&mut self) {
        self.settings_window_open = false;
        self.search_window_open = false;
        self.import_window_open = false;
        self.sort_menu_open = false;
        self.list_settings_open = false;
        self.create_list_window_open = false;
        self.legacy_import_settings_open = false;
        self.collection_import_window_open = false;
    }

    pub fn reset_list_settings(&mut self) {
        self.list_settings_version.clear();
        self.list_settings_loader.clear();
        self.list_settings_dir.clear();
    }

    pub fn reset_create_list(&mut self) {
        self.new_list_name.clear();
        self.new_list_type = ProjectType::Mod;
        self.new_list_version.clear();
        self.new_list_loader.clear();
        self.new_list_dir.clear();
    }

    pub fn reset_legacy_import(&mut self) {
        self.legacy_import_version.clear();
        self.legacy_import_loader.clear();
        self.legacy_import_dir.clear();
        self.legacy_import_mods = None;
        self.legacy_import_name.clear();
    }

    pub fn reset_collection_import(&mut self) {
        self.collection_import_input.clear();
        self.collection_import_loading = false;
        self.collection_import_finalizing = false;
        self.collection_import_error = None;
        self.collection_import_selected_type = ProjectType::Mod;
    }
}
