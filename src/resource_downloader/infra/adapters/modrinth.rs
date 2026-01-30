use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ProjectDependencyType, ProjectLnk, RESOURCE_TYPES, RTProjectData,
    RTProjectDependency, RTProjectVersion, ResourceType,
};
use crate::resource_downloader::infra::adapters::{ResourceProvider, ResourceProviderContext};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Deserialize)]
struct ModrinthSearchResult {
    hits: Vec<ModrinthSearchedProject>,
}

#[derive(Deserialize)]
struct ModrinthSearchedProject {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    author: String,
    downloads: u32,
    #[serde(default)]
    versions: Vec<String>,
    icon_url: String,
    #[serde(default)]
    categories: Vec<String>,
}

#[derive(Deserialize)]
struct ModrinthProjectDetails {
    slug: String,
    title: String,
    description: String,
    icon_url: String,
    #[serde(default)]
    team: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    downloads: u32,
}

#[derive(Deserialize)]
struct ModrinthDependency {
    version_id: Option<String>,
    project_id: Option<String>,
    dependency_type: String, // required, optional, incompatible, embedded
}

#[derive(Deserialize)]
struct ModrinthVersion {
    id: String,
    version_number: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    files: Vec<ModrinthVersionFile>,
    dependencies: Vec<ModrinthDependency>,
    version_type: String, // alpha, beta, release
}

#[derive(Deserialize)]
struct ModrinthVersionFile {
    id: String,
    url: String,
    primary: bool,
    size: u32,
    hashes: ModrinthVersionFileHashes,
}

#[derive(Deserialize)]
struct ModrinthGameVersion {
    version: String,
    version_type: String,
}

#[derive(Deserialize)]
struct ModrinthLoaderTag {
    name: String,
    supported_project_types: Vec<String>,
}

#[derive(Deserialize)]
struct ModrinthVersionFileHashes {
    sha1: String,
}

#[derive(Clone)]
pub struct ModrinthProvider {
    user_agent: String,
    client: Client,
}

