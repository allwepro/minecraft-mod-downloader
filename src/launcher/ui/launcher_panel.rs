use crate::infra::ConfigManager;
use crate::launcher::ui::{JavaDownloadWindow, MinecraftDownloadWindow};
use crate::launcher::{
    AdvancedLauncher, FabricInstaller, JavaDetector, JavaInstallation, LaunchConfig, LaunchProfile,
    LaunchResult, MinecraftDetector, MinecraftInstallation, ModCopier,
};
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

enum PanelMessage {
    Status(String),
    LaunchFinished(Result<LaunchResult, String>),
    FabricReady {
        mc_version: String,
        version_id: String,
    },
    FabricFailed {
        mc_version: String,
        error: String,
    },
    FabricSupportResult {
        mc_version: String,
        supported: bool,
        error: Option<String>,
    },
}

pub struct LauncherPanel {
    java_installations: Vec<JavaInstallation>,
    selected_java_index: Option<usize>,
    minecraft_installation: Option<MinecraftInstallation>,
    launcher_username: String,
    launcher_min_memory: u32,
    launcher_max_memory: u32,
    launch_status: Option<String>,
    launch_in_progress: bool,
    selected_mc_version: String,
    java_download_window: JavaDownloadWindow,
    minecraft_download_window: MinecraftDownloadWindow,
    panel_sender: mpsc::Sender<PanelMessage>,
    panel_receiver: mpsc::Receiver<PanelMessage>,
    fabric_installing: bool,
    fabric_progress: Arc<Mutex<(f32, String)>>,
    fabric_version_id: Option<String>,
    fabric_for_mc_version: Option<String>,
    fabric_error: Option<String>,
    fabric_support_checking: bool,
    fabric_supported: Option<bool>,
    fabric_support_for_mc_version: Option<String>,
    fabric_support_error: Option<String>,
}

