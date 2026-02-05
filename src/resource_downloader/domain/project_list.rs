use crate::resource_downloader::domain::project_operations::{MutationOutcome, MutationResult};
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, Project, ProjectLnk, ProjectVersion, ResourceType,
};
use crate::resource_downloader::infra::cache::time_now;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListMetadata {
    pub(crate) name: String,
    pub(crate) game_version: GameVersion,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectTypeConfig {
    pub loader: GameLoader,
    pub download_dir: String,
}

#[allow(dead_code)]
impl ProjectTypeConfig {
    pub fn new(loader: GameLoader, download_dir: String) -> Self {
        Self {
            loader,
            download_dir,
        }
    }

    pub fn get_loader(&self) -> &GameLoader {
        &self.loader
    }

    pub fn get_download_dir(&self) -> &str {
        &self.download_dir
    }

    pub fn set_download_dir(&mut self, download_dir: String) {
        self.download_dir = download_dir;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListConfig {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    do_updates: Option<bool>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectList {
    list_id: String,
    pub(crate) metadata: ListMetadata,
    pub(crate) type_config: HashMap<ResourceType, ProjectTypeConfig>,
    #[serde(default)]
    pub(crate) projects: Vec<Project>,
    pub(crate) config: ListConfig,
}

#[allow(dead_code)]
impl ProjectList {
    pub fn generate_id() -> String {
        format!("list_{}", time_now())
    }

    pub fn is_lnk(&self, list_lnk: &ListLnk) -> bool {
        list_lnk.is_for(self)
    }
    pub fn get_lnk(&self) -> ListLnk {
        ListLnk::from(self)
    }
    pub fn new(list_id: String, name: String, game_version: GameVersion) -> Self {
        Self {
            list_id,
            metadata: ListMetadata { name, game_version },
            type_config: HashMap::new(),
            projects: Vec::new(),
            config: ListConfig {
                version: 1,
                do_updates: None,
                created_at: Utc::now(),
            },
        }
    }

    pub fn new_from_existing(list_file: &ProjectList, list_id: String) -> Self {
        let mut type_config: HashMap<ResourceType, ProjectTypeConfig> = HashMap::new();
        for tc in list_file.type_config.iter() {
            type_config.insert(
                *tc.0,
                ProjectTypeConfig::new(tc.1.loader.clone(), tc.1.download_dir.clone()),
            );
        }
        let mut project_list = Vec::new();
        for pr in list_file.projects.iter() {
            project_list.push(Project::new_from_existing(pr));
        }

        Self {
            list_id,
            metadata: ListMetadata {
                name: list_file.metadata.name.clone(),
                game_version: list_file.metadata.game_version.clone(),
            },
            type_config,
            projects: project_list,
            config: ListConfig {
                version: list_file.config.version,
                do_updates: list_file.config.do_updates,
                created_at: Utc::now(),
            },
        }
    }

    // -------------  LIST -------------

    pub fn get_id(&self) -> String {
        self.list_id.clone()
    }

    // -------------  METADATA -------------

    pub fn get_name(&self) -> String {
        self.metadata.name.clone()
    }

    pub fn set_list_name(&mut self, name: String) {
        self.metadata.name = name;
    }

    pub fn get_game_version(&self) -> GameVersion {
        self.metadata.game_version.clone()
    }

    pub fn set_game_version(&mut self, version: GameVersion) {
        self.metadata.game_version = version;
    }

    // -------------  TYPE CONFIG -------------
    pub fn get_resource_types(&self) -> Vec<ResourceType> {
        let mut types: Vec<ResourceType> = self.projects.iter().map(|p| p.resource_type).collect();
        types.sort_by_key(|t| t.id().to_string());
        types.dedup();
        if types.is_empty() {
            types = self.type_config.keys().cloned().collect();
            types.sort_by_key(|t| t.id().to_string());
        }
        types
    }

    pub fn has_resource_types(&self) -> bool {
        !self.type_config.is_empty()
    }

    pub fn get_resource_type(&self, resource_type: &ResourceType) -> Option<&ProjectTypeConfig> {
        self.type_config.get(resource_type)
    }

    pub fn set_resource_type(
        &mut self,
        resource_type: ResourceType,
        type_config: ProjectTypeConfig,
    ) {
        self.type_config.insert(resource_type, type_config);
    }

    pub fn remove_resource_type(
        &mut self,
        resource_type: &ResourceType,
    ) -> Option<ProjectTypeConfig> {
        if self
            .projects
            .iter()
            .any(|p| &p.resource_type == resource_type)
        {
            panic!("Cannot remove project type that is still in use");
        }
        self.type_config.remove(resource_type)
    }

    // -------------  CONFIG -------------

    pub fn get_config_version(&self) -> u32 {
        self.config.version
    }

    pub fn get_do_updates(&self) -> bool {
        self.config.do_updates.unwrap_or(false)
    }

    pub fn set_do_updates(&mut self, do_updates: Option<bool>) {
        self.config.do_updates = do_updates;
    }

    pub fn get_config_created_at(&self) -> DateTime<Utc> {
        self.config.created_at
    }

    // -------------  PROJECTS -------------

    // projects reader

    pub fn get_target_projects(&self) -> &[Project] {
        &self.projects
    }

    fn get_projects(&self) -> Vec<ProjectLnk> {
        self.projects
            .iter()
            .map(|p| p.get_lnk().clone())
            .collect::<Vec<_>>()
    }

    pub fn get_manual_projects(&self) -> Vec<ProjectLnk> {
        self.projects
            .iter()
            .filter(|p| p.is_manual())
            .map(|p| p.get_lnk().clone())
            .collect::<Vec<_>>()
    }

    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }

    pub fn project_count(&self) -> usize {
        self.projects.len()
    }

    fn projects_by_type(&self, resource_type: ResourceType) -> Vec<&Project> {
        self.projects
            .iter()
            .filter(|p| p.resource_type == resource_type)
            .collect()
    }

    pub fn manual_projects_by_type(&self, resource_type: ResourceType) -> Vec<&Project> {
        self.projects
            .iter()
            .filter(|p| p.is_manual() && p.resource_type == resource_type)
            .collect()
    }

    pub fn count_projects_by_type(&self, resource_type: ResourceType) -> usize {
        self.projects
            .iter()
            .filter(|p| p.resource_type == resource_type)
            .count()
    }

    pub fn find_projects_by_name(&self, query: &str) -> Vec<&Project> {
        let query_lower = query.to_lowercase();
        self.projects
            .iter()
            .filter(|p| p.get_name().to_lowercase().contains(&query_lower))
            .collect()
    }

    pub fn has_project(&self, project: &ProjectLnk) -> bool {
        self.projects.iter().any(|p| p.is_lnk(project))
    }

    pub fn get_project(&self, project: &ProjectLnk) -> Option<&Project> {
        self.projects.iter().find(|p| p.is_lnk(project))
    }

    pub fn get_project_mut(&mut self, project: &ProjectLnk) -> Option<&mut Project> {
        self.projects.iter_mut().find(|p| p.is_lnk(project))
    }

    pub fn is_project_archived(&self, project: &ProjectLnk) -> bool {
        self.get_project(project).unwrap().is_archived()
            && self
                .get_project(project)
                .unwrap()
                .get_dependents()
                .iter()
                .find(|d| self.get_project(d).is_some_and(|p| !p.is_archived()))
                .is_none()
    }

    // public operations
    pub fn add_project(&mut self, project_target: Project) -> MutationResult {
        if self.has_project(&project_target.get_lnk()) {
            if project_target.is_manual()
                && self
                    .get_project(&project_target.get_lnk())
                    .is_some_and(|a| !a.is_manual())
            {
                self.get_project_mut(&project_target.get_lnk())
                    .unwrap()
                    .set_manual(true);
                MutationResult::new(MutationOutcome::ProjectPromoted)
            } else {
                MutationResult::new(MutationOutcome::AlreadyExists)
            }
        } else {
            let project_lnk = project_target.get_lnk();
            let mut mutation =
                MutationResult::new(MutationOutcome::ProjectAdded).with_target(project_lnk.clone());
            mutation.add_added(vec![project_lnk]);

            if project_target.has_version() {
                panic!(
                    "Cannot add project with a version - the project needs to be added without a version (all dependencies have to be added too) and then the version can be added and they can be declared as dependency!"
                );
            }

            self.projects.push(project_target);

            mutation
        }
    }

    pub fn remove_project(&mut self, project: &ProjectLnk) -> MutationResult {
        if !self.has_project(project) {
            return MutationResult::not_found();
        }

        let target_project = self.get_project_mut(project).unwrap();

        if target_project.has_dependents() {
            target_project.set_manual(false);
            return MutationResult::new(MutationOutcome::ProjectDemoted)
                .with_target(target_project.get_lnk());
        }

        let mut mutation =
            MutationResult::new(MutationOutcome::ProjectRemoved).with_target(project.clone());

        mutation.chain(self.clear_dependencies_internal(project));

        if let Some(pos) = self.get_project_internal_id(project) {
            mutation.add_removed(vec![self.projects.remove(pos)]);
        }
        mutation.chain(self.cleanup_orphaned_dependencies());
        mutation
    }

    pub fn add_version(&mut self, project: &ProjectLnk, version: ProjectVersion) -> MutationResult {
        if !self.has_project(project) {
            return MutationResult::not_found();
        }

        let mut mutation =
            MutationResult::new(MutationOutcome::VersionAdded).with_target(project.clone());

        for dep in version.depended_on.as_slice() {
            let depended_on_project_target =
                self.get_project_mut(&dep.project).unwrap_or_else(|| {
                    panic!("A dependency was added to a project that does not exist!");
                });

            depended_on_project_target.add_dependent(project.clone());
        }

        mutation.add_changed(
            version
                .depended_on
                .iter()
                .map(|d| d.project.clone())
                .collect(),
        );

        let target_project = self.get_project_mut(project);
        target_project.unwrap().set_version(version);

        mutation.add_changed(vec![project.clone()]);
        mutation
    }

    pub fn remove_version(&mut self, project: &ProjectLnk) -> MutationResult {
        if !self.has_project(project) {
            return MutationResult::not_found();
        }

        let mut mutation =
            MutationResult::new(MutationOutcome::VersionRemoved).with_target(project.clone());

        mutation.chain(self.clear_dependencies_internal(project));

        let target_project = self.get_project_mut(project).unwrap();

        if target_project.clear_project_version().is_none() {
            return MutationResult::unchanged();
        }

        mutation.chain(self.cleanup_orphaned_dependencies());
        mutation
    }

    pub fn set_manual_dependency_type(
        &mut self,
        project: &ProjectLnk,
        depended_on_project: &ProjectLnk,
        new_manual_dependency_type: Option<bool>,
    ) -> MutationResult {
        if !self.has_project(project) || !self.has_project(depended_on_project) {
            return MutationResult::not_found();
        }

        let target_project = self.get_project_mut(project).unwrap();

        if !target_project.has_version() {
            return MutationResult::not_found();
        }

        let target_project_version = target_project.get_version_mut().unwrap();

        if !target_project_version.has_depended_on(depended_on_project) {
            return MutationResult::not_found();
        }

        let target_project_dependency = target_project_version
            .get_depended_on_mut(depended_on_project.clone())
            .unwrap();

        let (old_manual_dependency_type, old_should_manage, new_should_manage) = (
            target_project_dependency.manual_dependency_type,
            target_project_dependency
                .dependency_type
                .needs_managing(target_project_dependency.manual_dependency_type),
            target_project_dependency
                .dependency_type
                .needs_managing(new_manual_dependency_type),
        );
        let management_changed = old_should_manage != new_should_manage;

        let mut mutation = MutationResult::new(MutationOutcome::OverrideUpdated {
            old: target_project_dependency
                .dependency_type
                .get_effective_type(old_manual_dependency_type),
            new: target_project_dependency
                .dependency_type
                .get_effective_type(new_manual_dependency_type),
            management_changed,
        });

        target_project_dependency.manual_dependency_type = new_manual_dependency_type;
        mutation.add_changed(vec![project.clone()]);

        let depended_on_target_project = self.get_project_mut(depended_on_project).unwrap();
        if old_should_manage && !new_should_manage {
            depended_on_target_project.remove_dependent(project.clone());
            mutation.chain(self.cleanup_orphaned_dependencies());
        } else if !old_should_manage && new_should_manage {
            depended_on_target_project.add_dependent(project.clone());
        }
        mutation.add_changed(vec![depended_on_project.clone()]);

        mutation
    }

    pub fn get_dependents_versions_ids(
        &self,
        project: &ProjectLnk,
        disallowed_mode: bool,
    ) -> Vec<String> {
        let mut versions = Vec::new();

        if let Some(target_project) = self.get_project(project) {
            for dependent_id in target_project.get_dependents() {
                if let Some(dependent_project) = self.get_project(dependent_id)
                    && let Some(dependent_version) = dependent_project.get_version()
                    && let Some(dependent_dependency) =
                        dependent_version.get_depended_on(project.clone())
                    && (!disallowed_mode
                        && dependent_dependency
                            .dependency_type
                            .get_effective_type(dependent_dependency.manual_dependency_type)
                            .is_positive()
                        || disallowed_mode
                            && dependent_dependency
                                .dependency_type
                                .get_effective_type(dependent_dependency.manual_dependency_type)
                                .is_negative())
                    && let Some(version_id) = dependent_dependency.version_id.clone()
                    && !versions.contains(&version_id)
                {
                    versions.push(version_id);
                }
            }
        }

        versions.dedup();
        versions
    }

    // internal helpers
    fn get_project_internal_id(&self, project: &ProjectLnk) -> Option<usize> {
        self.projects.iter().position(|p| p.is_lnk(project))
    }

    fn clear_dependencies_internal(&mut self, project: &ProjectLnk) -> MutationResult {
        let mut mutation =
            MutationResult::new(MutationOutcome::DependenciesCleared).with_target(project.clone());
        mutation.add_changed(vec![project.clone()]);

        let depended_on_projects: Vec<ProjectLnk> = if let Some(target_project) =
            self.get_project_mut(project)
            && let Some(version) = target_project.get_version_mut()
        {
            version.clear_depended_ons()
        } else {
            return MutationResult::not_found();
        };

        for depended_on_project in &depended_on_projects {
            if let Some(dep_project) = self.get_project_mut(depended_on_project) {
                dep_project.remove_dependent(project.clone());
            }
        }

        mutation.add_changed(depended_on_projects);
        mutation
    }

    fn cleanup_orphaned_dependencies(&mut self) -> MutationResult {
        let mut mutation = MutationResult::new(MutationOutcome::OrphansCleared);

        loop {
            let to_remove: Vec<ProjectLnk> = self
                .projects
                .iter()
                .filter(|p| p.is_cleanable_dependency())
                .map(|p| p.get_lnk())
                .collect();

            if to_remove.is_empty() {
                break;
            }

            for project in to_remove {
                mutation.chain(self.clear_dependencies_internal(&project));
                if let Some(pos) = self.get_project_internal_id(&project) {
                    mutation.add_removed(vec![self.projects.remove(pos)]);
                }
            }
        }

        mutation
    }
}
