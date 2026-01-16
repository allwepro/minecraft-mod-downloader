use crate::app::*;
use crate::domain::*;
use crate::infra::DownloadMetadata;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AppState {
    pub minecraft_versions: Vec<MinecraftVersion>,
    pub mod_loaders: Vec<ModLoader>,
    pub mod_lists: Vec<ModList>,
    pub current_list_id: Option<String>,
    pub download_progress: HashMap<String, f32>,
    pub download_status: HashMap<String, DownloadStatus>,
    event_rx: mpsc::Receiver<Event>,
    pub search_window_results: Vec<Arc<ModInfo>>,
    pub mods_being_loaded: HashSet<String>,
    pub mods_failed_loading: HashSet<String>,
    pub legacy_state: LegacyState,
    pub pending_legacy_mods: Option<Vec<Arc<ModInfo>>>,
    pub search_filter_exact: bool,
    pub default_list_name: String,
    pub initial_loading: bool,
    loaders_by_type: HashMap<ProjectType, Vec<ModLoader>>,
    loaders_loading: HashSet<ProjectType>,
    pub effective_settings_cache: HashMap<String, (String, String, String)>,
    pub cached_mods: HashMap<(String, String, String), Arc<ModInfo>>,
    metadata_cache: HashMap<String, DownloadMetadata>,
}

impl AppState {
    pub fn new(event_rx: mpsc::Receiver<Event>) -> (Self, Vec<Effect>) {
        let state = Self {
            minecraft_versions: Vec::new(),
            mod_loaders: Vec::new(),
            mod_lists: Vec::new(),
            current_list_id: None,
            download_progress: HashMap::new(),
            download_status: HashMap::new(),
            event_rx,
            search_window_results: Vec::new(),
            mods_being_loaded: HashSet::new(),
            mods_failed_loading: HashSet::new(),
            legacy_state: LegacyState::Idle,
            pending_legacy_mods: None,
            search_filter_exact: true,
            default_list_name: "New List".to_string(),

            initial_loading: true,
            loaders_by_type: HashMap::new(),
            loaders_loading: HashSet::new(),
            effective_settings_cache: HashMap::new(),
            cached_mods: HashMap::new(),
            metadata_cache: HashMap::new(),
        };

        (state, vec![Effect::LoadInitialData])
    }

    pub fn loaders_for_type(&self, project_type: ProjectType) -> Option<&[ModLoader]> {
        self.loaders_by_type
            .get(&project_type)
            .map(|v| v.as_slice())
            .or_else(|| {
                if project_type == ProjectType::Mod {
                    Some(self.mod_loaders.as_slice())
                } else {
                    None
                }
            })
    }

    pub fn get_cached_mod(&self, mod_id: &str) -> Option<Arc<ModInfo>> {
        let version = self.get_effective_version();
        let loader = self.get_effective_loader();

        let key = (mod_id.to_string(), version.clone(), loader.clone());
        let result = self.cached_mods.get(&key).cloned();

        if result.is_none() {
            log::debug!(
                "Mod {} not in cache (have {} entries, looking for version={} loader={})",
                mod_id,
                self.cached_mods.len(),
                version,
                loader
            );
        }

        result
    }

    pub fn get_cached_mod_with_context(
        &self,
        mod_id: &str,
        version: &str,
        loader: &str,
    ) -> Option<Arc<ModInfo>> {
        let key = (mod_id.to_string(), version.to_string(), loader.to_string());
        self.cached_mods.get(&key).cloned()
    }

    pub fn ensure_loaders_for_type(&mut self, project_type: ProjectType) -> Vec<Effect> {
        if self.loaders_by_type.contains_key(&project_type) {
            return Vec::new();
        }
        if self.loaders_loading.contains(&project_type) {
            return Vec::new();
        }
        self.loaders_loading.insert(project_type);
        vec![Effect::LoadLoadersForType { project_type }]
    }

    pub fn is_loading_loaders_for_type(&self, project_type: ProjectType) -> bool {
        self.loaders_loading.contains(&project_type)
    }

