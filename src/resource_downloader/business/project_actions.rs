use crate::get_list;
use crate::get_project_versions;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{
    ListLnk, MutationResult, Project, ProjectDependencyType, ProjectLnk, RTProjectData,
    RTProjectVersion, ResourceType,
};
use std::collections::HashSet;
use std::path::PathBuf;

pub struct ProjectActions;

impl ProjectActions {
    pub fn delete_projects(state: SharedRDState, list_lnk: ListLnk, projects: Vec<ProjectLnk>) {
        state.read().list_pool.mutate(&list_lnk, move |list| {
            let mut result = MutationResult::unchanged();
            for p_lnk in projects {
                result.accumulate(list.remove_project(&p_lnk));
            }
            result
        });
    }

    pub fn archive_projects(
        state: SharedRDState,
        list_lnk: ListLnk,
        projects: Vec<ProjectLnk>,
        archived: bool,
    ) {
        state.read().list_pool.mutate(&list_lnk, move |list| {
            let mut result = MutationResult::unchanged();
            for p_lnk in projects {
                result.accumulate(list.archive_project(&p_lnk, archived));
            }
            result
        });
    }

    pub fn add_project(
        state: SharedRDState,
        list_lnk: ListLnk,
        project_lnk: ProjectLnk,
        resource_type: ResourceType,
        data: RTProjectData,
    ) {
        state.write().pending_scroll = Some((list_lnk.clone(), project_lnk.clone()));
        state.read().list_pool.mutate(&list_lnk, move |list| {
            list.add_project(Project::new_from_rt_project(
                project_lnk,
                resource_type,
                true,
                data,
            ))
        });
    }

    pub fn download_projects_latest(
        state: SharedRDState,
        list_lnk: ListLnk,
        projects: Vec<ProjectLnk>,
        found_hashes: &HashSet<String>,
    ) {
        let (ver, loader, dir, content_type) = {
            let list_arc = get_list!(state, &list_lnk);
            let list = list_arc.read();
            let rt = list
                .get_resource_types()
                .first()
                .cloned()
                .unwrap_or(ResourceType::Mod);
            let config = list.get_resource_type_config(&rt).unwrap();
            (
                list.get_game_version().clone(),
                config.loader.clone(),
                config.download_dir.clone(),
                rt,
            )
        };

        let mut triggered = HashSet::new();

        for p_lnk in projects {
            let selected_version_hash = {
                let list_arc = get_list!(state, &list_lnk);
                let list = list_arc.read();
                list.get_project(&p_lnk)
                    .and_then(|p| p.get_version())
                    .map(|v| v.artifact_hash.clone())
            };

            let versions = get_project_versions!(
                state,
                p_lnk.clone(),
                content_type,
                ver.clone(),
                loader.clone()
            );

            if let Ok(Some(v_list)) = versions {
                let target_version = if let Some(hash) = selected_version_hash {
                    v_list
                        .iter()
                        .find(|v| v.artifact_hash == hash)
                        .or_else(|| v_list.first())
                } else {
                    v_list.first()
                };

                if let Some(version) = target_version {
                    Self::trigger_download_recursive(
                        state.clone(),
                        &list_lnk,
                        &p_lnk,
                        version,
                        &dir,
                        &content_type,
                        found_hashes,
                        &mut triggered,
                    );
                }
            }
        }
    }

