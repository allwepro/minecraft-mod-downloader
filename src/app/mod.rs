mod app_state;
mod effect;
mod runtime;

use crate::domain::{ModLoader, ProjectType};
pub use app_state::AppState;
pub use effect::Effect;
pub use runtime::AppRuntime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Default)]
pub enum ListAction {
    #[default]
    Import,
    Duplicate,
}

#[derive(Clone, Debug)]
pub struct PendingCollection {
    pub name: String,
    pub project_type_suggestions: HashMap<ProjectType, (String, ModLoader)>, // (recommended_version, recommended_loader)
    pub projects: Vec<(String, String, ProjectType)>, // (project_id, project_name, project_type)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Name,
    DateAdded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FilterMode {
    #[default]
    All,
    CompatibleOnly,
    IncompatibleOnly,
    MissingOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OrderMode {
    #[default]
    Ascending,
    Descending,
}