    pub fn process_events(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();

        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Event::InitialDataLoaded {
                    mod_lists,
                    current_list_id,
                    minecraft_versions,
                    mod_loaders,
                    default_list_name,
                } => {
                    self.mod_lists = mod_lists;
                    self.current_list_id = current_list_id;
                    self.minecraft_versions = minecraft_versions;
                    self.mod_loaders = mod_loaders.clone();
                    self.default_list_name = default_list_name;
                    self.initial_loading = false;

                    self.loaders_by_type.insert(ProjectType::Mod, mod_loaders);
                    self.loaders_loading.remove(&ProjectType::Mod);

                    self.effective_settings_cache.clear();

                    let download_dir = self.get_effective_download_dir();
                    effects.push(Effect::ValidateMetadata { download_dir });

                    effects.extend(self.invalidate_and_reload());
                }

                Event::LoadersForTypeLoaded {
                    project_type,
                    loaders,
                } => {
                    self.loaders_by_type.insert(project_type, loaders);
                    self.loaders_loading.remove(&project_type);
                }
                Event::SearchResults(results) => {
                    let version = self.get_effective_version();
                    let loader = self.get_effective_loader();
                    for mod_info in &results {
                        let key = (mod_info.id.clone(), version.clone(), loader.clone());
                        self.cached_mods.insert(key, mod_info.clone());
                    }
                    self.search_window_results = results;
                }
                Event::ModDetails {
                    info: mod_info,
                    version,
                    loader,
                } => {
                    let mod_id = mod_info.id.clone();
                    log::debug!(
                        "ModDetails event for {}: version='{}', download_url='{}', fetched_for=({},{})",
                        mod_id,
                        mod_info.version,
                        mod_info.download_url,
                        version,
                        loader
                    );
                    let key = (mod_id.clone(), version, loader);
                    self.cached_mods.insert(key, mod_info);
                    self.mods_being_loaded.remove(&mod_id);
                }
                Event::ModDetailsFailed { mod_id } => {
                    self.mods_being_loaded.remove(&mod_id);
                    self.mods_failed_loading.insert(mod_id);
                }
                Event::DownloadProgress { mod_id, progress } => {
                    if progress > 0.0 {
                        self.download_status
                            .insert(mod_id.clone(), DownloadStatus::Downloading);
                    }
                    self.download_progress.insert(mod_id, progress);
                }
                Event::DownloadComplete { mod_id, success } => {
                    self.download_status.insert(
                        mod_id.clone(),
                        if success {
                            DownloadStatus::Complete
                        } else {
                            DownloadStatus::Failed
                        },
                    );

                    if success {
                        let download_dir = self.get_effective_download_dir();
                        effects.push(Effect::ValidateMetadata { download_dir });
                    }
                }
                Event::LegacyListProgress {
                    current,
                    total,
                    message,
                } => {
                    self.legacy_state = LegacyState::InProgress {
                        current,
                        total,
                        message,
                    };
                }
                Event::LegacyListComplete {
                    suggested_name,
                    successful,
                    failed,
                    warnings,
                    is_import: is_importable,
                } => {
                    let successful_ids = successful.iter().map(|m| m.id.clone()).collect();
                    self.pending_legacy_mods = Some(successful);
                    self.legacy_state = LegacyState::Complete {
                        suggested_name,
                        successful: successful_ids,
                        failed,
                        warnings,
                        is_import: is_importable,
                    };
                }
                Event::LegacyListFailed {
                    error,
                    is_import: is_importable,
                } => {
                    self.pending_legacy_mods = None;
                    self.legacy_state = LegacyState::Complete {
                        suggested_name: "".parse().unwrap(),
                        successful: Vec::new(),
                        failed: Vec::new(),
                        warnings: vec![error],
                        is_import: is_importable,
                    };
                }
                Event::MetadataLoaded {
                    download_dir,
                    metadata,
                } => {
                    self.metadata_cache.insert(download_dir, metadata);
                }
            }
        }

        effects
    }

    pub fn get_current_list(&self) -> Option<&ModList> {
        self.current_list_id
            .as_ref()
            .and_then(|id| self.mod_lists.iter().find(|l| &l.id == id))
    }

    pub fn get_list_by_id(&self, list_id: &str) -> Option<&ModList> {
        self.mod_lists.iter().find(|l| l.id == list_id)
    }

    pub fn get_current_list_mut(&mut self) -> Option<&mut ModList> {
        let current_id = self.current_list_id.clone();
        current_id
            .as_ref()
            .and_then(|id| self.mod_lists.iter_mut().find(|l| &l.id == id))
    }

    pub fn get_current_list_type(&self) -> ProjectType {
        self.get_current_list()
            .map(|l| l.content_type)
            .unwrap_or(ProjectType::Mod)
    }

    fn default_version_fallback(&self) -> String {
        self.minecraft_versions
            .first()
            .map(|v| v.id.clone())
            .unwrap_or_default()
    }

    fn default_loader_fallback(&self, project_type: ProjectType) -> String {
        self.loaders_for_type(project_type)
            .and_then(|loaders| loaders.first())
            .map(|l| l.id.clone())
            .unwrap_or_default()
    }

    fn default_dir_fallback(&self) -> String {
        dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("minecraft")
            .to_string_lossy()
            .to_string()
    }

    fn compute_effective_settings_for_list(&self, list: &ModList) -> (String, String, String) {
        let version = if list.version.is_empty() {
            self.default_version_fallback()
        } else {
            list.version.clone()
        };

        let loader = if list.loader.id.is_empty() {
            self.default_loader_fallback(list.content_type)
        } else {
            list.loader.id.clone()
        };

        let dir = if list.download_dir.is_empty() {
            self.default_dir_fallback()
        } else {
            list.download_dir.clone()
        };

        (version, loader, dir)
    }

    fn get_cached_effective_settings(&self, list_id: &str) -> Option<(String, String, String)> {
        self.effective_settings_cache.get(list_id).cloned()
    }

    pub fn get_effective_version(&self) -> String {
        let Some(list) = self.get_current_list() else {
            return self.default_version_fallback();
        };

        if let Some((v, _, _)) = self.get_cached_effective_settings(&list.id) {
            return v;
        }

        self.compute_effective_settings_for_list(list).0
    }

    pub fn get_effective_loader(&self) -> String {
        let Some(list) = self.get_current_list() else {
            return self.default_loader_fallback(ProjectType::Mod);
        };

        if let Some((_, l, _)) = self.get_cached_effective_settings(&list.id) {
            return l;
        }

        self.compute_effective_settings_for_list(list).1
    }

    pub fn get_effective_download_dir(&self) -> String {
        let Some(list) = self.get_current_list() else {
            return self.default_dir_fallback();
        };

        if let Some((_, _, d)) = self.get_cached_effective_settings(&list.id) {
            return d;
        }

        self.compute_effective_settings_for_list(list).2
    }

    pub fn invalidate_and_reload(&mut self) -> Vec<Effect> {
        let current_version = self.get_effective_version();
        let current_loader = self.get_effective_loader();

        log::info!(
            "=== Invalidate and reload for version={current_version} loader={current_loader} ==="
        );
        log::info!(
            "Cache has {} entries before reload:",
            self.cached_mods.len()
        );
        for ((id, ver, ldr), info) in &self.cached_mods {
            log::info!("  - {} : {}/{} (version={})", id, ver, ldr, info.version);
        }

        self.mods_being_loaded.clear();
        self.mods_failed_loading.clear();
        // Don't clear cached_mods! It holds data for multiple version/loader combinations. This allows switching between versions without refetching.

        let mod_ids: Vec<String> = self
            .get_current_list()
            .map(|l| l.mods.iter().map(|e| e.mod_id.clone()).collect())
            .unwrap_or_default();

        log::info!(
            "Reloading {} mods for {}/{}",
            mod_ids.len(),
            current_version,
            current_loader
        );

        let mut effects = Vec::new();
        for mod_id in mod_ids {
            effects.extend(self.load_mod_details_if_needed(&mod_id));
        }

        log::info!(
            "=== Reload complete, {} effects generated ===",
            effects.len()
        );

        effects
    }

    pub fn load_mod_details_if_needed(&mut self, mod_id: &str) -> Vec<Effect> {
        if self.mods_being_loaded.contains(mod_id) {
            log::debug!("Mod {mod_id} already being loaded");
            return Vec::new();
        }
        if self.mods_failed_loading.contains(mod_id) {
            log::debug!("Mod {mod_id} previously failed");
            return Vec::new();
        }

        let version = self.get_effective_version();
        let loader = self.get_effective_loader();

        let key = (mod_id.to_string(), version.clone(), loader.clone());

        if let Some(info) = self.cached_mods.get(&key) {
            if !info.version.is_empty() {
                log::debug!(
                    "Mod {mod_id} has complete cached info for {version}/{loader}, skipping fetch"
                );
                return Vec::new();
            }
            log::debug!("Mod {mod_id} has incomplete info (empty version), fetching details");
        } else {
            log::debug!("Mod {mod_id} not in cache for {version}/{loader}, fetching");
        }

        self.mods_being_loaded.insert(mod_id.to_string());
        log::debug!("Triggering fetch for mod {mod_id} with version={version} loader={loader}");

        vec![Effect::FetchModDetails {
            mod_id: mod_id.to_string(),
            version,
            loader,
        }]
    }

    pub fn force_reload_mod(&mut self, mod_id: &str) -> Vec<Effect> {
        self.mods_failed_loading.remove(mod_id);
        self.mods_being_loaded.remove(mod_id);
        self.load_mod_details_if_needed(mod_id)
    }

    pub fn start_download(&mut self, mod_id: &str) -> Vec<Effect> {
        self.download_status
            .insert(mod_id.to_string(), DownloadStatus::Queued);
        self.download_progress.insert(mod_id.to_string(), 0.0);

        if let Some(mod_info) = self.get_cached_mod(mod_id) {
            return vec![Effect::DownloadMod {
                mod_info,
                download_dir: self.get_effective_download_dir(),
            }];
        }

        Vec::new()
    }

    pub fn perform_search(&self, query: &str) -> Vec<Effect> {
        if query.is_empty() {
            return Vec::new();
        }

        let current_type = self.get_current_list_type();

        vec![Effect::SearchMods {
            query: query.to_string(),
            version: if self.search_filter_exact {
                self.get_effective_version()
            } else {
                String::new()
            },
            loader: if self.search_filter_exact
                && (current_type == ProjectType::Mod
                    || current_type == ProjectType::Shader
                    || current_type == ProjectType::Plugin)
            {
                self.get_effective_loader()
            } else {
                String::new()
            },
            project_type: current_type,
        }]
    }

    pub fn add_mod_to_current_list(&mut self, mod_info: Arc<ModInfo>) -> Vec<Effect> {
        let mut list_to_save = None;

        if let Some(current_list) = self.get_current_list_mut()
            && !current_list.mods.iter().any(|e| e.mod_id == mod_info.id)
        {
            current_list.mods.push(ModEntry {
                mod_id: mod_info.id.clone(),
                mod_name: mod_info.name.clone(),
                added_at: Utc::now(),
                archived: false,
                compatibility_override: false,
            });
            list_to_save = Some(current_list.clone());
        }

        if list_to_save.is_some() {
            self.download_status
                .insert(mod_info.id.clone(), DownloadStatus::Idle);
        }

        list_to_save
            .map(|list| vec![Effect::SaveList { list }])
            .unwrap_or_default()
    }

    pub fn delete_mod(&mut self, mod_id: &str) -> Vec<Effect> {
        let mut effects = Vec::new();

        let download_dir = self.get_effective_download_dir();

        if let Some(current_list) = self.get_current_list_mut() {
            current_list.mods.retain(|e| e.mod_id != mod_id);
            effects.push(Effect::SaveList {
                list: current_list.clone(),
            });
        }

        self.mods_being_loaded.remove(mod_id);
        self.mods_failed_loading.remove(mod_id);
        self.download_progress.remove(mod_id);
        self.download_status.remove(mod_id);

        effects.push(Effect::RemoveFromMetadata {
            download_dir: download_dir.clone(),
            mod_id: mod_id.to_string(),
        });
        effects.push(Effect::DeleteModFile {
            download_dir: download_dir.clone(),
            mod_id: mod_id.to_string(),
        });

        effects
    }

    pub fn toggle_archive_mod(&mut self, mod_id: &str) -> Vec<Effect> {
        let download_dir = self.get_effective_download_dir();

        if let Some(list) = self.get_current_list_mut()
            && let Some(entry) = list.mods.iter_mut().find(|e| e.mod_id == mod_id)
        {
            entry.archived = !entry.archived;
            let is_archived = entry.archived;

            let list_clone = list.clone();
            let mut effects = vec![Effect::SaveList { list: list_clone }];

            if is_archived {
                effects.push(Effect::ArchiveModFile {
                    download_dir,
                    mod_id: mod_id.to_string(),
                });
            } else {
                effects.push(Effect::UnarchiveModFile {
                    download_dir,
                    mod_id: mod_id.to_string(),
                });
            }
            return effects;
        }
        Vec::new()
    }

    pub fn toggle_compatibility_override(&mut self, mod_id: &str) -> Vec<Effect> {
        if let Some(list) = self.get_current_list_mut()
            && let Some(entry) = list.mods.iter_mut().find(|e| e.mod_id == mod_id)
        {
            entry.compatibility_override = !entry.compatibility_override;
            return vec![Effect::SaveList { list: list.clone() }];
        }
        Vec::new()
    }

    pub fn has_compatibility_override(&self, mod_id: &str) -> bool {
        if let Some(list) = self.get_current_list()
            && let Some(entry) = list.mods.iter().find(|e| e.mod_id == mod_id)
        {
            return entry.compatibility_override;
        }
        false
    }

    pub fn delete_current_list(&mut self) -> Vec<Effect> {
        if let Some(list_id) = self.current_list_id.clone() {
            self.mod_lists.retain(|l| l.id != list_id);
            self.current_list_id = None;
            return vec![Effect::DeleteList { list_id }];
        }
        Vec::new()
    }

    pub fn export_current_list(&mut self, path: std::path::PathBuf) -> Vec<Effect> {
        let export_info = self.get_current_list().map(|list| {
            (
                list.mods
                    .iter()
                    .map(|m| m.mod_id.clone())
                    .collect::<Vec<String>>(),
                list.clone(),
            )
        });

        let (mod_ids, current_list_obj) = match export_info {
            Some(data) => data,
            None => return Vec::new(),
        };

        match path.extension().and_then(|s| s.to_str()) {
            Some("mods") => {
                self.legacy_state = LegacyState::InProgress {
                    current: 0,
                    total: mod_ids.len(),
                    message: "Initializing export...".into(),
                };

                vec![Effect::LegacyListExport {
                    path,
                    mod_ids,
                    version: self.get_effective_version(),
                    loader: self.get_effective_loader(),
                }]
            }
            _ => vec![Effect::ExportListToml {
                path,
                list: current_list_obj,
            }],
        }
    }

    pub fn start_legacy_import(&mut self, path: std::path::PathBuf) -> Vec<Effect> {
        self.legacy_state = LegacyState::InProgress {
            current: 0,
            total: 0,
            message: "Preparing import...".into(),
        };

        vec![Effect::LegacyListImport {
            path,
            version: self.get_effective_version(),
            loader: self.get_effective_loader(),
        }]
    }

    pub fn finalize_import(&mut self, list: ModList) -> Vec<Effect> {
        self.current_list_id = Some(list.id.clone());
        self.mod_lists.push(list.clone());

        vec![Effect::SaveList { list }]
    }

    pub fn create_new_list(
        &mut self,
        new_name: String,
        content_type: ProjectType,
        version: String,
        loader: String,
        download_dir: String,
    ) -> Vec<Effect> {
        let list_name = new_name.to_string();

        let loader_obj = self
            .loaders_for_type(content_type)
            .and_then(|loaders| loaders.iter().find(|l| l.id == loader).cloned())
            .or_else(|| self.mod_loaders.iter().find(|l| l.id == loader).cloned())
            .unwrap_or(crate::domain::ModLoader {
                id: loader.clone(),
                name: loader.clone(),
            });

        let new_list = ModList {
            id: format!("list_{}", Utc::now().timestamp()),
            name: list_name,
            created_at: Utc::now(),
            mods: Vec::new(),
            version,
            loader: loader_obj,
            download_dir,
            content_type,
        };

        self.current_list_id = Some(new_list.id.clone());
        self.mod_lists.push(new_list.clone());

        vec![Effect::SaveList { list: new_list }]
    }

    pub fn is_mod_compatible(&self, mod_id: &str) -> Option<bool> {
        if let Some(list) = self.get_current_list()
            && let Some(entry) = list.mods.iter().find(|e| e.mod_id == mod_id)
            && entry.compatibility_override
        {
            return Some(true);
        }

        let version = self.get_effective_version();
        let loader = self.get_effective_loader();
        self.is_mod_compatible_with_context(mod_id, &version, &loader)
    }

    pub fn is_mod_compatible_raw(&self, mod_id: &str) -> Option<bool> {
        let version = self.get_effective_version();
        let loader = self.get_effective_loader();
        self.is_mod_compatible_with_context(mod_id, &version, &loader)
    }

    pub fn is_mod_compatible_with_context(
        &self,
        mod_id: &str,
        version: &str,
        loader: &str,
    ) -> Option<bool> {
        let info = self.get_cached_mod_with_context(mod_id, version, loader)?;

        let version_ok = info.supported_versions.is_empty()
            || info.supported_versions.iter().any(|v| v == version);
        let loader_ok =
            info.supported_loaders.is_empty() || info.supported_loaders.iter().any(|l| l == loader);

        Some(version_ok && loader_ok)
    }

    pub fn get_filtered_mods(
        &self,
        query: &str,
        sort_mode: SortMode,
        order_mode: OrderMode,
        filter_mode: FilterMode,
    ) -> Vec<ModEntry> {
        let query = query.to_lowercase();
        let effective_version = self.get_effective_version();
        let effective_loader = self.get_effective_loader();

        let mut mods: Vec<ModEntry> = self
            .get_current_list()
            .map(|l| l.mods.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                if query.is_empty() {
                    return true;
                }

                if entry.mod_name.to_lowercase().contains(&query) {
                    return true;
                }

                if let Some(info) = self.get_cached_mod(&entry.mod_id) {
                    info.description.to_lowercase().contains(&query)
                } else {
                    false
                }
            })
            .collect();

        if matches!(
            filter_mode,
            FilterMode::CompatibleOnly | FilterMode::IncompatibleOnly | FilterMode::MissingOnly
        ) {
            mods.retain(|entry| {
                let comp = self
                    .is_mod_compatible_with_context(
                        &entry.mod_id,
                        &effective_version,
                        &effective_loader,
                    )
                    .unwrap_or(true);

                let missing = !entry.archived
                    && (!self.is_mod_downloaded(&entry.mod_id)
                        || self.is_mod_updateable(&entry.mod_id));

                match filter_mode {
                    FilterMode::MissingOnly => missing,
                    FilterMode::CompatibleOnly => comp,
                    FilterMode::IncompatibleOnly => !comp,
                    FilterMode::All => true,
                }
            });
        }

        mods.sort_by(|a, b| match sort_mode {
            SortMode::Name => a.mod_name.to_lowercase().cmp(&b.mod_name.to_lowercase()),
            SortMode::DateAdded => a.added_at.cmp(&b.added_at),
        });

        if matches!(order_mode, OrderMode::Descending) {
            mods.reverse();
        }

        mods
    }

    pub fn is_mod_downloaded(&self, mod_id: &str) -> bool {
        let download_dir = self.get_effective_download_dir();

        if let Some(metadata) = self.metadata_cache.get(&download_dir)
            && let Some(entry) = metadata.get_entry(mod_id)
        {
            let file_path = std::path::Path::new(&download_dir).join(&entry.file);
            return file_path.exists();
        }

        false
    }

    pub fn has_download_metadata(&self, mod_id: &str) -> bool {
        let download_dir = self.get_effective_download_dir();
        if let Some(metadata) = self.metadata_cache.get(&download_dir) {
            metadata.get_entry(mod_id).is_some()
        } else {
            false
        }
    }

    pub fn is_mod_updateable(&self, mod_id: &str) -> bool {
        let Some(mod_info) = self.get_cached_mod(mod_id) else {
            return false;
        };

        let download_dir = self.get_effective_download_dir();

        if let Some(metadata) = self.metadata_cache.get(&download_dir)
            && let Some(entry) = metadata.get_entry(mod_id)
        {
            let file_path = std::path::Path::new(&download_dir).join(&entry.file);
            return file_path.exists() && entry.version != mod_info.version;
        }

        false
    }

    pub fn get_missing_mod_ids(&self, filtered_mods: &[ModEntry]) -> Vec<String> {
        let download_dir = self.get_effective_download_dir();
        let effective_version = self.get_effective_version();
        let effective_loader = self.get_effective_loader();

        let metadata = self.metadata_cache.get(&download_dir);

        filtered_mods
            .iter()
            .filter(|entry| !entry.archived)
            .filter(|entry| {
                if self.mods_being_loaded.contains(&entry.mod_id) {
                    return false;
                }

                if let Some(status) = self.download_status.get(&entry.mod_id)
                    && matches!(status, DownloadStatus::Queued | DownloadStatus::Downloading)
                {
                    return false;
                }

                if !entry.compatibility_override
                    && !self
                        .is_mod_compatible_with_context(
                            &entry.mod_id,
                            &effective_version,
                            &effective_loader,
                        )
                        .unwrap_or(false)
                {
                    return false;
                }

                if let Some(mod_info) = self.get_cached_mod(&entry.mod_id) {
                    if let Some(meta) = metadata
                        && let Some(entry) = meta.get_entry(&mod_info.id)
                    {
                        let file_path = std::path::Path::new(&download_dir).join(&entry.file);
                        return !file_path.exists() || entry.version != mod_info.version;
                    }

                    let filename = generate_mod_filename(&mod_info);
                    let file_path = std::path::Path::new(&download_dir).join(&filename);
                    !file_path.exists()
                } else {
                    false
                }
            })
            .map(|entry| entry.mod_id.clone())
            .collect()
    }

    pub fn get_unknown_mod_files(&self) -> Vec<String> {
        let download_dir = self.get_effective_download_dir();
        let download_path = std::path::Path::new(&download_dir);

        if !download_path.exists() {
            return Vec::new();
        }

        let metadata = self.metadata_cache.get(&download_dir);

        if metadata.is_none() {
            log::debug!(
                "Metadata not loaded yet for {download_dir}, skipping unknown file detection"
            );
            return Vec::new();
        }

        let current_list = self.get_current_list();

        let known_filenames: HashSet<String> = if let Some(meta) = metadata {
            meta.mods.values().map(|entry| entry.file.clone()).collect()
        } else {
            HashSet::new()
        };

        log::debug!(
            "Unknown file detection: metadata has {} entries with {} unique filenames",
            metadata.map(|m| m.mods.len()).unwrap_or(0),
            known_filenames.len()
        );

        let mut unknown_files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(download_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_file()
                    && let Some(filename) = entry.file_name().to_str()
                {
                    if filename.starts_with(".mcd") {
                        continue;
                    }

                    let current_type = current_list.map(|l| l.content_type).unwrap_or_default();
                    let expected_ext = current_type.fileext();
                    let base_filename = filename.strip_suffix(".archived").unwrap_or(filename);
                    if !base_filename.ends_with(&format!(".{expected_ext}")) {
                        continue;
                    }

                    let is_known = known_filenames.contains(filename);

                    if !is_known {
                        log::debug!("Found unknown file: {filename}");
                        unknown_files.push(filename.to_string());
                    }
                }
            }
        }

        unknown_files.sort();
        unknown_files
    }
}
