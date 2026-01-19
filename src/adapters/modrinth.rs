use crate::domain::{MinecraftVersion, ModInfo, ModLoader, ModProvider, ProjectType};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ModrinthProvider {
    client: Client,
}

impl ModrinthProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[derive(Deserialize)]
struct ModrinthSearchResult {
    hits: Vec<ModrinthProject>,
}

#[derive(Deserialize)]
struct ModrinthProject {
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
    project_type: String,
}

#[derive(Deserialize)]
struct ModrinthProjectDetails {
    id: String,
    slug: String,
    title: String,
    description: String,
    #[serde(default)]
    team: String,
    downloads: u32,
    #[allow(dead_code)]
    #[serde(default)]
    versions: Vec<String>,
    icon_url: String,
    #[allow(dead_code)]
    #[serde(default)]
    categories: Vec<String>,
    project_type: String,
}

#[derive(Deserialize)]
struct ModrinthVersion {
    id: String,
    version_number: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    files: Vec<ModrinthFile>,
}

#[derive(Deserialize)]
struct ModrinthFile {
    url: String,
}

#[derive(Deserialize)]
struct ModrinthGameVersion {
    version: String,
    version_type: String,
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
struct ModrinthProjectBasic {
    id: String,
    title: String,
    project_type: String,
    #[serde(default)]
    loaders: Vec<String>,
}

fn calculate_version_distance(target: &[u32], candidate: &[u32]) -> i64 {
    let max_len = target.len().max(candidate.len());
    let mut distance: i64 = 0;

    for i in 0..max_len {
        let target_part = target.get(i).copied().unwrap_or(0) as i64;
        let candidate_part = candidate.get(i).copied().unwrap_or(0) as i64;
        let diff = (target_part - candidate_part).abs();
        let weight = 10000_i64.pow((max_len - i - 1) as u32);
        distance += diff * weight;
    }

    distance
}

#[async_trait]
impl ModProvider for ModrinthProvider {
    async fn search_mods(
        &self,
        query: &str,
        version: &str,
        loader: &str,
        project_type: &ProjectType,
    ) -> anyhow::Result<Vec<ModInfo>> {
        let base = format!(
            "https://api.modrinth.com/v2/search?query={}",
            urlencoding::encode(query)
        );

        let mut facets = vec![format!("\"project_type:{}\"", project_type.id())];

        if !version.is_empty() {
            facets.push(format!("\"versions:{version}\""));
        }
        if !loader.is_empty() && *project_type == ProjectType::Mod {
            facets.push(format!("\"categories:{loader}\""));
        }

        let url = format!(
            "{}&facets=[{}]",
            base,
            facets
                .iter()
                .map(|f| format!("[{f}]"))
                .collect::<Vec<_>>()
                .join(",")
        );

        let response: ModrinthSearchResult = self
            .client
            .get(&url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?
            .json()
            .await?;

        let mods = response
            .hits
            .into_iter()
            .map(|hit| {
                let pt = match hit.project_type.as_str() {
                    "mod" => ProjectType::Mod,
                    "resourcepack" => ProjectType::ResourcePack,
                    "shader" => ProjectType::Shader,
                    "datapack" => ProjectType::Datapack,
                    "plugin" => ProjectType::Plugin,
                    _ => ProjectType::Mod,
                };

                ModInfo {
                    id: hit.project_id,
                    slug: hit.slug,
                    name: hit.title,
                    description: hit.description,
                    version: String::new(),
                    author: hit.author,
                    icon_url: hit.icon_url,
                    download_count: hit.downloads,
                    download_url: String::new(),
                    supported_versions: hit.versions,
                    supported_loaders: hit.categories,
                    project_type: pt,
                }
            })
            .collect();

        Ok(mods)
    }

    async fn fetch_mod_details(
        &self,
        mod_id: &str,
        version: &str,
        loader: &str,
    ) -> anyhow::Result<ModInfo> {
        let project_url = format!("https://api.modrinth.com/v2/project/{mod_id}");
        let versions_url = format!("https://api.modrinth.com/v2/project/{mod_id}/version");
        let team_url = format!("https://api.modrinth.com/v2/project/{mod_id}/members");

        let project_response = self
            .client
            .get(&project_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let project_text = project_response.text().await?;
        let project: ModrinthProjectDetails = serde_json::from_str(&project_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse project: {e}"))?;

        let author = match self
            .client
            .get(&team_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
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
                        .unwrap_or_else(|| project.team.clone()),
                    Err(_) => project.team.clone(),
                }
            }
            Err(_) => project.team.clone(),
        };

        let project_type = match project.project_type.as_str() {
            "mod" => ProjectType::Mod,
            "resourcepack" => ProjectType::ResourcePack,
            "shader" => ProjectType::Shader,
            "datapack" => ProjectType::Datapack,
            "plugin" => ProjectType::Plugin,
            _ => ProjectType::Mod,
        };

        let versions_response = self
            .client
            .get(&versions_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let versions_text = versions_response.text().await?;
        let versions: Vec<ModrinthVersion> = serde_json::from_str(&versions_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse versions: {e}"))?;

        log::debug!(
            "Mod {} has {} versions. Looking for version={} loader={}",
            mod_id,
            versions.len(),
            version,
            loader
        );

        let compatible_version = versions
            .iter()
            .find(|v| {
                let version_match = v.game_versions.contains(&version.to_string());
                let loader_match = loader.is_empty()
                    || v.loaders.iter().any(|l| l.eq_ignore_ascii_case(loader));

                if !version_match {
                    log::debug!(
                        "Version {} doesn't match for mod {} (has: {:?})",
                        version,
                        mod_id,
                        v.game_versions
                    );
                }
                if !loader_match {
                    log::debug!(
                        "Loader {} doesn't match for mod {} (has: {:?})",
                        loader,
                        mod_id,
                        v.loaders
                    );
                }

                version_match && loader_match
            })
            .or_else(|| {
                if !loader.is_empty() {
                    log::warn!(
                        "No exact match for mod {mod_id} version={version} loader={loader}. Looking for closest version with same loader."
                    );

                    let loader_compatible: Vec<&ModrinthVersion> = versions
                        .iter()
                        .filter(|v| v.loaders.iter().any(|l| l.eq_ignore_ascii_case(loader)))
                        .collect();

                    if !loader_compatible.is_empty() {
                        let target_parts: Vec<u32> = version
                            .split('.')
                            .filter_map(|s| s.parse::<u32>().ok())
                            .collect();

                        let mut best_match: Option<(&ModrinthVersion, i64)> = None;

                        for mod_version in &loader_compatible {
                            for game_version in &mod_version.game_versions {
                                let game_parts: Vec<u32> = game_version
                                    .split('.')
                                    .filter_map(|s| s.parse::<u32>().ok())
                                    .collect();

                                let distance = calculate_version_distance(&target_parts, &game_parts);

                                if distance == 0 {
                                    log::info!(
                                        "Found exact version match '{}' (game version {}) for mod {} with correct loader {}",
                                        mod_version.version_number,
                                        game_version,
                                        mod_id,
                                        loader
                                    );
                                    return Some(*mod_version);
                                }

                                match best_match {
                                    None => best_match = Some((*mod_version, distance)),
                                    Some((_, best_distance)) if distance < best_distance => {
                                        best_match = Some((*mod_version, distance));
                                    }
                                    _ => {}
                                }
                            }
                        }

                        if let Some((best_version, distance)) = best_match {
                            log::info!(
                                "Using closest version '{}' for mod {} with correct loader {} (distance {} from target version {})",
                                best_version.version_number,
                                mod_id,
                                loader,
                                distance,
                                version
                            );
                            return Some(best_version);
                        }

                        log::warn!(
                            "Could not parse versions, using latest '{}' for mod {} with correct loader {}",
                            loader_compatible[0].version_number,
                            mod_id,
                            loader
                        );
                        return Some(loader_compatible[0]);
                    }

                    log::warn!(
                        "No versions found for mod {mod_id} with loader {loader}. Mod may be incompatible."
                    );
                    None
                } else {
                    log::warn!(
                        "No exact match for mod {mod_id} version={version}. Using first available version."
                    );
                    versions.first()
                }
            });

        let (version_number, download_url, supported_versions, supported_loaders) =
            if let Some(compatible_version) = compatible_version {
                log::debug!(
                    "Selected version '{}' for mod {} (file count: {}, id: {})",
                    compatible_version.version_number,
                    mod_id,
                    compatible_version.files.len(),
                    compatible_version.id
                );

                let download_url = compatible_version
                    .files
                    .first()
                    .map(|f| f.url.clone())
                    .unwrap_or_default();

                (
                    compatible_version.version_number.clone(),
                    download_url,
                    compatible_version.game_versions.clone(),
                    compatible_version.loaders.clone(),
                )
            } else {
                log::info!(
                    "Using fallback for mod {mod_id}: no compatible version for {version}/{loader}"
                );

                let all_versions: Vec<String> = versions
                    .iter()
                    .flat_map(|v| v.game_versions.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                let all_loaders: Vec<String> = versions
                    .iter()
                    .flat_map(|v| v.loaders.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                (String::new(), String::new(), all_versions, all_loaders)
            };

        log::debug!(
            "Creating ModInfo: version='{}' (len={}), download_url='{}' (len={})",
            version_number,
            version_number.len(),
            download_url,
            download_url.len()
        );

        Ok(ModInfo {
            id: project.id,
            slug: project.slug,
            name: project.title,
            description: project.description,
            version: version_number,
            author,
            icon_url: project.icon_url,
            download_count: project.downloads,
            download_url,
            supported_versions,
            supported_loaders,
            project_type,
        })
    }

    async fn get_minecraft_versions(&self) -> anyhow::Result<Vec<MinecraftVersion>> {
        let response: Vec<ModrinthGameVersion> = self
            .client
            .get("https://api.modrinth.com/v2/tag/game_version")
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?
            .json()
            .await?;

        let versions = response
            .into_iter()
            .filter(|v| v.version_type == "release")
            .map(|v| MinecraftVersion {
                id: v.version.clone(),
                name: v.version,
            })
            .collect();

        Ok(versions)
    }

    async fn get_mod_loaders_for_type(
        &self,
        project_type: ProjectType,
    ) -> anyhow::Result<Vec<ModLoader>> {
        //for dynamic loading https://api.modrinth.com/v2/tag/loader
        Ok(match project_type {
            ProjectType::Mod => vec![
                ModLoader {
                    id: "fabric".to_string(),
                    name: "Fabric".to_string(),
                },
                ModLoader {
                    id: "forge".to_string(),
                    name: "Forge".to_string(),
                },
                ModLoader {
                    id: "neoforge".to_string(),
                    name: "NeoForge".to_string(),
                },
                ModLoader {
                    id: "quilt".to_string(),
                    name: "Quilt".to_string(),
                },
                ModLoader {
                    id: "babric".to_string(),
                    name: "Babric".to_string(),
                },
                ModLoader {
                    id: "bta-babric".to_string(),
                    name: "BTA (Babric)".to_string(),
                },
                ModLoader {
                    id: "java-agent".to_string(),
                    name: "Java Agent".to_string(),
                },
                ModLoader {
                    id: "legacy-fabric".to_string(),
                    name: "Legacy Fabric".to_string(),
                },
                ModLoader {
                    id: "liteloader".to_string(),
                    name: "LiteLoader".to_string(),
                },
                ModLoader {
                    id: "modloader".to_string(),
                    name: "Risugami's ModLoader".to_string(),
                },
                ModLoader {
                    id: "nilloader".to_string(),
                    name: "NilLoader".to_string(),
                },
                ModLoader {
                    id: "ornithe".to_string(),
                    name: "Ornithe".to_string(),
                },
                ModLoader {
                    id: "rift".to_string(),
                    name: "Rift".to_string(),
                },
            ],
            ProjectType::Plugin => vec![
                ModLoader {
                    id: "paper".to_string(),
                    name: "Paper".to_string(),
                },
                ModLoader {
                    id: "spigot".to_string(),
                    name: "Spigot".to_string(),
                },
                ModLoader {
                    id: "bukkit".to_string(),
                    name: "Bukkit".to_string(),
                },
                ModLoader {
                    id: "folia".to_string(),
                    name: "Folia".to_string(),
                },
                ModLoader {
                    id: "purpur".to_string(),
                    name: "Purpur".to_string(),
                },
                ModLoader {
                    id: "sponge".to_string(),
                    name: "Sponge".to_string(),
                },
                ModLoader {
                    id: "velocity".to_string(),
                    name: "Velocity".to_string(),
                },
                ModLoader {
                    id: "bungeecord".to_string(),
                    name: "BungeeCord".to_string(),
                },
                ModLoader {
                    id: "geyser".to_string(),
                    name: "Geyser".to_string(),
                },
                ModLoader {
                    id: "waterfall".to_string(),
                    name: "Waterfall".to_string(),
                },
            ],
            ProjectType::ResourcePack => vec![ModLoader {
                id: "minecraft".to_string(),
                name: "Vanilla".to_string(),
            }],
            ProjectType::Shader => vec![
                ModLoader {
                    id: "vanilla".to_string(),
                    name: "Vanilla".to_string(),
                },
                ModLoader {
                    id: "iris".to_string(),
                    name: "Iris".to_string(),
                },
                ModLoader {
                    id: "optifine".to_string(),
                    name: "OptiFine".to_string(),
                },
                ModLoader {
                    id: "canvas".to_string(),
                    name: "Canvas".to_string(),
                },
            ],
            ProjectType::Datapack => vec![ModLoader {
                id: "datapack".to_string(),
                name: "Vanilla".to_string(),
            }],
        })
    }

    async fn download_mod(
        &self,
        download_url: &str,
        destination: &std::path::Path,
        progress_callback: Box<dyn Fn(f32) + Send>,
    ) -> anyhow::Result<()> {
        let response = self
            .client
            .get(download_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        tokio::fs::create_dir_all(destination.parent().unwrap()).await?;
        let mut file = tokio::fs::File::create(destination).await?;

        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let progress = downloaded as f32 / total_size as f32;
                progress_callback(progress);
            }
        }

        file.flush().await?;
        Ok(())
    }

    fn get_project_link(&self, project_type: &ProjectType, mod_id: &str) -> String {
        format!("https://modrinth.com/{}/{}", project_type.id(), mod_id)
    }
}

impl ModrinthProvider {
    pub async fn fetch_collection(
        &self,
        collection_id: &str,
    ) -> anyhow::Result<(
        String,
        HashMap<ProjectType, (String, ModLoader)>,
        Vec<(String, String, ProjectType)>,
    )> {
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
        let projects: Vec<ModrinthProjectBasic> = serde_json::from_str(&projects_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse projects: {e}"))?;

        let plugin_loaders = self
            .get_mod_loaders_for_type(ProjectType::Plugin)
            .await
            .unwrap_or_default();
        let plugin_loader_ids: Vec<String> = plugin_loaders.iter().map(|l| l.id.clone()).collect();

        let datapack_loaders = self
            .get_mod_loaders_for_type(ProjectType::Datapack)
            .await
            .unwrap_or_default();
        let datapack_loader_ids: Vec<String> =
            datapack_loaders.iter().map(|l| l.id.clone()).collect();

        let mod_loaders = self
            .get_mod_loaders_for_type(ProjectType::Mod)
            .await
            .unwrap_or_default();
        let mod_loader_ids: Vec<String> = mod_loaders.iter().map(|l| l.id.clone()).collect();

        let valid_projects: Vec<(String, String, ProjectType)> = projects
            .iter()
            .filter_map(|p| {
                if p.project_type == "modpack" {
                    return None;
                }

                let project_type = if !p.loaders.is_empty() {
                    let is_plugin = p.loaders.iter().any(|l| plugin_loader_ids.contains(l));
                    let is_datapack = p.loaders.iter().any(|l| datapack_loader_ids.contains(l));
                    let is_mod = p.loaders.iter().any(|l| mod_loader_ids.contains(l));

                    let declared_type = match p.project_type.as_str() {
                        "mod" if is_mod => Some(ProjectType::Mod),
                        "plugin" if is_plugin => Some(ProjectType::Plugin),
                        "datapack" if is_datapack => Some(ProjectType::Datapack),
                        "resourcepack" => Some(ProjectType::ResourcePack),
                        "shader" => Some(ProjectType::Shader),
                        _ => None,
                    };

                    if declared_type.is_some() {
                        declared_type
                    } else if is_plugin {
                        Some(ProjectType::Plugin)
                    } else if is_datapack {
                        Some(ProjectType::Datapack)
                    } else if is_mod {
                        Some(ProjectType::Mod)
                    } else {
                        match p.project_type.as_str() {
                            "mod" => Some(ProjectType::Mod),
                            "resourcepack" => Some(ProjectType::ResourcePack),
                            "shader" => Some(ProjectType::Shader),
                            "datapack" => Some(ProjectType::Datapack),
                            "plugin" => Some(ProjectType::Plugin),
                            _ => None,
                        }
                    }
                } else {
                    match p.project_type.as_str() {
                        "mod" => Some(ProjectType::Mod),
                        "resourcepack" => Some(ProjectType::ResourcePack),
                        "shader" => Some(ProjectType::Shader),
                        "datapack" => Some(ProjectType::Datapack),
                        "plugin" => Some(ProjectType::Plugin),
                        _ => None,
                    }
                };

                project_type.map(|pt| (p.id.clone(), p.title.clone(), pt))
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

        let mut result: HashMap<ProjectType, (String, ModLoader)> = HashMap::new();

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

            let mut common_game_versions: Option<std::collections::HashSet<String>> = None;
            for versions in &sample_versions {
                let game_versions: std::collections::HashSet<String> = versions
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

            let mod_loader = ModLoader {
                id: most_common_loader.clone(),
                name: loader_name,
            };

            result.insert(*project_type, (most_recent_version, mod_loader));
        }

        Ok((collection.name, result, valid_projects))
    }
}
