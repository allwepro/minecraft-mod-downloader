mod app_state;
mod effect;
mod runtime;

pub use app_state::AppState;
pub use effect::Effect;
pub use runtime::AppRuntime;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Default)]
pub enum ListAction {
    #[default]
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
