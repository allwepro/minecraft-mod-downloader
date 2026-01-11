mod app_state;
mod app_ui;

pub use app_state::AppState;
pub use app_ui::App;
use serde::{Deserialize, Serialize};

#[derive(PartialEq)]
pub enum ListAction {
    Import,
    Duplicate,
}

#[derive(PartialEq)]
pub enum LegacyState {
    Idle,
    InProgress {
        current: usize,
        total: usize,
        message: String,
    },
    Complete {
        suggested_name: String,
        successful: Vec<String>,
        failed: Vec<String>,
        warnings: Vec<String>,
        is_import: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Idle,
    Queued,
    Downloading,
    Complete,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortMode {
    Name,
    DateAdded,
    Compatibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    All,
    CompatibleOnly,
    IncompatibleOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderMode {
    Ascending,
    Descending,
}
