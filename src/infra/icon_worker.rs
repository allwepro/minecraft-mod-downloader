use crate::infra::ApiService;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

pub struct IconWorker {
    api_service: Arc<ApiService>,
    cache_dir: PathBuf,
    url_rx: mpsc::Receiver<String>,
    icon_tx: mpsc::Sender<(String, Vec<u8>)>,
    in_flight: HashSet<String>,
}

impl IconWorker {
    pub fn new(
        api_service: Arc<ApiService>,
        cache_dir: PathBuf,
        url_rx: mpsc::Receiver<String>,
        icon_tx: mpsc::Sender<(String, Vec<u8>)>,
    ) -> Self {
        Self {
            api_service,
            cache_dir,
            url_rx,
            icon_tx,
            in_flight: HashSet::new(),
        }
    }

    pub async fn run(mut self) {
        while let Some(url) = self.url_rx.recv().await {
            if url.is_empty() {
                continue;
            }
            if self.in_flight.contains(&url) {
                continue;
            }
            self.in_flight.insert(url.clone());

            let api_service = self.api_service.clone();
            let cache_dir = self.cache_dir.clone();
            let icon_tx = self.icon_tx.clone();

            tokio::spawn(async move {
                let bytes = fetch_icon_bytes(&api_service, &cache_dir, &url).await;
                if let Some(data) = bytes {
                    let _ = icon_tx.send((url, data)).await;
                }
            });
        }
    }
}

async fn fetch_icon_bytes(
    api_service: &ApiService,
    cache_dir: &PathBuf,
    url: &str,
) -> Option<Vec<u8>> {
    let icon_path = cache_path_for_url(cache_dir, url);

    if icon_path.exists() {
        if let Ok(metadata) = tokio::fs::metadata(&icon_path).await {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    const THIRTY_DAYS: Duration = Duration::from_secs(30 * 24 * 60 * 60);

                    if elapsed > THIRTY_DAYS {
                        let _ = tokio::fs::remove_file(&icon_path).await;
                    } else {
                        return tokio::fs::read(&icon_path).await.ok();
                    }
                }
            }
        }
    }

    let _permit = api_service.limiter.acquire(1).await;

    let resp = reqwest::get(url).await.ok()?;
    let data = resp.bytes().await.ok()?;

    if let Some(parent) = icon_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let _ = tokio::fs::write(&icon_path, &data).await;

    Some(data.to_vec())
}

fn cache_path_for_url(cache_dir: &PathBuf, url: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = format!("{:x}", hasher.finish());
    cache_dir.join("icons").join(hash)
}
