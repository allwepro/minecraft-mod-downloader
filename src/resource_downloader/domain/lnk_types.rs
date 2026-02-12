use crate::resource_downloader::domain::Project;
use crate::resource_downloader::domain::project_list::ProjectList;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fmt::{Display, Formatter};

/// A simple representation of a project that can be used in a list.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectLnk {
    project_id: String,
}

impl ProjectLnk {
    pub fn is_for(&self, project_list: &Project) -> bool {
        self.project_id == project_list.get_id()
    }

    pub fn to_context_id(&self) -> String {
        self.project_id.clone()
    }
}

impl From<&Project> for ProjectLnk {
    fn from(project: &Project) -> Self {
        Self {
            project_id: project.get_id(),
        }
    }
}
impl From<&String> for ProjectLnk {
    fn from(project_id: &String) -> Self {
        Self {
            project_id: project_id.clone(),
        }
    }
}
impl From<ProjectLnk> for String {
    fn from(val: ProjectLnk) -> Self {
        val.project_id
    }
}

impl Serialize for ProjectLnk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.project_id)
    }
}

impl Display for ProjectLnk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.project_id)
    }
}

impl<'de> Deserialize<'de> for ProjectLnk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ProjectLnk { project_id: s })
    }
}

/// A simple representation of a project list that can be used in a list.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ListLnk {
    list_id: String,
}

impl ListLnk {
    pub fn new(list_id: String) -> Self {
        Self { list_id }
    }

    pub fn is_for(&self, project_list: &ProjectList) -> bool {
        self.list_id == project_list.get_id()
    }

    pub fn to_context_id(&self) -> String {
        self.list_id.clone()
    }
}

impl From<&ProjectList> for ListLnk {
    fn from(project_list: &ProjectList) -> Self {
        Self::new(project_list.get_id())
    }
}

impl From<ListLnk> for String {
    fn from(val: ListLnk) -> Self {
        val.list_id
    }
}

impl Serialize for ListLnk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.list_id)
    }
}

impl Display for ListLnk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.list_id)
    }
}

impl<'de> Deserialize<'de> for ListLnk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ListLnk { list_id: s })
    }
}

/// A simple representation of a list group that can contain lists.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ListGroupLnk {
    group_id: String,
}

impl ListGroupLnk {
    pub fn new(group_id: String) -> Self {
        Self { group_id }
    }

    pub fn to_context_id(&self) -> String {
        self.group_id.clone()
    }
}

impl PartialEq<ListGroupLnk> for &mut ListGroupLnk {
    fn eq(&self, other: &ListGroupLnk) -> bool {
        self.group_id == other.group_id
    }
}

impl From<String> for ListGroupLnk {
    fn from(folder_id: String) -> Self {
        Self {
            group_id: folder_id,
        }
    }
}

impl From<ListGroupLnk> for String {
    fn from(val: ListGroupLnk) -> Self {
        val.group_id
    }
}

impl Serialize for ListGroupLnk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.group_id)
    }
}

impl Display for ListGroupLnk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.group_id)
    }
}

impl<'de> Deserialize<'de> for ListGroupLnk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ListGroupLnk { group_id: s })
    }
}