    pub fn download_project_specific(
        state: SharedRDState,
        list_lnk: ListLnk,
        project_lnk: ProjectLnk,
        version: &RTProjectVersion,
        found_hashes: &HashSet<String>,
    ) {
        let (dir, content_type) = {
            let list_arc = get_list!(state, &list_lnk);
            let list = list_arc.read();
            let rt = list
                .get_resource_types()
                .first()
                .cloned()
                .unwrap_or(ResourceType::Mod);
            let config = list.get_resource_type_config(&rt).unwrap();
            (config.download_dir.clone(), rt)
        };

        let mut triggered = HashSet::new();
        Self::trigger_download_recursive(
            state,
            &list_lnk,
            &project_lnk,
            version,
            &dir,
            &content_type,
            found_hashes,
            &mut triggered,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn trigger_download_recursive(
        state: SharedRDState,
        lnk: &ListLnk,
        p_lnk: &ProjectLnk,
        version: &RTProjectVersion,
        dir: &String,
        rt: &ResourceType,
        found_hashes: &HashSet<String>,
        triggered: &mut HashSet<ProjectLnk>,
    ) {
        if triggered.contains(p_lnk) {
            return;
        }
        triggered.insert(p_lnk.clone());

        let is_downloaded = found_hashes.contains(&version.artifact_hash);

        if !is_downloaded {
            let safe_name_opt = {
                let list_arc = get_list!(state, lnk);
                let list = list_arc.read();
                list.get_project(p_lnk).map(|p| p.get_safe_filename())
            };

            if let Some(safe_name) = safe_name_opt {
                let dest = PathBuf::from(dir).join(safe_name);
                state.write().download_artifact(
                    &state,
                    p_lnk.clone(),
                    *rt,
                    version.version_id.clone(),
                    version.artifact_id.clone(),
                    dest,
                );
            }
        }

        for dep in &version.depended_on {
            if dep.dependency_type == ProjectDependencyType::Required {
                Self::trigger_list_project_download(
                    state.clone(),
                    lnk,
                    &dep.project,
                    dir,
                    found_hashes,
                    triggered,
                );
            }
        }
    }

    fn trigger_list_project_download(
        state: SharedRDState,
        lnk: &ListLnk,
        p_lnk: &ProjectLnk,
        dir: &String,
        found_hashes: &HashSet<String>,
        triggered: &mut HashSet<ProjectLnk>,
    ) {
        if triggered.contains(p_lnk) {
            return;
        }

        let download_info = {
            let list_arc = get_list!(state, lnk);
            let list = list_arc.read();
            list.get_project(p_lnk).and_then(|p| {
                p.get_version().map(|v| {
                    (
                        p.resource_type,
                        v.version_id.clone(),
                        v.artifact_id.clone(),
                        v.artifact_hash.clone(),
                        v.get_depended_ons().to_vec(),
                        p.get_safe_filename(),
                    )
                })
            })
        };

        if let Some((rt, v_id, a_id, a_hash, deps, safe_name)) = download_info {
            triggered.insert(p_lnk.clone());

            if !found_hashes.contains(&a_hash) {
                let dest = PathBuf::from(dir).join(safe_name);
                state
                    .write()
                    .download_artifact(&state, p_lnk.clone(), rt, v_id, a_id, dest);
            }

            for dep in deps {
                if dep.dependency_type == ProjectDependencyType::Required {
                    Self::trigger_list_project_download(
                        state.clone(),
                        lnk,
                        &dep.project,
                        dir,
                        found_hashes,
                        triggered,
                    );
                }
            }
        }
    }

    pub fn update_all_projects(state: SharedRDState, list_lnk: ListLnk) {
        let projects: Vec<ProjectLnk> = {
            let list_arc = get_list!(state, &list_lnk);
            let list = list_arc.read();
            list.get_target_projects()
                .iter()
                .filter(|p| !p.is_archived())
                .map(|p| p.get_lnk())
                .collect()
        };

        Self::update_selected_projects(state, list_lnk, projects);
    }

    pub fn update_selected_projects(
        state: SharedRDState,
        list_lnk: ListLnk,
        projects: Vec<ProjectLnk>,
    ) {
        let (ver, loader, rt) = {
            let list_arc = get_list!(state, &list_lnk);
            let list = list_arc.read();
            let rt = list
                .get_resource_types()
                .first()
                .cloned()
                .unwrap_or(ResourceType::Mod);
            (
                list.get_game_version().clone(),
                list.get_resource_type_config(&rt).unwrap().loader.clone(),
                rt,
            )
        };

        for p_lnk in projects {
            let versions =
                get_project_versions!(state, p_lnk.clone(), rt, ver.clone(), loader.clone());

            if let Ok(Some(v_list)) = versions
                && let Some(latest) = v_list.first()
            {
                let current_hash = {
                    let list_arc = get_list!(state, &list_lnk);
                    let list = list_arc.read();
                    list.get_project(&p_lnk)
                        .and_then(|p| p.get_version())
                        .map(|v| v.artifact_hash.clone())
                };

                if Some(latest.artifact_hash.clone()) != current_hash {
                    state.read().list_pool.select_version(
                        &list_lnk,
                        p_lnk,
                        latest.version_id.clone(),
                    );
                }
            }
        }
    }
}
