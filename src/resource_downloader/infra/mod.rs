use std::sync::Arc;
use tokio::sync::Semaphore;

pub(crate) mod adapters;
pub(crate) mod cache;
mod config_manager;
mod game_detection;
mod legacy_list;
mod lists_manager;
mod resource_detector;
mod rm_runtime;
pub(crate) mod xcache;

pub use config_manager::ConfigManager;
pub use game_detection::GameDetection;
pub use legacy_list::LegacyListService;
pub use lists_manager::ListFileManager;
pub use resource_detector::ResourceDetector;
pub use rm_runtime::RMRuntime;

#[derive(Clone)]
pub struct ConnectionLimiter {
    semaphore: Arc<Semaphore>,
}

impl ConnectionLimiter {
    pub fn new(max_connections: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }

    pub async fn acquire(&self, slots: u32) -> tokio::sync::OwnedSemaphorePermit {
        self.semaphore
            .clone()
            .acquire_many_owned(slots)
            .await
            .expect("Semaphore closed")
    }
}
