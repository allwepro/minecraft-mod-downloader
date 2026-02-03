use crate::domain::ModList;
use crate::infra::ConfigManager;
use crate::launcher::ui::JavaDownloadWindow;
use crate::launcher::{
    AdvancedLauncher, JavaDetector, JavaInstallation, LaunchConfig, LaunchProfile, LaunchResult,
    MinecraftDetector, MinecraftInstallation, ModCopier,
};
use eframe::egui;
use std::sync::Arc;

pub struct LauncherPanel {
    java_installations: Vec<JavaInstallation>,
    selected_java_index: Option<usize>,
    minecraft_installation: Option<MinecraftInstallation>,
    launcher_username: String,
    launcher_min_memory: u32,
    launcher_max_memory: u32,
    launch_status: Option<String>,
    selected_mc_version: String,
    java_download_window: JavaDownloadWindow,
}

impl LauncherPanel {
    pub fn new() -> Self {
        let java_installations = JavaDetector::detect_java_installations();
        let minecraft_installation = MinecraftDetector::detect_minecraft();

        let selected_mc_version = minecraft_installation
            .as_ref()
            .and_then(|mc| mc.available_versions.first().cloned())
            .unwrap_or_else(|| "1.20.1".to_string());

        Self {
            java_installations,
            selected_java_index: None,
            minecraft_installation,
            launcher_username: whoami::username(),
            launcher_min_memory: 1024,
            launcher_max_memory: 4096,
            launch_status: None,
            selected_mc_version,
            java_download_window: JavaDownloadWindow::new(),
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

            if let Some(ref mc) = self.minecraft_installation {
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
            } else {
                ui.colored_label(egui::Color32::RED, "⚠ Minecraft not found!");
                ui.label("Please install Minecraft to use the launcher.");
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

        let can_launch = !self.java_installations.is_empty()
            && self.selected_java_index.is_some()
            && self.minecraft_installation.is_some()
            && has_valid_mc_version
            && !self.launcher_username.is_empty()
            && self.launcher_min_memory <= self.launcher_max_memory;

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

    fn launch_minecraft(
        &mut self,
        config_manager: &Arc<ConfigManager>,
        rt_handle: &tokio::runtime::Handle,
        current_list_id: &Option<String>,
        download_dir: &str,
        selected_loader: &str,
    ) {
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

        // Copy mods if a list is selected
        if let Some(list_id) = current_list_id {
            let mod_lists = rt_handle
                .block_on(async { config_manager.load_all_lists().await.unwrap_or_default() });

            if let Some(list) = mod_lists.iter().find(|l| &l.id == list_id) {
                let mod_names: Vec<String> = list.mods.iter().map(|m| m.mod_name.clone()).collect();
                let source_dir = std::path::PathBuf::from(download_dir);
                let mods_dir = mc_install.mods_dir.clone();

                let status_msg = format!("Copying {} mods...", mod_names.len());
                self.launch_status = Some(status_msg);

                rt_handle.spawn(async move {
                    let _ =
                        ModCopier::copy_mods_to_minecraft(&source_dir, &mods_dir, &mod_names).await;
                });
            }
        }

        // Build launch config
        let config = LaunchConfig {
            profile: LaunchProfile {
                minecraft_version: self.selected_mc_version.clone(),
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

        // Launch Minecraft using advanced launcher
        match AdvancedLauncher::launch_minecraft(&config) {
            Ok(LaunchResult::Success { pid }) => {
                self.launch_status = Some(format!(
                    "✅ Minecraft launched successfully! (PID: {})",
                    pid
                ));
            }
            Ok(LaunchResult::Failed { error }) => {
                self.launch_status = Some(format!("❌ Launch failed: {}", error));
            }
            Err(e) => {
                self.launch_status = Some(format!("❌ Error: {}", e));
            }
        }
    }
}

impl Default for LauncherPanel {
    fn default() -> Self {
        Self::new()
    }
}