impl LauncherPanel {
    pub fn new() -> Self {
        let java_installations = JavaDetector::detect_java_installations();
        let minecraft_installation = MinecraftDetector::detect_minecraft();

        let selected_mc_version = minecraft_installation
            .as_ref()
            .and_then(|mc| mc.available_versions.first().cloned())
            .unwrap_or_else(|| "1.20.1".to_string());

        let (panel_tx, panel_rx) = mpsc::channel(20);

        Self {
            java_installations,
            selected_java_index: None,
            minecraft_installation,
            launcher_username: whoami::username(),
            launcher_min_memory: 1024,
            launcher_max_memory: 4096,
            launch_status: None,
            launch_in_progress: false,
            selected_mc_version,
            java_download_window: JavaDownloadWindow::new(),
            minecraft_download_window: MinecraftDownloadWindow::new(),
            panel_sender: panel_tx,
            panel_receiver: panel_rx,
            fabric_installing: false,
            fabric_progress: Arc::new(Mutex::new((0.0, String::new()))),
            fabric_version_id: None,
            fabric_for_mc_version: None,
            fabric_error: None,
            fabric_support_checking: false,
            fabric_supported: None,
            fabric_support_for_mc_version: None,
            fabric_support_error: None,
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config_manager: &Arc<ConfigManager>,
        rt_handle: &tokio::runtime::Handle,
        current_list_id: &Option<String>,
        download_dir: &str,
        selected_loader: &str,
    ) {
        self.process_panel_messages();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_content(
                    ui,
                    config_manager,
                    rt_handle,
                    current_list_id,
                    download_dir,
                    selected_loader,
                );
            });
        });

        if self.java_download_window.is_open() {
            if self.java_download_window.show(ctx, rt_handle) {
                // Reload java installations if download completed
                self.java_installations = JavaDetector::detect_java_installations();
                // Try to select the newly installed one (simplistic heuristic: select last or one with "jdk" in path)
                self.selected_java_index = self.java_installations.iter().position(|j| j.is_valid);
            }
        }

        if self.minecraft_download_window.is_open() {
            if self.minecraft_download_window.show(ctx, rt_handle) {
                // Reload minecraft installations if download completed
                self.minecraft_installation = MinecraftDetector::detect_minecraft();
                if let Some(mc) = &self.minecraft_installation {
                    if !mc.available_versions.contains(&self.selected_mc_version) {
                        if let Some(first) = mc.available_versions.first().cloned() {
                            self.selected_mc_version = first;
                        }
                    }
                }
            }
        }
    }

    fn process_panel_messages(&mut self) {
        while let Ok(msg) = self.panel_receiver.try_recv() {
            match msg {
                PanelMessage::Status(status) => {
                    self.launch_status = Some(status);
                }
                PanelMessage::LaunchFinished(result) => {
                    self.launch_in_progress = false;
                    match result {
                        Ok(LaunchResult::Success { pid }) => {
                            self.launch_status = Some(format!(
                                "✅ Minecraft launched successfully! (PID: {})",
                                pid
                            ));
                        }
                        Ok(LaunchResult::Failed { error }) => {
                            self.launch_status = Some(format!("❌ Launch failed: {}", error));
                        }
                        Err(error) => {
                            self.launch_status = Some(format!("❌ Error: {}", error));
                        }
                    }
                }
                PanelMessage::FabricReady {
                    mc_version,
                    version_id,
                } => {
                    if self.fabric_for_mc_version.as_deref() == Some(&mc_version) {
                        self.fabric_version_id = Some(version_id);
                        self.fabric_installing = false;
                        self.fabric_error = None;
                        if let Ok(mut guard) = self.fabric_progress.lock() {
                            *guard = (1.0, "Fabric ready".to_string());
                        }
                    }
                }
                PanelMessage::FabricFailed { mc_version, error } => {
                    if self.fabric_for_mc_version.as_deref() == Some(&mc_version) {
                        self.fabric_installing = false;
                        self.fabric_error = Some(error);
                        self.fabric_version_id = None;
                        if let Ok(mut guard) = self.fabric_progress.lock() {
                            *guard = (0.0, "Fabric install failed".to_string());
                        }
                    }
                }
                PanelMessage::FabricSupportResult {
                    mc_version,
                    supported,
                    error,
                } => {
                    if self.fabric_support_for_mc_version.as_deref() == Some(&mc_version) {
                        self.fabric_support_checking = false;
                        self.fabric_supported = Some(supported);
                        self.fabric_support_error = error;

                        if !supported {
                            self.fabric_installing = false;
                            self.fabric_version_id = None;
                            self.fabric_error = None;
                            if let Ok(mut guard) = self.fabric_progress.lock() {
                                *guard = (0.0, "Fabric not supported for this version".to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_content(
        &mut self,
        ui: &mut egui::Ui,
        config_manager: &Arc<ConfigManager>,
        rt_handle: &tokio::runtime::Handle,
        current_list_id: &Option<String>,
        download_dir: &str,
        selected_loader: &str,
    ) {
        ui.heading("Minecraft Launcher");
        ui.add_space(10.0);

        // Java installation section
        ui.group(|ui| {
            ui.label(egui::RichText::new("Java Installation").strong());
            ui.add_space(5.0);

            if self.java_installations.is_empty() {
                ui.colored_label(egui::Color32::RED, "⚠ No Java installations found!");
                ui.label("Please install Java 17+ to launch Minecraft.");
            } else {
                egui::ComboBox::from_label("Select Java")
                    .selected_text(
                        self.selected_java_index
                            .and_then(|idx| self.java_installations.get(idx))
                            .map(|j| format!("{} ({})", j.version, j.path.display()))
                            .unwrap_or_else(|| "Select Java...".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for (idx, java) in self.java_installations.iter().enumerate() {
                            let label = format!("{} - {}", java.version, java.path.display());
                            ui.selectable_value(&mut self.selected_java_index, Some(idx), label);
                        }
                    });

                if ui.button("➕").on_hover_text("Download Java").clicked() {
                    self.java_download_window.set_open(true, rt_handle);
                }

                if let Some(idx) = self.selected_java_index {
                    if let Some(java) = self.java_installations.get(idx) {
                        if java.is_valid {
                            ui.colored_label(egui::Color32::GREEN, "✓ Valid Java installation");
                        } else {
                            ui.colored_label(egui::Color32::YELLOW, "⚠ Java validation failed");
                        }
                    }
                }
            }
        });

        ui.add_space(10.0);

        // Minecraft installation section
        ui.group(|ui| {
            ui.label(egui::RichText::new("Minecraft Installation").strong());
            ui.add_space(5.0);

            if let Some(mc) = self.minecraft_installation.clone() {
                ui.colored_label(egui::Color32::GREEN, "✓ Minecraft found");
                ui.label(format!("Location: {}", mc.root_dir.display()));
                ui.label(format!(
                    "Installed versions: {}",
                    mc.available_versions.len()
                ));

                ui.add_space(5.0);

                if !mc.available_versions.is_empty() {
                    egui::ComboBox::from_label("Select Minecraft Version")
                        .selected_text(&self.selected_mc_version)
                        .show_ui(ui, |ui| {
                            for version in &mc.available_versions {
                                ui.selectable_value(
                                    &mut self.selected_mc_version,
                                    version.clone(),
                                    version,
                                );
                            }
                        });
                } else {
                    ui.colored_label(egui::Color32::YELLOW, "⚠ No Minecraft versions installed");
                }

                if ui.button("⬇ Install Minecraft").clicked() {
                    self.minecraft_download_window
                        .set_open(true, Some(mc.root_dir.clone()), rt_handle);
                }

                // Fabric support check + background install (if selected)
                self.maybe_start_fabric_support_check(rt_handle, selected_loader);
                self.maybe_start_fabric_install(rt_handle, &mc, selected_loader);
                if selected_loader == "fabric" {
                    ui.add_space(5.0);
                    if self.fabric_support_checking {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Checking Fabric support...");
                        });
                    } else if let Some(err) = &self.fabric_support_error {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!("Fabric check failed: {}", err),
                        );
                        if ui.button("Retry Fabric Check").clicked() {
                            self.fabric_support_error = None;
                            self.fabric_supported = None;
                            self.fabric_support_for_mc_version = None;
                        }
                    } else if self.fabric_supported == Some(false)
                        && self.fabric_support_for_mc_version.as_deref()
                            == Some(&self.selected_mc_version)
                    {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            "Fabric is not available for this Minecraft version. Launch in vanilla.",
                        );
                    } else if self.fabric_installing {
                        let (progress, status) = {
                            let guard = self.fabric_progress.lock().unwrap();
                            (guard.0, guard.1.clone())
                        };

                        let display = if status.is_empty() {
                            "Installing Fabric loader (latest)...".to_string()
                        } else {
                            status
                        };
                        ui.label(display);
                        ui.add(egui::ProgressBar::new(progress).show_percentage());
                    } else if let Some(err) = &self.fabric_error {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!("Fabric install failed: {}", err),
                        );
                        if ui.button("Retry Fabric Install").clicked() {
                            self.fabric_error = None;
                            self.fabric_version_id = None;
                            self.fabric_for_mc_version = None;
                        }
                    } else if self.fabric_for_mc_version.as_deref()
                        == Some(&self.selected_mc_version)
                    {
                        if let Some(version_id) = &self.fabric_version_id {
                            ui.colored_label(
                                egui::Color32::GREEN,
                                format!("Fabric ready: {}", version_id),
                            );
                        }
                    }
                }
            } else {
                ui.colored_label(egui::Color32::RED, "⚠ Minecraft not found!");
                ui.label("Please install Minecraft to use the launcher.");

                if ui.button("⬇ Install Minecraft").clicked() {
                    let mc_dir = MinecraftDetector::get_or_create_minecraft_dir();
                    self.minecraft_download_window
                        .set_open(true, mc_dir, rt_handle);
                }
            }
        });

        ui.add_space(10.0);

        // Launch settings
        ui.group(|ui| {
            ui.label(egui::RichText::new("Launch Settings").strong());
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Username:");
                ui.text_edit_singleline(&mut self.launcher_username);
            });

            ui.add_space(5.0);

            ui.label(format!("Minimum Memory: {} MB", self.launcher_min_memory));
            ui.add(egui::Slider::new(&mut self.launcher_min_memory, 512..=8192).suffix(" MB"));

            ui.add_space(5.0);

            ui.label(format!("Maximum Memory: {} MB", self.launcher_max_memory));
            ui.add(egui::Slider::new(&mut self.launcher_max_memory, 1024..=16384).suffix(" MB"));

            if self.launcher_min_memory > self.launcher_max_memory {
                ui.colored_label(
                    egui::Color32::RED,
                    "⚠ Min memory cannot be greater than max memory!",
                );
            }
        });

        ui.add_space(10.0);

        // Mod list selection
        ui.group(|ui| {
            ui.label(egui::RichText::new("Mod List").strong());
            ui.add_space(5.0);

            let mod_lists = rt_handle
                .block_on(async { config_manager.load_all_lists().await.unwrap_or_default() });

            if let Some(list_id) = current_list_id {
                if let Some(list) = mod_lists.iter().find(|l| &l.id == list_id) {
                    ui.colored_label(egui::Color32::GREEN, format!("✓ Using list: {}", list.name));
                    ui.label(format!("Mods: {}", list.mods.len()));
                } else {
                    ui.label("No list selected");
                }
            } else {
                ui.label("No list selected - will launch vanilla Minecraft");
            }
        });

        ui.add_space(15.0);

        // Launch button
        let has_valid_mc_version = self
            .minecraft_installation
            .as_ref()
            .map(|mc| mc.available_versions.contains(&self.selected_mc_version))
            .unwrap_or(false);

        let fabric_ready = if selected_loader == "fabric" {
            self.fabric_version_id.is_some()
                && self.fabric_for_mc_version.as_deref() == Some(&self.selected_mc_version)
                && !self.fabric_installing
                && self.fabric_error.is_none()
                && self.fabric_supported != Some(false)
        } else {
            true
        };

        let can_launch = !self.java_installations.is_empty()
            && self.selected_java_index.is_some()
            && self.minecraft_installation.is_some()
            && has_valid_mc_version
            && !self.launcher_username.is_empty()
            && self.launcher_min_memory <= self.launcher_max_memory
            && fabric_ready
            && !self.launch_in_progress;

        ui.horizontal(|ui| {
            let launch_button =
                egui::Button::new(egui::RichText::new("🚀 Launch Minecraft").size(18.0));

            if ui
                .add_sized([200.0, 40.0], launch_button)
                .on_disabled_hover_text("Configure Java and Minecraft to launch")
                .on_hover_text("Launch Minecraft with selected settings")
                .clicked()
                && can_launch
            {
                self.launch_minecraft(
                    config_manager,
                    rt_handle,
                    current_list_id,
                    download_dir,
                    selected_loader,
                );
            }
        });

        // Status message
        if let Some(ref status) = self.launch_status {
            ui.add_space(10.0);
            ui.separator();
            ui.label(status);
        }
    }

    fn maybe_start_fabric_install(
        &mut self,
        rt_handle: &tokio::runtime::Handle,
        mc_install: &MinecraftInstallation,
        selected_loader: &str,
    ) {
        if selected_loader != "fabric" {
            return;
        }

        if self.fabric_support_checking || self.fabric_supported == Some(false) {
            return;
        }

        if self.fabric_error.is_some() {
            return;
        }

        let mc_version = self.selected_mc_version.clone();
        let needs_start = !self.fabric_installing
            && (self.fabric_for_mc_version.as_deref() != Some(&mc_version)
                || self.fabric_version_id.is_none());

        if !needs_start {
            return;
        }

        self.fabric_installing = true;
        self.fabric_error = None;
        self.fabric_version_id = None;
        if let Ok(mut guard) = self.fabric_progress.lock() {
            *guard = (0.0, "Starting Fabric install...".to_string());
        }
        self.fabric_for_mc_version = Some(mc_version.clone());

        let tx = self.panel_sender.clone();
        let mc_root = mc_install.root_dir.clone();
        let progress_state = Arc::clone(&self.fabric_progress);

        rt_handle.spawn(async move {
            let progress_cb = std::sync::Arc::new(move |progress: f32, status: String| {
                if let Ok(mut guard) = progress_state.lock() {
                    *guard = (progress, status);
                }
            });

            let result =
                FabricInstaller::ensure_fabric_profile(&mc_root, &mc_version, Some(progress_cb))
                    .await;
            let msg = match result {
                Ok(version_id) => PanelMessage::FabricReady {
                    mc_version,
                    version_id,
                },
                Err(e) => PanelMessage::FabricFailed {
                    mc_version,
                    error: e.to_string(),
                },
            };

            let _ = tx.send(msg).await;
        });
    }

    fn maybe_start_fabric_support_check(
        &mut self,
        rt_handle: &tokio::runtime::Handle,
        selected_loader: &str,
    ) {
        if selected_loader != "fabric" {
            return;
        }

        let mc_version = self.selected_mc_version.clone();
        let needs_check = !self.fabric_support_checking
            && (self.fabric_support_for_mc_version.as_deref() != Some(&mc_version)
                || self.fabric_supported.is_none());

        if !needs_check {
            return;
        }

        self.fabric_support_checking = true;
        self.fabric_supported = None;
        self.fabric_support_error = None;
        self.fabric_support_for_mc_version = Some(mc_version.clone());

        let tx = self.panel_sender.clone();
        rt_handle.spawn(async move {
            let result = FabricInstaller::is_supported(&mc_version).await;
            let msg = match result {
                Ok(supported) => PanelMessage::FabricSupportResult {
                    mc_version,
                    supported,
                    error: None,
                },
                Err(e) => PanelMessage::FabricSupportResult {
                    mc_version,
                    supported: false,
                    error: Some(e.to_string()),
                },
            };
            let _ = tx.send(msg).await;
        });
    }

    fn launch_minecraft(
        &mut self,
        config_manager: &Arc<ConfigManager>,
        rt_handle: &tokio::runtime::Handle,
        current_list_id: &Option<String>,
        download_dir: &str,
        selected_loader: &str,
    ) {
        if self.launch_in_progress {
            return;
        }

        self.launch_status = Some("Preparing to launch...".to_string());

        let java_idx = match self.selected_java_index {
            Some(idx) => idx,
            None => {
                self.launch_status = Some("❌ No Java selected".to_string());
                return;
            }
        };

        let java = match self.java_installations.get(java_idx) {
            Some(j) => j,
            None => {
                self.launch_status = Some("❌ Invalid Java selection".to_string());
                return;
            }
        };

        let mc_install = match &self.minecraft_installation {
            Some(mc) => mc,
            None => {
                self.launch_status = Some("❌ Minecraft not found".to_string());
                return;
            }
        };

        let launch_version = if selected_loader == "fabric" {
            if self.fabric_supported == Some(false) {
                self.launch_status = Some(
                    "❌ Fabric is not available for this Minecraft version. Launch vanilla instead."
                        .to_string(),
                );
                return;
            }
            match &self.fabric_version_id {
                Some(id)
                    if self.fabric_for_mc_version.as_deref() == Some(&self.selected_mc_version) =>
                {
                    id.clone()
                }
                _ => {
                    self.launch_status =
                        Some("❌ Fabric is not ready yet. Please wait.".to_string());
                    return;
                }
            }
        } else {
            self.selected_mc_version.clone()
        };

        // Build launch config
        let config = LaunchConfig {
            profile: LaunchProfile {
                minecraft_version: launch_version,
                mod_loader: selected_loader.to_string(),
                mod_loader_version: None,
                java_path: java.path.clone(),
                game_directory: mc_install.root_dir.clone(),
                mod_list_id: current_list_id.clone(),
            },
            username: self.launcher_username.clone(),
            max_memory_mb: self.launcher_max_memory,
            min_memory_mb: self.launcher_min_memory,
        };

        // Launch in background so we can await mod copy
        self.launch_in_progress = true;
        let tx = self.panel_sender.clone();
        let list_id = current_list_id.clone();
        let download_dir = download_dir.to_string();
        let mods_dir = mc_install.mods_dir.clone();
        let config_manager = Arc::clone(config_manager);

        rt_handle.spawn(async move {
            if let Some(list_id) = list_id {
                let mod_lists = config_manager.load_all_lists().await.unwrap_or_default();
                if let Some(list) = mod_lists.iter().find(|l| l.id == list_id) {
                    let mod_names: Vec<String> =
                        list.mods.iter().map(|m| m.mod_name.clone()).collect();
                    let source_dir = std::path::PathBuf::from(download_dir);

                    let status_msg = format!("Copying {} mods...", mod_names.len());
                    let _ = tx.send(PanelMessage::Status(status_msg)).await;

                    // Clean old mods first
                    let _ = ModCopier::clear_mods_directory(&mods_dir).await;

                    let _ =
                        ModCopier::copy_mods_to_minecraft(&source_dir, &mods_dir, &mod_names).await;
                }
            }

            let _ = tx
                .send(PanelMessage::Status("Launching Minecraft...".to_string()))
                .await;

            let result = match AdvancedLauncher::launch_minecraft(&config) {
                Ok(res) => Ok(res),
                Err(e) => Err(e.to_string()),
            };

            let _ = tx.send(PanelMessage::LaunchFinished(result)).await;
        });
    }
}

impl Default for LauncherPanel {
    fn default() -> Self {
        Self::new()
    }
}