impl ModrinthProvider {
    pub fn new(user_agent: String) -> Self {
        Self {
            user_agent,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl ResourceProvider for ModrinthProvider {
    fn get_project_link(&self, project_type: &ResourceType, project: ProjectLnk) -> String {
        format!("https://modrinth.com/{}/{}", project_type.id(), project)
    }

    async fn fetch_release_game_versions(
        &self,
        _context: &ResourceProviderContext,
    ) -> anyhow::Result<Vec<GameVersion>> {
        let url = "https://api.modrinth.com/v2/tag/game_version";

        let response: Vec<ModrinthGameVersion> = self
            .client
            .get(url)
            .header("User-Agent", self.user_agent.clone())
            .send()
            .await?
            .json()
            .await?;

        let versions = response
            .into_iter()
            .filter(|v| v.version_type == "release")
            .map(|v| GameVersion::release(v.version))
            .collect();

        Ok(versions)
    }

    async fn fetch_game_loaders_for_resource_type(
        &self,
        context: &ResourceProviderContext,
        resource_type: ResourceType,
    ) -> anyhow::Result<Vec<GameLoader>> {
        let url = "https://api.modrinth.com/v3/tag/loader";

        let response: Vec<ModrinthLoaderTag> = self
            .client
            .get(url)
            .header("User-Agent", self.user_agent.clone())
            .send()
            .await?
            .json()
            .await?;

        let target_type_id = resource_type.id();

        let loaders: Vec<(ModrinthLoaderTag, GameLoader)> = response
            .into_iter()
            .map(|tag| {
                let tag_name = tag.name.clone();
                let pretty_name = match tag_name.as_str() {
                    "neoforge" => "NeoForge".to_string(),
                    "liteloader" => "LiteLoader".to_string(),
                    "modloader" => "Risugami's ModLoader".to_string(),
                    "nilloader" => "NilLoader".to_string(),
                    "bungeecord" => "BungeeCord".to_string(),
                    "minecraft" | "datapack" => "Vanilla".to_string(),
                    _ => tag_name
                        .split('-')
                        .map(|word| {
                            let mut chars = word.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                            }
                        })
                        .collect::<Vec<String>>()
                        .join(" "),
                };

                (
                    tag,
                    GameLoader {
                        id: tag_name,
                        name: pretty_name,
                    },
                )
            })
            .collect();

        for rt in RESOURCE_TYPES {
            if rt == resource_type {
                continue;
            }
            let other_type_id = rt.id();
            let conflicting_loaders: Vec<GameLoader> = loaders
                .iter()
                .filter(|(tag, _)| {
                    tag.supported_project_types
                        .contains(&other_type_id.parse().unwrap())
                })
                .map(|g| g.1.clone())
                .collect();
            if !conflicting_loaders.is_empty() {
                context
                    .game_loader_pool
                    .warm_loader(rt, conflicting_loaders);
            }
        }

        Ok(loaders
            .into_iter()
            .filter_map(|(tag, game_loader)| {
                if tag
                    .supported_project_types
                    .contains(&target_type_id.parse().unwrap())
                {
                    Some(game_loader)
                } else {
                    None
                }
            })
            .collect())
    }

    async fn search_projects(
        &self,
        context: &ResourceProviderContext,
        query: String,
        resource_type: &ResourceType,
        version: Option<&GameVersion>,
        loader: Option<&GameLoader>,
    ) -> anyhow::Result<Vec<ProjectLnk>> {
        let url = "https://api.modrinth.com/v2/search";

        let mut facets = vec![vec![format!("project_type:{}", resource_type.id())]];

        if let Some(v) = version {
            facets.push(vec![format!("versions:{}", v.name)]);
        }

        if let Some(l) = loader {
            facets.push(vec![format!("categories:{}", l.id)]);
        }

        let facets_json = serde_json::to_string(&facets)?;

        let full_url = reqwest::Url::parse_with_params(
            url,
            &[
                ("query", query),
                ("facets", facets_json),
                ("limit", 50.to_string()),
            ],
        )?;

        let response: ModrinthSearchResult = self
            .client
            .get(full_url)
            .header("User-Agent", self.user_agent.clone())
            .send()
            .await?
            .json()
            .await?;

        let mut results = Vec::new();
        for hit in response.hits {
            let mut loaders = Vec::new();
            // categories are suboptimal since it lumped together with actual categories but let's assume no category is named like a loader
            for cat_id in hit.categories {
                if let Ok(Some(loader)) = context
                    .game_loader_pool
                    .get_loader_by_id_blocking(cat_id, resource_type)
                    .await
                {
                    loaders.push(loader);
                }
            }

            let project = ProjectLnk::from(&hit.project_id);
            let project_data = RTProjectData {
                slug: hit.slug.clone(),
                name: hit.title,
                description: hit.description,
                author: hit.author,
                icon_url: hit.icon_url,
                download_count: hit.downloads,
                supported_versions: hit
                    .versions
                    .into_iter()
                    .map(|v| GameVersion::from(&v))
                    .collect(),
                supported_loaders: loaders,
            };

            // Warm the cache by the data provided for "free"
            context
                .rt_project_pool
                .warm_slug(hit.slug.clone(), *resource_type, project.clone());
            context
                .rt_project_pool
                .warm_metadata(project.clone(), *resource_type, project_data);

            results.push(project);
        }

        Ok(results)
    }

    async fn fetch_project_from_slug(
        &self,
        context: &ResourceProviderContext,
        slug: String,
        resource_type: &ResourceType,
        version: Option<&GameVersion>,
        loader: Option<&GameLoader>,
    ) -> anyhow::Result<ProjectLnk> {
        let (project, data) = self
            .fetch_project(context, slug, resource_type, version, loader)
            .await?;

        context
            .rt_project_pool
            .warm_metadata(project.clone(), *resource_type, data);

        Ok(project)
    }

    async fn fetch_project_data(
        &self,
        context: &ResourceProviderContext,
        project: ProjectLnk,
        resource_type: &ResourceType,
        version: Option<&GameVersion>,
        loader: Option<&GameLoader>,
    ) -> anyhow::Result<RTProjectData> {
        let (project, data) = self
            .fetch_project(
                context,
                project.to_context_id().unwrap(),
                resource_type,
                version,
                loader,
            )
            .await?;

        context
            .rt_project_pool
            .warm_slug(data.slug.clone(), *resource_type, project.clone());

        Ok(data)
    }

    #[allow(clippy::wildcard_in_or_patterns)]
    async fn fetch_project_versions(
        &self,
        _context: &ResourceProviderContext,
        project: ProjectLnk,
        _resource_type: &ResourceType,
        version: Option<&GameVersion>,
        loader: Option<&GameLoader>,
    ) -> anyhow::Result<Vec<RTProjectVersion>> {
        let url = format!("https://api.modrinth.com/v2/project/{project}/version");

        let mut parms = Vec::new();
        if let Some(v) = version {
            parms.push((
                "game_versions",
                serde_json::to_string(&vec![v.name.clone()])?,
            ));
        }
        if let Some(l) = loader {
            parms.push(("loaders", serde_json::to_string(&vec![l.id.clone()])?));
        }

        let full_url = reqwest::Url::parse_with_params(&url, &parms)?;

        let modrinth_versions: Vec<ModrinthVersion> = self
            .client
            .get(full_url)
            .header("User-Agent", self.user_agent.clone())
            .send()
            .await?
            .json()
            .await?;

        let mut results = Vec::new();
        for mv in modrinth_versions {
            let primary_file = match mv
                .files
                .iter()
                .find(|f| f.primary)
                .or_else(|| mv.files.first())
            {
                Some(f) => f,
                None => continue,
            };

            let dependencies = mv
                .dependencies
                .into_iter()
                .filter_map(|d| {
                    let project_id = d.project_id?;

                    let dep_type = match d.dependency_type.as_str() {
                        "required" => ProjectDependencyType::Required,
                        "incompatible" => ProjectDependencyType::Incompatible,
                        "optional" | "embedded" | _ => ProjectDependencyType::Ignored,
                    };

                    Some(RTProjectDependency {
                        project: ProjectLnk::from(&project_id),
                        dependency_type: dep_type,
                        version_id: d.version_id,
                    })
                })
                .collect();

            results.push(RTProjectVersion {
                version_id: mv.id,
                version_name: mv.version_number,
                artifact_id: primary_file.id.clone(),
                artifact_hash: primary_file.hashes.sha1.clone(),
                channel: mv.version_type,
                depended_on: dependencies,
            });
        }

        Ok(results)
    }

    async fn load_project_icon(
        &self,
        _context: &ResourceProviderContext,
        project: ProjectLnk,
    ) -> anyhow::Result<Bytes> {
        let project_url = format!("https://api.modrinth.com/v2/project/{project}");

        let project_details: ModrinthProjectDetails = self
            .client
            .get(&project_url)
            .header("User-Agent", self.user_agent.clone())
            .send()
            .await?
            .json()
            .await?;

        let icon_url = project_details.icon_url;

        let resp = reqwest::get(icon_url).await?;
        Ok(resp.bytes().await?)
    }

    async fn download_project_artifact(
        &self,
        _context: &ResourceProviderContext,
        project: ProjectLnk,
        _resource_type: &ResourceType,
        version_id: String,
        artifact_id: String,
        destination: PathBuf,
        progress_callback: Box<dyn Fn(f32) + Send>,
    ) -> anyhow::Result<()> {
        let version_url =
            format!("https://api.modrinth.com/v2/project/{project}/version/{version_id}");

        let version_info: ModrinthVersion = self
            .client
            .get(&version_url)
            .header("User-Agent", self.user_agent.clone())
            .send()
            .await?
            .json()
            .await?;

        let target_file = version_info
            .files
            .into_iter()
            .find(|f| f.id == artifact_id)
            .ok_or_else(|| {
                anyhow::anyhow!("Artifact {artifact_id} not found in version {version_id}")
            })?;

        let response = self
            .client
            .get(&target_file.url)
            .header("User-Agent", self.user_agent.clone())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download file: status {}",
                response.status()
            ));
        }

