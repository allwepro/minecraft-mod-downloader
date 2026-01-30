use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod artifact_cache_worker;

pub(crate) use artifact_cache_worker::ArtifactWorker;

pub const ARTIFACT_MAX_CONNECTIONS: usize = 2;
pub const ARTIFACT_CACHE_TIME: Duration = duration_from_days(30);

pub(crate) const fn duration_from_minutes(minutes: u64) -> Duration {
    Duration::from_secs(minutes * 60)
}

pub(crate) const fn duration_from_hours(hours: u64) -> Duration {
    duration_from_minutes(hours * 60)
}

pub(crate) const fn duration_from_days(days: u64) -> Duration {
    duration_from_hours(days * 24)
}

pub fn time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}
