use crate::adapters::ModrinthProvider;
use crate::domain::ModProvider;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct ApiService {
    pub provider: Arc<dyn ModProvider>,
    pub limiter: Arc<ConnectionLimiter>,
}

impl ApiService {
    pub fn new() -> Self {
        let provider: Arc<dyn ModProvider> = Arc::new(ModrinthProvider::new());
        let connection_limiter = Arc::new(ConnectionLimiter::new(5));

        Self {
            provider,
            limiter: connection_limiter,
        }
    }
}

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
