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
    #[serde(default)]
    categories: Vec<String>,
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
    #[serde(default)]
    versions: Vec<String>,
    #[serde(default)]
    categories: Vec<String>,
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
    filename: String,
}

#[derive(Deserialize)]
struct ModrinthGameVersion {
    version: String,
    version_type: String,
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
        let url = format!(
            "https://api.modrinth.com/v2/search?query={}&facets=[[\"versions:{}\"],[\"categories:{}\"],[\"project_type:{}\"]]",
            urlencoding::encode(query),
            version,
            loader,
            project_type.id()
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
            .map(|hit| ModInfo {
                id: hit.project_id.clone(),
                slug: hit.slug,
                name: hit.title,
                icon_url: format!("https://cdn.modrinth.com/data/{}/icon.png", hit.project_id),
                description: hit.description,
                version: "Latest".to_string(),
                author: hit.author,
                download_count: hit.downloads,
                download_url: String::new(),
                supported_versions: hit.versions,
                supported_loaders: hit.categories,
                project_type: *project_type,
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
        let project_url = format!("https://api.modrinth.com/v2/project/{}", mod_id);
        let versions_url = format!("https://api.modrinth.com/v2/project/{}/version", mod_id);

        let project_response = self
            .client
            .get(&project_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let project_text = project_response.text().await?;
        let project: ModrinthProjectDetails = serde_json::from_str(&project_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse project: {}", e))?;

        let versions_response = self
            .client
            .get(&versions_url)
            .header("User-Agent", "MinecraftModDownloader/1.0")
            .send()
            .await?;

        let versions_text = versions_response.text().await?;
        let versions: Vec<ModrinthVersion> = serde_json::from_str(&versions_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse versions: {}", e))?;

        let compatible_version = versions
            .iter()
            .find(|v| {
                v.game_versions.contains(&version.to_string())
                    && v.loaders.iter().any(|l| l.eq_ignore_ascii_case(loader))
            })
            .or_else(|| versions.first())
            .ok_or_else(|| anyhow::anyhow!("No versions available for mod {}", mod_id))?;

        let download_url = compatible_version
            .files
            .first()
            .map(|f| f.url.clone())
            .unwrap_or_default();

        Ok(ModInfo {
            id: project.id.clone(),
            slug: project.slug,
            name: project.title,
            icon_url: format!("https://cdn.modrinth.com/data/{}/icon.png", project.id),
            description: project.description,
            version: compatible_version.version_number.clone(),
            author: project.team,
            download_count: project.downloads,
            download_url,
            supported_versions: compatible_version.game_versions.clone(),
            supported_loaders: compatible_version.loaders.clone(),
            project_type: ProjectType::Mod,
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
            .take(10)
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
        match project_type {
            ProjectType::Mod => Ok(vec![
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
            ]),
            ProjectType::Plugin => Ok(vec![
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
            ]),
            _ => Ok(vec![]),
        }
    }

    fn get_project_link(&self, project_type: &ProjectType, mod_id: &str) -> String {
        format!("https://modrinth.com/{}/{}", project_type.id(), mod_id)
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
}
