use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::domain::{
    ListLnk, MutationResult, Project, ProjectDependencyType, ProjectLnk, RTProjectData,
    RTProjectVersion, ResourceType,
};
use crate::{get_list, get_project_versions, get_project_versions_best};
use std::collections::HashSet;
use std::path::PathBuf;

pub struct ProjectActions;

impl ProjectActions {
    pub fn remove_projects(state: SharedRDState, list_lnk: ListLnk, projects: Vec<ProjectLnk>) {
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

    pub fn download_projects(
        state: SharedRDState,
        list_lnk: ListLnk,
        projects: Vec<(ProjectLnk, Option<RTProjectVersion>)>,
        found_hashes: &HashSet<String>,
    ) {
        let list_arc = get_list!(state, &list_lnk);
        let ver = {
            let list = list_arc.read();
            ListActions::get_effective_game_version(&state, &list_lnk, &list.get_game_version())
        };

        let mut triggered = HashSet::new();

        for (p_lnk, specific_version) in projects {
            let (content_type, loader, dir, is_overruled, selected_version_hash) = {
                let list = list_arc.read();
                let p = list.get_project(&p_lnk);
                let rt = p.map(|p| p.get_type()).unwrap_or(ResourceType::Mod);
                if let Some(config) = list.get_resource_type_config(&rt) {
                    let original_dir = config.download_dir.clone();
                    let effective_dir = ListActions::get_effective_download_dir(
                        &state,
                        &list_lnk,
                        rt,
                        &original_dir,
                    );
                    (
                        rt,
                        config.loader.clone(),
                        effective_dir,
                        p.map(|p| p.is_compatibility_overruled()).unwrap_or(false),
                        p.and_then(|p| p.get_version())
                            .map(|v| v.artifact_hash.clone()),
                    )
                } else {
                    continue;
                }
            };

            let version = if let Some(v) = specific_version {
                Some(v)
            } else {
                let versions = if is_overruled {
                    get_project_versions_best!(
                        state,
                        p_lnk.clone(),
                        content_type,
                        ver.clone(),
                        loader.clone()
                    )
                } else {
                    get_project_versions!(
                        state,
                        p_lnk.clone(),
                        content_type,
                        ver.clone(),
                        loader.clone()
                    )
                };

                if let Ok(Some(v_list)) = versions {
                    if let Some(hash) = selected_version_hash {
                        v_list
                            .iter()
                            .find(|v| v.artifact_hash == hash)
                            .cloned()
                            .or_else(|| v_list.first().cloned())
                    } else {
                        v_list.into_iter().next()
                    }
                } else {
                    None
                }
            };

            if let Some(version) = version {
                Self::download_version_with_dependencies(
                    state.clone(),
                    &list_lnk,
                    &p_lnk,
                    &version,
                    &dir,
                    &content_type,
                    found_hashes,
                    &mut triggered,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn download_version_with_dependencies(
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
                Self::download_project_with_dependencies(
                    state.clone(),
                    lnk,
                    &dep.project,
                    found_hashes,
                    triggered,
                );
            }
        }
    }

    fn download_project_with_dependencies(
        state: SharedRDState,
        lnk: &ListLnk,
        p_lnk: &ProjectLnk,
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
                    let rt = p.resource_type;
                    let config = list.get_resource_type_config(&rt).unwrap();
                    let dir = ListActions::get_effective_download_dir(
                        &state,
                        lnk,
                        rt,
                        &config.download_dir,
                    );
                    (
                        rt,
                        v.version_id.clone(),
                        v.artifact_id.clone(),
                        v.artifact_hash.clone(),
                        v.get_depended_ons().to_vec(),
                        p.get_safe_filename(),
                        dir,
                    )
                })
            })
        };

        if let Some((rt, v_id, a_id, a_hash, deps, safe_name, dir)) = download_info {
            triggered.insert(p_lnk.clone());

            if !found_hashes.contains(&a_hash) {
                let dest = PathBuf::from(dir).join(safe_name);
                state
                    .write()
                    .download_artifact(&state, p_lnk.clone(), rt, v_id, a_id, dest);
            }

            for dep in deps {
                if dep.dependency_type == ProjectDependencyType::Required {
                    Self::download_project_with_dependencies(
                        state.clone(),
                        lnk,
                        &dep.project,
                        found_hashes,
                        triggered,
                    );
                }
            }
        }
    }

    pub fn update_version_for_all_projects(state: SharedRDState, list_lnk: ListLnk) {
        let projects: Vec<ProjectLnk> = {
            let list_arc = get_list!(state, &list_lnk);
            let list = list_arc.read();
            list.get_target_projects()
                .iter()
                .filter(|p| !p.is_archived())
                .map(|p| p.get_lnk())
                .collect()
        };

        Self::update_version_for_projects(state, list_lnk, projects);
    }

    pub fn update_version_for_projects(
        state: SharedRDState,
        list_lnk: ListLnk,
        projects: Vec<ProjectLnk>,
    ) {
        let list_arc = get_list!(state, &list_lnk);
        let ver = {
            let list = list_arc.read();
            list.get_game_version().clone()
        };

        for p_lnk in projects {
            let (content_type, loader, is_overruled, current_hash) = {
                let list = list_arc.read();
                let p = list.get_project(&p_lnk);
                let rt = p.map(|p| p.get_type()).unwrap_or(ResourceType::Mod);
                if let Some(config) = list.get_resource_type_config(&rt) {
                    (
                        rt,
                        config.loader.clone(),
                        p.map(|p| p.is_compatibility_overruled()).unwrap_or(false),
                        p.and_then(|p| p.get_version())
                            .map(|v| v.artifact_hash.clone()),
                    )
                } else {
                    continue;
                }
            };

            let versions = if is_overruled {
                get_project_versions_best!(
                    state,
                    p_lnk.clone(),
                    content_type,
                    ver.clone(),
                    loader.clone()
                )
            } else {
                get_project_versions!(
                    state,
                    p_lnk.clone(),
                    content_type,
                    ver.clone(),
                    loader.clone()
                )
            };

            if let Ok(Some(v_list)) = versions {
                if let Some(latest) = v_list.first() {
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
}
