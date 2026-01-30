pub(crate) mod cache;
mod effects;
mod events;
mod list_pool;
mod rd_state;
pub(crate) mod services;
pub(crate) mod xcache;

pub use effects::Effect;
pub use events::{Event, InternalEvent};
pub use rd_state::{DownloadStatus, RDState, SharedRDState};

#[macro_export]
macro_rules! get_versions {
    ($state:expr) => {
        $state
            .read()
            .api()
            .game_version_pool
            .get_versions()
            .unwrap()
    };
}

#[macro_export]
macro_rules! get_loaders {
    ($state:expr, $resource_type:expr) => {
        $state
            .read()
            .api()
            .game_loader_pool
            .get_loaders($resource_type)
            .unwrap()
    };
}

#[macro_export]
macro_rules! get_default_dir {
    ($state:expr, $resource_type:expr) => {
        $state.read().default_dirs.get($resource_type).unwrap()
    };
}

#[macro_export]
macro_rules! get_project_metadata {
    ($state:expr, $project:expr, $resource_type:expr) => {
        $state
            .read()
            .api()
            .rt_project_pool
            .get_metadata($project, $resource_type)
    };
}

#[macro_export]
macro_rules! clear_project_metadata {
    ($state:expr, $project:expr, $resource_type:expr) => {
        $state
            .read()
            .api()
            .rt_project_pool
            .clear_metadata($project, $resource_type)
    };
}

#[macro_export]
macro_rules! get_project_icon_texture {
    ($state:expr, $project:expr) => {
        $state.read().api().icon_pool.get_icon($project)
    };
}

#[macro_export]
macro_rules! get_project_link {
    ($state:expr, $project:expr, $resource_type:expr) => {
        $state
            .read()
            .api()
            .get_project_link($project, $resource_type)
    };
}

#[macro_export]
macro_rules! get_project_versions {
    ($state:expr, $project:expr, $resource_type:expr, $opt_game_version:expr, $opt_game_loader:expr) => {
        $state.read().api().rt_project_pool.get_versions(
            $project,
            $resource_type,
            $opt_game_version,
            $opt_game_loader,
        )
    };
}

#[macro_export]
macro_rules! search_projects {
    ($state:expr, $query:expr, $resource_type:expr, $opt_game_version:expr, $opt_game_loader:expr) => {
        $state.read().api().rt_project_pool.search(
            $query.to_string(),
            $resource_type,
            $opt_game_version,
            $opt_game_loader,
        )
    };
}

#[macro_export]
macro_rules! get_list_type {
    ($state:expr, $list_lnk:expr) => {
        $state
            .read()
            .list_pool
            .get($list_lnk)
            .expect("List not found")
            .read()
            .get_resource_types()
            .first()
            .cloned()
            .unwrap_or(ResourceType::Mod)
    };
}

#[macro_export]
macro_rules! get_list {
    ($state:expr, $list_lnk:expr) => {
        $state
            .read()
            .list_pool
            .get($list_lnk)
            .expect("List not found in pool")
    };
}

#[macro_export]
macro_rules! get_list_mut {
    ($state:expr, $list_lnk:expr) => {
        $state
            .read()
            .list_pool
            .get($list_lnk)
            .expect("List not found")
            .write()
    };
}

#[macro_export]
macro_rules! get_project {
    ($state:expr, $list_lnk:expr, $proj_lnk:expr) => {
        parking_lot::RwLockReadGuard::map(
            $state
                .read()
                .list_pool
                .get($list_lnk)
                .expect("List not found")
                .read(),
            |l| l.get_project($proj_lnk).expect("Project not found"),
        )
    };
}

#[macro_export]
macro_rules! get_project_mut {
    ($state:expr, $list_lnk:expr, $proj_lnk:expr) => {
        parking_lot::RwLockWriteGuard::map(
            $state
                .read()
                .list_pool
                .get($list_lnk)
                .expect("List not found")
                .write(),
            |l| l.get_project_mut($proj_lnk).expect("Project not found"),
        )
    };
}
