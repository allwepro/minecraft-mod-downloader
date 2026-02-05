use crate::resource_downloader::domain::lnk_types::ProjectLnk;
use crate::resource_downloader::domain::{RTProjectData, ResourceType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Dependency to another project.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectDependencyType {
    Required,
    Incompatible,
    Ignored,
}

#[allow(dead_code)]
impl ProjectDependencyType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "required" => Some(Self::Required),
            "incompatible" => Some(Self::Incompatible),
            "embedded" => Some(Self::Ignored),
            "optional" => Some(Self::Ignored),
            _ => None,
        }
    }

    pub fn get_effective_type(&self, overruled_type: Option<bool>) -> ProjectDependencyType {
        match overruled_type {
            Some(true) => Self::Required,
            Some(false) => Self::Incompatible,
            _ => *self,
        }
    }

    pub fn needs_managing(&self, manual_dependency_type: Option<bool>) -> bool {
        !matches!(self, Self::Ignored) || manual_dependency_type.unwrap_or(false)
    }

    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Required)
    }

    pub fn is_negative(&self) -> bool {
        matches!(self, Self::Incompatible)
    }
}

/// Specific project dependency.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ProjectDependency {
    pub(crate) project: ProjectLnk,
    pub(crate) dependency_type: ProjectDependencyType,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// None = No change, True = DependencyType::Required, False = DependencyType::Ignored
    ///     (a detected dependency that is not already DependencyType::Incompatible will never be made DependencyType::Incompatible)
    pub(crate) manual_dependency_type: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) version_id: Option<String>,
}
#[allow(dead_code)]
impl ProjectDependency {
    pub fn new(
        project: ProjectLnk,
        dependency_type: ProjectDependencyType,
        manual_dependency_type: Option<bool>,
        version_id: Option<String>,
    ) -> Self {
        Self {
            project,
            dependency_type,
            manual_dependency_type,
            version_id,
        }
    }
}

/// Specific version of a project.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectVersion {
    pub(crate) is_manual: bool,
    pub(crate) version_id: String,
    pub(crate) artifact_id: String,
    ///sha1
    pub(crate) artifact_hash: String,
    pub(crate) channel: String,
    pub(crate) depended_on: Vec<ProjectDependency>,
}

#[allow(dead_code)]
impl ProjectVersion {
    pub fn new(
        is_manual: bool,
        version_id: String,
        artifact_id: String,
        artifact_hash: String,
        channel: String,
        depended_on: Vec<ProjectDependency>,
    ) -> Self {
        Self {
            version_id,
            is_manual,
            artifact_id,
            artifact_hash,
            channel,
            depended_on,
        }
    }
}

/// Settings of a project.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    archived: bool,
    compat_overruled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<ProjectVersion>,
    pub(crate) dependents: Vec<ProjectLnk>,
}

/// Cached data of a project for offline use and caching.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectCache {
    name: String,
    description: String,
    author: String,
}

/// Project.
#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    project_id: String,
    pub(crate) resource_type: ResourceType,
    is_manual: bool,
    pub(crate) settings: ProjectSettings,
    pub(crate) cache: ProjectCache,
    pub(crate) added_at: DateTime<Utc>,
}

#[allow(dead_code)]
impl Project {
    pub fn new(
        project_id: String,
        resource_type: ResourceType,
        is_manual: bool,
        name: String,
        description: String,
        author: String,
    ) -> Self {
        Self {
            project_id,
            resource_type,
            is_manual,
            settings: ProjectSettings {
                archived: false,
                compat_overruled: false,
                version: None,
                dependents: Vec::new(),
            },
            cache: ProjectCache {
                name,
                description,
                author,
            },
            added_at: Utc::now(),
        }
    }

    pub fn new_from_rt_project(
        project: ProjectLnk,
        resource_type: ResourceType,
        is_manual: bool,
        rt_project_data: RTProjectData,
    ) -> Self {
        let data = rt_project_data;
        Self {
            project_id: project.to_string(),
            resource_type,
            is_manual,
            settings: ProjectSettings {
                archived: false,
                compat_overruled: false,
                version: None,
                dependents: Vec::new(),
            },
            cache: ProjectCache {
                name: data.name.clone(),
                description: data.description.clone(),
                author: data.author.clone(),
            },
            added_at: Utc::now(),
        }
    }

