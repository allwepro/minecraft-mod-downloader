use crate::launcher::infra::{MinecraftDownloadService, MinecraftVersionInfo};
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct MinecraftDownloadWindow {
    open: bool,
    available_versions: Vec<MinecraftVersionInfo>,
    loading_versions: bool,
    downloading: bool,
    progress: Arc<Mutex<(f32, String)>>,
    error_msg: Option<String>,
    sender: mpsc::Sender<DownloadMessage>,
    receiver: mpsc::Receiver<DownloadMessage>,
    minecraft_dir: Option<PathBuf>,
    filter_text: String,
    only_releases: bool,
    visible_count: usize,
}

enum DownloadMessage {
    VersionsLoaded(Result<Vec<MinecraftVersionInfo>, String>),
    DownloadComplete(Result<(), String>),
}

const PAGE_SIZE: usize = 40;

impl MinecraftDownloadWindow {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);
        Self {
            open: false,
            available_versions: Vec::new(),
            loading_versions: false,
            downloading: false,
            progress: Arc::new(Mutex::new((0.0, String::new()))),
            error_msg: None,
            sender: tx,
            receiver: rx,
            minecraft_dir: None,
            filter_text: String::new(),
            only_releases: true,
            visible_count: PAGE_SIZE,
        }
    }

    pub fn set_open(
        &mut self,
        open: bool,
        minecraft_dir: Option<PathBuf>,
        rt_handle: &tokio::runtime::Handle,
    ) {
        self.open = open;
        self.minecraft_dir = minecraft_dir;
        if open && self.available_versions.is_empty() && !self.loading_versions {
            self.fetch_versions(rt_handle);
        }
        if open {
            self.visible_count = PAGE_SIZE;
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn fetch_versions(&mut self, rt_handle: &tokio::runtime::Handle) {
        let mc_dir = match &self.minecraft_dir {
            Some(dir) => dir.clone(),
            None => {
                self.error_msg = Some("Minecraft directory not found".to_string());
                return;
            }
        };

        self.loading_versions = true;
        self.error_msg = None;
        let tx = self.sender.clone();

        rt_handle.spawn(async move {
            let result = MinecraftDownloadService::fetch_available_versions(&mc_dir).await;
            let msg = match result {
                Ok(versions) => DownloadMessage::VersionsLoaded(Ok(versions)),
                Err(e) => DownloadMessage::VersionsLoaded(Err(e.to_string())),
            };
            let _ = tx.send(msg).await;
        });
    }

    fn start_download(
        &mut self,
        version: MinecraftVersionInfo,
        rt_handle: &tokio::runtime::Handle,
    ) {
        let mc_dir = match &self.minecraft_dir {
            Some(dir) => dir.clone(),
            None => {
                self.error_msg = Some("Minecraft directory not found".to_string());
                return;
            }
        };

        self.downloading = true;
        self.progress = Arc::new(Mutex::new((0.0, "Starting download...".to_string())));
        let progress = self.progress.clone();
        let tx = self.sender.clone();

        rt_handle.spawn(async move {
            let progress_cb = std::sync::Arc::new(move |p: f32, status: String| {
                if let Ok(mut guard) = progress.lock() {
                    *guard = (p, status);
                }
            });

            let result =
                MinecraftDownloadService::install_version(&mc_dir, &version.id, Some(progress_cb))
                    .await;

            let msg = match result {
                Ok(_) => DownloadMessage::DownloadComplete(Ok(())),
                Err(e) => DownloadMessage::DownloadComplete(Err(e.to_string())),
            };
            let _ = tx.send(msg).await;
        });
    }

    pub fn show(&mut self, ctx: &egui::Context, rt_handle: &tokio::runtime::Handle) -> bool {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                DownloadMessage::VersionsLoaded(res) => {
                    self.loading_versions = false;
                    match res {
                        Ok(v) => self.available_versions = v,
                        Err(e) => self.error_msg = Some(e),
                    }
                }
                DownloadMessage::DownloadComplete(res) => {
                    self.downloading = false;
                    match res {
                        Ok(_) => {
                            self.open = false;
                            return true;
                        }
                        Err(e) => self.error_msg = Some(e),
                    }
                }
            }
        }

        let mut open = self.open;

        egui::Window::new("Install Minecraft Version")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([520.0, 420.0])
            .show(ctx, |ui| {
                if self.downloading {
                    let (progress, status) = {
                        let guard = self.progress.lock().unwrap();
                        (guard.0, guard.1.clone())
                    };

                    ui.label(status);
                    ui.add(egui::ProgressBar::new(progress).show_percentage());
                    return;
                }

                if self.loading_versions {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Fetching available versions...");
                    });
                    return;
                }

                if let Some(err) = &self.error_msg {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                    if ui.button("Retry").clicked() {
                        self.fetch_versions(rt_handle);
                    }
                }

                if self.available_versions.is_empty()
                    && !self.loading_versions
                    && self.error_msg.is_none()
                {
                    ui.label("No versions found.");
                    return;
                }

                let mut filter_changed = false;
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    let response = ui.text_edit_singleline(&mut self.filter_text);
                    if response.changed() {
                        filter_changed = true;
                    }
                    if ui.checkbox(&mut self.only_releases, "Release only").changed() {
                        filter_changed = true;
                    }
                });

                if filter_changed {
                    self.visible_count = PAGE_SIZE;
                }

                let filter_lower = self.filter_text.to_lowercase();
                let filtered: Vec<&MinecraftVersionInfo> = self
                    .available_versions
                    .iter()
                    .filter(|v| {
                        if self.only_releases && v.version_type != "release" {
                            return false;
                        }
                        if filter_lower.is_empty() {
                            return true;
                        }
                        v.id.to_lowercase().contains(&filter_lower)
                    })
                    .collect();

                if filtered.is_empty() {
                    ui.label("No versions match your filter.");
                    return;
                }

                let mut download_action: Option<MinecraftVersionInfo> = None;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for version in filtered.iter().take(self.visible_count) {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&version.id)
                                        .strong()
                                        .size(15.0),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Install").clicked() {
                                            download_action = Some((*version).clone());
                                        }
                                        ui.label(&version.version_type);
                                    },
                                );
                            });
                            ui.label(egui::RichText::new(&version.release_time).small().weak());
                        });
                        ui.add_space(4.0);
                    }
                });

                if self.visible_count < filtered.len() {
                    ui.add_space(6.0);
                    if ui.button("Show more").clicked() {
                        self.visible_count =
                            (self.visible_count + PAGE_SIZE).min(filtered.len());
                    }
                }

                if let Some(version) = download_action {
                    self.start_download(version, rt_handle);
                }
            });

        self.open = open;
        false
    }
}
