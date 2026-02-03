use crate::launcher::infra::JavaDownloadService;
use crate::launcher::infra::java_downloader::JavaVersion;
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct JavaDownloadWindow {
    open: bool,
    available_versions: Vec<JavaVersion>,
    loading_versions: bool,
    downloading: bool,
    progress: Arc<Mutex<(f32, String)>>,
    error_msg: Option<String>,
    sender: mpsc::Sender<DownloadMessage>,
    receiver: mpsc::Receiver<DownloadMessage>,
}

enum DownloadMessage {
    VersionsLoaded(Result<Vec<JavaVersion>, String>),
    DownloadComplete(Result<PathBuf, String>),
}

impl JavaDownloadWindow {
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
        }
    }

    pub fn set_open(&mut self, open: bool, rt_handle: &tokio::runtime::Handle) {
        self.open = open;
        if open && self.available_versions.is_empty() && !self.loading_versions {
            self.fetch_versions(rt_handle);
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn fetch_versions(&mut self, rt_handle: &tokio::runtime::Handle) {
        self.loading_versions = true;
        self.error_msg = None;
        let tx = self.sender.clone();

        rt_handle.spawn(async move {
            match JavaDownloadService::fetch_available_versions().await {
                Ok(versions) => {
                    let _ = tx.send(DownloadMessage::VersionsLoaded(Ok(versions))).await;
                }
                Err(e) => {
                    let _ = tx
                        .send(DownloadMessage::VersionsLoaded(Err(e.to_string())))
                        .await;
                }
            }
        });
    }

    fn start_download(&mut self, version: JavaVersion, rt_handle: &tokio::runtime::Handle) {
        self.downloading = true;
        self.progress = Arc::new(Mutex::new((0.0, "Starting...".to_string())));
        let progress = self.progress.clone();
        let tx = self.sender.clone();

        // Target directory: <minecraft_dir>/java_runtimes
        // We'll use absolute path relative to CWD to ensure it matches detector
        let target_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("java_runtimes");

        rt_handle.spawn(async move {
            let result =
                JavaDownloadService::download_and_extract(&version, target_dir, move |p, msg| {
                    if let Ok(mut guard) = progress.lock() {
                        *guard = (p, msg.to_string());
                    }
                })
                .await;

            let msg = match result {
                Ok(path) => DownloadMessage::DownloadComplete(Ok(path)),
                Err(e) => DownloadMessage::DownloadComplete(Err(e.to_string())),
            };
            let _ = tx.send(msg).await;
        });
    }

    pub fn show(&mut self, ctx: &egui::Context, rt_handle: &tokio::runtime::Handle) -> bool {
        // Handle async messages
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
                            self.open = false; // Close on success
                            return true; // Signal reload
                        }
                        Err(e) => self.error_msg = Some(e),
                    }
                }
            }
        }

        let mut reload_needed = false;
        let mut open = self.open;

        egui::Window::new("Download Java")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if self.downloading {
                    let (progress, status) = {
                        let guard = self.progress.lock().unwrap();
                        (guard.0, guard.1.clone())
                    };

                    ui.label(format!("Installing Java... {}", status));
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

                ui.label("Select a Java version to download:");
                ui.add_space(5.0);

                if self.available_versions.is_empty()
                    && !self.loading_versions
                    && self.error_msg.is_none()
                {
                    ui.label("No versions found.");
                    return;
                }

                let mut download_action = None;

                for version in &self.available_versions {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Java {}", version.version))
                                    .strong()
                                    .size(16.0),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Download").clicked() {
                                        download_action = Some(version.clone());
                                    }
                                    ui.label(format!(
                                        "{:.1} MB",
                                        version.size as f64 / 1024.0 / 1024.0
                                    ));
                                },
                            );
                        });
                        ui.label(egui::RichText::new(&version.release_name).small().weak());
                    });
                    ui.add_space(5.0);
                }

                if let Some(version) = download_action {
                    self.start_download(version, rt_handle);
                }
            });

        self.open = open;
        reload_needed
    }
}
