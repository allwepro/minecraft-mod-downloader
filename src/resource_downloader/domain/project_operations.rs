use crate::resource_downloader::domain::{ProjectDependencyType, ProjectLnk};

#[derive(Clone, Debug)]
pub enum MutationOutcome {
    Unchanged,

    // Project operations
    ProjectAdded,
    ProjectRemoved,
    ProjectDemoted,

    // Dependency operations
    VersionAdded,
    VersionRemoved,

    // Override operations
    OverrideUpdated {
        old: ProjectDependencyType,
        new: ProjectDependencyType,
        management_changed: bool,
    },

    // Global operations
    DependenciesCleared,
    OrphansCleared,

    // Failures
    NotFound,
    AlreadyExists,
}

#[derive(Clone, Debug)]
pub struct MutationResult {
    outcome: MutationOutcome,
    /// The main project being operated on
    target: Option<ProjectLnk>,
    /// Projects that were added due to the operation, e.g., dependencies added to a project
    added_projects: Vec<ProjectLnk>,
    /// Projects that were the subject of the operation, e.g., dependencies removed from a project
    changed_projects: Vec<ProjectLnk>,
    /// Projects that were cleared due to the operation
    removed_projects: Vec<ProjectLnk>,
}

impl MutationResult {
    pub fn new(outcome: MutationOutcome) -> Self {
        Self {
            outcome,
            target: None,
            added_projects: Vec::new(),
            changed_projects: Vec::new(),
            removed_projects: Vec::new(),
        }
    }

    pub fn not_found() -> Self {
        Self::new(MutationOutcome::NotFound)
    }

    pub fn unchanged() -> Self {
        Self::new(MutationOutcome::Unchanged)
    }

    pub fn with_target(mut self, target: ProjectLnk) -> Self {
        self.target = Some(target);
        self
    }

    pub fn accumulate(&mut self, other: MutationResult) {
        if self.target.is_none() {
            self.target = other.target;
        }

        self.add_added(other.added_projects);
        self.add_changed(other.changed_projects);
        self.add_removed(other.removed_projects);
    }

    pub fn chain(&mut self, other: MutationResult) -> &Self {
        self.accumulate(other);
        self
    }

    pub fn add_added(&mut self, mut projects: Vec<ProjectLnk>) -> &Self {
        self.added_projects.append(&mut projects);
        self
    }

    pub fn add_changed(&mut self, mut projects: Vec<ProjectLnk>) -> &Self {
        self.changed_projects.append(&mut projects);
        self
    }

    pub fn add_removed(&mut self, mut orphans: Vec<ProjectLnk>) -> &Self {
        self.removed_projects.append(&mut orphans);
        self
    }

    pub fn is_success(&self) -> bool {
        !matches!(
            self.outcome,
            MutationOutcome::NotFound | MutationOutcome::AlreadyExists
        )
    }

    pub fn total_affected_count(&self) -> usize {
        let mut count = 0;
        if self.target.is_some() {
            count += 1;
        }
        count
            + self.added_projects.len()
            + self.changed_projects.len()
            + self.removed_projects.len()
    }

    pub fn all_affected_ids(&self) -> Vec<ProjectLnk> {
        let mut ids = Vec::new();
        if let Some(ref t) = self.target {
            ids.push(t.clone());
        }
        ids.extend(self.added_projects.clone());
        ids.extend(self.changed_projects.clone());
        ids.extend(self.removed_projects.clone());
        ids
    }
}