    pub fn new_from_existing(project: &Project) -> Self {
        Self {
            project_id: project.project_id.clone(),
            resource_type: project.resource_type,
            is_manual: project.is_manual,
            settings: ProjectSettings {
                archived: project.settings.archived,
                compat_overruled: project.settings.compat_overruled,
                version: project.get_version().map(|v| {
                    ProjectVersion::new(
                        v.is_manual,
                        v.version_id.clone(),
                        v.artifact_id.clone(),
                        v.artifact_hash.clone(),
                        v.channel.clone(),
                        v.depended_on.clone(),
                    )
                }),
                dependents: project.get_dependents().to_vec(),
            },
            cache: ProjectCache {
                name: project.cache.name.clone(),
                description: project.cache.description.clone(),
                author: project.cache.author.clone(),
            },
            added_at: Utc::now(),
        }
    }

    pub fn is_lnk(&self, project_lnk: &ProjectLnk) -> bool {
        project_lnk.is_for(self)
    }
    pub fn get_lnk(&self) -> ProjectLnk {
        ProjectLnk::from(self)
    }

    // -------------  PROJECT -------------
    pub fn get_id(&self) -> String {
        self.project_id.clone()
    }

    pub fn get_type(&self) -> ResourceType {
        self.resource_type
    }

    pub fn is_manual(&self) -> bool {
        self.is_manual
    }

    pub fn is_cleanable_dependency(&self) -> bool {
        !self.is_manual() && !self.has_dependents()
    }

    pub fn set_manual(&mut self, manual: bool) {
        self.is_manual = manual;
    }

    // ------------- SETTINGS -------------
    pub fn is_archived(&self) -> bool {
        self.settings.archived
    }

    pub fn set_archived(&mut self, archived: bool) {
        self.settings.archived = archived;
    }

    pub fn is_compatibility_overruled(&self) -> bool {
        self.settings.compat_overruled
    }

    pub fn set_compatibility_overruled(&mut self, overruled: bool) {
        self.settings.compat_overruled = overruled;
    }

    pub fn toggle_compatibility_overruled(&mut self) -> bool {
        self.set_compatibility_overruled(!self.is_compatibility_overruled());
        self.is_compatibility_overruled()
    }

    pub fn has_version(&self) -> bool {
        self.get_version().is_some()
    }

    pub fn get_version(&self) -> Option<&ProjectVersion> {
        self.settings.version.as_ref()
    }

    pub(crate) fn get_version_mut(&mut self) -> Option<&mut ProjectVersion> {
        self.settings.version.as_mut()
    }

    pub fn get_version_id(&self) -> Option<&str> {
        self.get_version().map(|v| v.version_id.as_str())
    }

    pub fn get_version_artifact_id(&self) -> Option<&str> {
        self.get_version().as_ref().map(|v| v.artifact_id.as_str())
    }

    pub fn set_version(&mut self, version: ProjectVersion) {
        self.settings.version = Some(version);
    }

    pub fn clear_project_version(&mut self) -> Option<ProjectVersion> {
        self.settings.version.take()
    }

    pub(crate) fn has_dependents(&self) -> bool {
        !self.get_dependents().is_empty()
    }

    pub(crate) fn dependent_count(&self) -> usize {
        self.get_dependents().len()
    }
    pub(crate) fn has_dependent(&self, dependent_project: &ProjectLnk) -> bool {
        self.get_dependents().iter().any(|d| d == dependent_project)
    }

    pub(crate) fn get_dependents(&self) -> &[ProjectLnk] {
        self.settings.dependents.as_slice()
    }

    fn get_dependents_mut(&mut self) -> &mut Vec<ProjectLnk> {
        &mut self.settings.dependents
    }

    fn get_internal_dependent_id(&self, dependent_project: ProjectLnk) -> Option<usize> {
        self.get_dependents()
            .iter()
            .position(|d| d == &dependent_project)
    }

