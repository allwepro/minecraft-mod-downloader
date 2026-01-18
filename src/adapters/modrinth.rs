use crate::domain::{MinecraftVersion, ModInfo, ModLoader, ModProvider, ProjectType};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

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
    icon_url: String,
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
    #[serde(default, alias = "title")]
    description: Option<String>,
    #[serde(default, alias = "project_ids")]
    projects: Vec<String>,
}

#[derive(Deserialize)]
struct ModrinthProjectBasic {
    id: String,
    title: String,
    project_type: String,
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
                    .map(|f| {
                        log::debug!("Download URL: {}", f.url);
                        f.url.clone()
                    })
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

                (
                    String::new(), // No specific version
                    String::new(), // No download URL
                    all_versions,
                    all_loaders,
                )
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
            ProjectType::Modpack => vec![ModLoader {
                id: "modpack".to_string(),
                name: "Modpack".to_string(),
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
        String,
        String,
        String,
        Vec<(String, String, ProjectType)>,
    )> {
        let collection_url = format!("https://api.modrinth.com/v3/collection/{collection_id}");

        log::info!("Fetching collection from: {}", collection_url);

        let response = self
            .client
            .get(&collection_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let status = response.status();
        log::info!("Collection API response status: {}", status);

        if status == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!(
                "Collection not found. Make sure the collection ID is correct and the collection is public."
            );
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Collection API error: {} - {}", status, error_text);
            anyhow::bail!("API error: {} - {}", status, error_text);
        }

        let response_text = response.text().await?;
        log::info!(
            "Collection API response length: {} bytes",
            response_text.len()
        );
        log::debug!(
            "Collection API response: {}",
            &response_text[..response_text.len().min(1000)]
        );

        let collection: ModrinthCollection = serde_json::from_str(&response_text).map_err(|e| {
            log::error!(
                "Failed to parse: {} - Response: {}",
                e,
                &response_text[..response_text.len().min(500)]
            );
            anyhow::anyhow!("Failed to parse collection: {}", e)
        })?;

        log::info!(
            "Parsed collection '{}' with {} projects",
            collection.name,
            collection.projects.len()
        );

        if collection.projects.is_empty() {
            return Ok((
                collection.name,
                collection.description.unwrap_or_default(),
                String::new(),
                String::new(),
                Vec::new(),
            ));
        }

        let project_ids = collection.projects.join(r#"",""#);
        let projects_url = format!(
            r#"https://api.modrinth.com/v2/projects?ids=["{}"]"#,
            project_ids
        );

        log::debug!("Fetching projects from: {}", projects_url);

        let projects_response = self
            .client
            .get(&projects_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let projects_status = projects_response.status();
        if !projects_status.is_success() {
            let error_text = projects_response.text().await.unwrap_or_default();
            log::warn!("Projects API error: {} - {}", projects_status, error_text);
            // Return collection with just IDs if we can't fetch project names
            let result: Vec<(String, String, ProjectType)> = collection
                .projects
                .into_iter()
                .map(|id| (id.clone(), id, ProjectType::Mod))
                .collect();
            return Ok((
                collection.name,
                collection.description.unwrap_or_default(),
                String::new(),
                String::new(),
                result,
            ));
        }

        let projects: Vec<ModrinthProjectBasic> = projects_response.json().await?;

        let project_map: std::collections::HashMap<String, (String, ProjectType)> = projects
            .into_iter()
            .map(|p| {
                let pt = match p.project_type.as_str() {
                    "mod" => ProjectType::Mod,
                    "resourcepack" => ProjectType::ResourcePack,
                    "shader" => ProjectType::Shader,
                    "datapack" => ProjectType::Datapack,
                    "modpack" => ProjectType::Modpack,
                    "plugin" => ProjectType::Plugin,
                    _ => ProjectType::Mod,
                };
                (p.id.clone(), (p.title, pt))
            })
            .collect();

        let (recommended_version, recommended_loader) = self
            .extract_recommended_from_projects(&collection.projects)
            .await;

        let result: Vec<(String, String, ProjectType)> = collection
            .projects
            .into_iter()
            .map(|id| {
                if let Some((name, pt)) = project_map.get(&id) {
                    (id, name.clone(), *pt)
                } else {
                    (id.clone(), id, ProjectType::Mod)
                }
            })
            .collect();

        Ok((
            collection.name,
            collection.description.unwrap_or_default(),
            recommended_version,
            recommended_loader,
            result,
        ))
    }

    async fn extract_recommended_from_projects(&self, project_ids: &[String]) -> (String, String) {
        use std::collections::HashMap;

        let mut version_counts: HashMap<String, usize> = HashMap::new();
        let mut loader_counts: HashMap<String, usize> = HashMap::new();

        for project_id in project_ids.iter().take(5) {
            match self.get_project_versions(project_id).await {
                Ok(versions) => {
                    for version in versions.iter().take(3) {
                        for game_version in &version.game_versions {
                            *version_counts.entry(game_version.clone()).or_insert(0) += 1;
                        }
                        for loader in &version.loaders {
                            *loader_counts.entry(loader.clone()).or_insert(0) += 1;
                        }
                    }
                }
                Err(e) => log::debug!("Failed to fetch versions for {project_id}: {e}"),
            }
        }

        let recommended_version = version_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(v, _)| v)
            .unwrap_or_else(|| "1.20.1".to_string());

        let recommended_loader = loader_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(l, _)| l)
            .unwrap_or_else(|| "fabric".to_string());

        log::info!(
            "Recommended settings for collection: version={}, loader={}",
            recommended_version,
            recommended_loader
        );

        (recommended_version, recommended_loader)
    }

    async fn get_project_versions(&self, project_id: &str) -> anyhow::Result<Vec<ModrinthVersion>> {
        let versions_url = format!("https://api.modrinth.com/v2/project/{project_id}/version");
        let response = self
            .client
            .get(&versions_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let versions: Vec<ModrinthVersion> = response.json().await?;
        Ok(versions)
    }
}
