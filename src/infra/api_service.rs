use crate::adapters::ModrinthProvider;
use crate::domain::{ConnectionLimiter, ModProvider};
use std::sync::Arc;

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