    pub(crate) fn add_dependent(&mut self, dependent_project: ProjectLnk) {
        if !self.has_dependent(&dependent_project) {
            self.get_dependents_mut().push(dependent_project);
        }
    }

    pub(crate) fn remove_dependent(&mut self, dependent_project: ProjectLnk) -> bool {
        if let Some(pos) = self.get_internal_dependent_id(dependent_project) {
            self.get_dependents_mut().remove(pos);
            true
        } else {
            false
        }
    }

    // ------------- CACHE -------------
    pub fn get_name(&self) -> String {
        self.cache.name.clone()
    }

    pub fn set_name(&mut self, name: String) {
        self.cache.name = name;
    }

    pub fn get_description(&self) -> String {
        self.cache.description.clone()
    }

    pub fn set_description(&mut self, description: String) {
        self.cache.description = description;
    }

    pub fn get_author(&self) -> String {
        self.cache.author.clone()
    }

    pub fn set_author(&mut self, author: String) {
        self.cache.author = author;
    }

    pub fn update_cache(&mut self, rt_project_data: RTProjectData) -> bool {
        let mut changed = false;
        if self.cache.name != rt_project_data.name {
            self.cache.name = rt_project_data.name;
            changed = true;
        }
        if self.cache.description != rt_project_data.description {
            self.cache.description = rt_project_data.description;
            changed = true;
        }
        if self.cache.author != rt_project_data.author {
            self.cache.author = rt_project_data.author;
            changed = true;
        }
        changed
    }
}

#[allow(dead_code)]
impl ProjectVersion {
    pub(crate) fn has_depended_ons(&self) -> bool {
        self.depended_on.is_empty()
    }

    pub(crate) fn depended_on_count(&self) -> usize {
        self.depended_on.len()
    }

    pub(crate) fn get_depended_ons(&self) -> &[ProjectDependency] {
        self.depended_on.as_slice()
    }

    fn get_depended_ons_mut(&mut self) -> &mut Vec<ProjectDependency> {
        &mut self.depended_on
    }

    pub fn has_depended_on(&self, depended_on_project: &ProjectLnk) -> bool {
        self.get_depended_ons()
            .iter()
            .any(|d| &d.project == depended_on_project)
    }

    fn get_internal_depended_on_id(&self, depended_on_project: ProjectLnk) -> Option<usize> {
        self.get_depended_ons()
            .iter()
            .position(|d| d.project == depended_on_project)
    }

    pub(crate) fn get_depended_on(
        &self,
        depended_on_project: ProjectLnk,
    ) -> Option<&ProjectDependency> {
        self.get_depended_ons()
            .iter()
            .find(|d| d.project == depended_on_project)
    }

    pub(crate) fn get_depended_on_mut(
        &mut self,
        depended_on_project: ProjectLnk,
    ) -> Option<&mut ProjectDependency> {
        self.get_depended_ons_mut()
            .iter_mut()
            .find(|d| d.project == depended_on_project)
    }
    pub(crate) fn add_depended_on(&mut self, depended_on: ProjectDependency) {
        if self.has_depended_on(&depended_on.project) {
            self.remove_depended_on(depended_on.project.clone());
        }
        self.get_depended_ons_mut().push(depended_on);
    }

    pub(crate) fn remove_depended_on(&mut self, depended_on_project: ProjectLnk) -> bool {
        if let Some(pos) = self.get_internal_depended_on_id(depended_on_project) {
            self.get_depended_ons_mut().remove(pos);
            true
        } else {
            false
        }
    }

    pub(crate) fn clear_depended_ons(&mut self) -> Vec<ProjectLnk> {
        let removed_depended_ons: Vec<ProjectLnk> = self
            .get_depended_ons()
            .iter()
            .map(|d| d.project.clone())
            .collect();
        self.get_depended_ons_mut().clear();
        removed_depended_ons
    }

    pub(crate) fn get_effective_depended_on_type(
        &self,
        depended_on_project: ProjectLnk,
    ) -> Option<ProjectDependencyType> {
        self.get_depended_on(depended_on_project).map(|d| {
            d.dependency_type
                .get_effective_type(d.manual_dependency_type)
        })
    }
}
