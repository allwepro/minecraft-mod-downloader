mod app_state;
mod app_ui;

pub use app_state::AppState;
pub use app_ui::App;
use serde::{Deserialize, Serialize};

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