        let total_size = response.content_length().unwrap_or(target_file.size as u64);

        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::File::create(&destination).await?;
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        progress_callback(0.0);

        while let Some(item) = stream.next().await {
            let chunk = item?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let progress = downloaded as f32 / total_size as f32;
                progress_callback(progress);
            }
        }

        file.flush().await?;
        progress_callback(1.0);

        Ok(())
    }
}

impl ModrinthProvider {
    async fn fetch_project(
        &self,
        context: &ResourceProviderContext,
        id_or_slug: String,
        resource_type: &ResourceType,
        _version: Option<&GameVersion>,
        _loader: Option<&GameLoader>,
    ) -> anyhow::Result<(ProjectLnk, RTProjectData)> {
        let project_url = format!("https://api.modrinth.com/v2/project/{id_or_slug}");
        let team_url = format!("https://api.modrinth.com/v2/project/{id_or_slug}/members");

        let project_response = self
            .client
            .get(&project_url)
            .header("User-Agent", self.user_agent.clone())
            .send()
            .await?;
        let project_text = project_response.text().await?;
        let project_details: ModrinthProjectDetails = serde_json::from_str(&project_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse project: {e}"))?;

        let author = match self
            .client
            .get(&team_url)
            .header("User-Agent", self.user_agent.clone())
            .send()
            .await
        {
            Ok(resp) => {
                #[derive(Deserialize)]
                struct TeamMember {
                    role: String,
                    user: TeamUser,
                }
                #[derive(Deserialize)]
                struct TeamUser {
                    username: String,
                }
                match resp.json::<Vec<TeamMember>>().await {
                    Ok(members) => members
                        .into_iter()
                        .find(|m| m.role == "Owner")
                        .map(|m| m.user.username.clone())
                        .unwrap_or_else(|| project_details.team.clone()),
                    Err(_) => project_details.team.clone(),
                }
            }
            Err(_) => project_details.team.clone(),
        };

        let mut loaders = Vec::new();
        for cat_id in project_details.loaders {
            if let Ok(Some(loader)) = context
                .game_loader_pool
                .get_loader_by_id_blocking(cat_id, resource_type)
                .await
            {
                loaders.push(loader);
            }
        }

        Ok((
            ProjectLnk::from(&id_or_slug),
            RTProjectData {
                slug: project_details.slug,
                name: project_details.title,
                description: project_details.description,
                author,
                icon_url: project_details.icon_url,
                supported_versions: project_details
                    .game_versions
                    .into_iter()
                    .map(|v| GameVersion::from(&v))
                    .collect(),
                supported_loaders: loaders,
                download_count: project_details.downloads,
            },
        ))
    }
}

#[derive(Deserialize, Debug)]
struct ModrinthCollection {
    #[serde(default)]
    name: String,
    #[allow(dead_code)]
    #[serde(default, alias = "title")]
    description: Option<String>,
    #[serde(default)]
    projects: Vec<serde_json::Value>,
}

#[derive(Deserialize, Clone)]
struct ModrinthCollectionProject {
    id: String,
    title: String,
    project_type: String,
    #[serde(default)]
    loaders: Vec<String>,
}
impl ModrinthProvider {
    pub async fn fetch_collection(
        &self,
        context: &ResourceProviderContext,
        collection_id: &str,
    ) -> anyhow::Result<(
        String,
        HashMap<ResourceType, (String, GameLoader)>,
        Vec<(String, String, ResourceType)>,
    )> {
        use crate::resource_downloader::domain::{
            GameLoader as ProjectLoader, ResourceType as ProjectType,
        };
        let collection_url = format!("https://api.modrinth.com/v3/collection/{collection_id}");

        let response = self
            .client
            .get(&collection_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!(
                "Collection not found. Make sure the collection ID is correct and the collection is public."
            );
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: {status} - {error_text}");
        }

        let response_text = response.text().await?;
        let collection: ModrinthCollection = serde_json::from_str(&response_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse collection: {e}"))?;

        if collection.projects.is_empty() {
            return Ok((collection.name, HashMap::new(), Vec::new()));
        }

        let project_ids: Vec<String> = collection
            .projects
            .iter()
            .filter_map(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else if let Some(obj) = v.as_object() {
                    obj.get("id")
                        .or_else(|| obj.get("project_id"))
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();

        if project_ids.is_empty() {
            return Ok((collection.name, HashMap::new(), Vec::new()));
        }

        let project_ids_str = project_ids.join(r#"",""#);
        let projects_url =
            format!(r#"https://api.modrinth.com/v2/projects?ids=["{project_ids_str}"]"#);

        let projects_response = self
            .client
            .get(&projects_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let projects_status = projects_response.status();
        if !projects_status.is_success() {
            let error_text = projects_response.text().await.unwrap_or_default();
            anyhow::bail!("API error: {projects_status} - {error_text}");
        }

        let projects_text = projects_response.text().await?;
        let projects: Vec<ModrinthCollectionProject> = serde_json::from_str(&projects_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse projects: {e}"))?;

        let plugin_loaders = context
            .game_loader_pool
            .get_loaders_blocking(ProjectType::Plugin)
            .await;
        let plugin_loader_ids: Vec<String> = plugin_loaders.map_or(vec![], |r| {
            r.map(|o| o.iter().map(|l| l.id.clone()).collect())
                .unwrap_or_default()
        });

        let datapack_loaders = context
            .game_loader_pool
            .get_loaders_blocking(ProjectType::Datapack)
            .await;
        let datapack_loader_ids: Vec<String> = datapack_loaders.map_or(vec![], |r| {
            r.map(|o| o.iter().map(|l| l.id.clone()).collect())
                .unwrap_or_default()
        });

        let mod_loaders = context
            .game_loader_pool
            .get_loaders_blocking(ProjectType::Mod)
            .await;
        let mod_loader_ids: Vec<String> = mod_loaders.map_or(vec![], |r| {
            r.map(|o| o.iter().map(|l| l.id.clone()).collect())
                .unwrap_or_default()
        });

        let valid_projects: Vec<(String, String, ProjectType)> = projects
            .iter()
            .flat_map(|p| {
                if p.project_type == "modpack" {
                    return Vec::new();
                }

                let mut detected_types = HashSet::new();

                if !p.loaders.is_empty() {
                    if p.loaders.iter().any(|l| mod_loader_ids.contains(l)) {
                        detected_types.insert(ProjectType::Mod);
                    }
                    if p.loaders.iter().any(|l| plugin_loader_ids.contains(l)) {
                        detected_types.insert(ProjectType::Plugin);
                    }
                    if p.loaders.iter().any(|l| datapack_loader_ids.contains(l)) {
                        detected_types.insert(ProjectType::Datapack);
                    }
                }

                if let Some(declared_type) = ProjectType::from_str(p.project_type.clone()) {
                    detected_types.insert(declared_type);
                }

                detected_types
                    .into_iter()
                    .map(move |pt| (p.id.clone(), p.title.clone(), pt))
                    .collect::<Vec<_>>()
            })
            .collect();

        if valid_projects.is_empty() {
            return Ok((collection.name, HashMap::new(), Vec::new()));
        }

        let mut projects_by_type: HashMap<ProjectType, Vec<&(String, String, ProjectType)>> =
            HashMap::new();
        for project in &valid_projects {
            projects_by_type.entry(project.2).or_default().push(project);
        }

        let mut result: HashMap<ProjectType, (String, ProjectLoader)> = HashMap::new();

        for (project_type, type_projects) in &projects_by_type {
            let sample_size = type_projects.len().min(3);
            let mut sample_versions: Vec<Vec<ModrinthVersion>> = Vec::new();

            for project in type_projects.iter().take(sample_size) {
                let versions_url =
                    format!("https://api.modrinth.com/v2/project/{}/version", project.0);

                if let Ok(resp) = self
                    .client
                    .get(&versions_url)
                    .header("User-Agent", "MinecraftModDownloader/1.0")
                    .send()
                    .await
                    && let Ok(versions) = resp.json::<Vec<ModrinthVersion>>().await
                {
                    sample_versions.push(versions);
                }
            }

            if sample_versions.is_empty() {
                continue;
            }

            let mut loader_counts: HashMap<String, usize> = HashMap::new();
            for versions in &sample_versions {
                for version in versions {
                    for loader in &version.loaders {
                        *loader_counts.entry(loader.clone()).or_insert(0) += 1;
                    }
                }
            }

            let most_common_loader = loader_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(loader, _)| loader)
                .unwrap_or_else(|| match project_type {
                    ProjectType::Mod => "fabric".to_string(),
                    ProjectType::Plugin => "paper".to_string(),
                    ProjectType::ResourcePack => "minecraft".to_string(),
                    ProjectType::Shader => "iris".to_string(),
                    ProjectType::Datapack => "datapack".to_string(),
                });

            let mut common_game_versions: Option<HashSet<String>> = None;
            for versions in &sample_versions {
                let game_versions: HashSet<String> = versions
                    .iter()
                    .flat_map(|v| v.game_versions.iter().cloned())
                    .collect();

                common_game_versions = match common_game_versions {
                    None => Some(game_versions),
                    Some(existing) => {
                        Some(existing.intersection(&game_versions).cloned().collect())
                    }
                };
            }

            let most_recent_version = if let Some(common_versions) = common_game_versions {
                let mut sorted_versions: Vec<String> = common_versions.into_iter().collect();
                sorted_versions.sort_by(|a, b| {
                    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
                    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
                    b_parts.cmp(&a_parts)
                });
                sorted_versions.first().cloned().unwrap_or_default()
            } else {
                String::new()
            };

            let loader_name = if most_common_loader.is_empty() {
                String::new()
            } else {
                most_common_loader
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_default()
                    + &most_common_loader[1..]
            };

            let mod_loader = ProjectLoader {
                id: most_common_loader.clone(),
                name: loader_name,
            };

            result.insert(*project_type, (most_recent_version, mod_loader));
        }

        Ok((collection.name, result, valid_projects))
    }
}
