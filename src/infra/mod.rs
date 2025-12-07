use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/*
reqwest for:
    searching mods via Modrinth API
    fetching version metadata
    downloading files

serde_json:
    deserializing JSON returned by Modrinth

std::fs
    saving downloaded .jar files
    creating temp dirs

implementing ModRepo & ArtifactDownloader from domain
 */

const API_URL: &str = "https://api.modrinth.com/v2/project";

// structs might have to made public if needed in other modules or for JSON parsing
#[derive(Deserialize, Debug)]
struct ModrinthFile {
    url: String,
    filename: String,
}

#[derive(Deserialize, Debug)]
struct ModrinthVersion {
    files: Vec<ModrinthFile>,
}

pub async fn download_project(
    client: &reqwest::Client,
    url: &str,
    out_dir: &Path,
    project_id: &str,
) -> Result<String> {
    let versions: Vec<ModrinthVersion> = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("Failed to parse JSON response from Modrinth API")?;

    let latest_version = match versions.get(0) {
        Some(v) => v,
        None => {
            println!("Skipped: No compatible version found for project {}", project_id);
            return Ok("skipped".into());
        }
    };
    let file_to_download = latest_version
        .files
        .get(0)
        .ok_or_else(|| anyhow!("Version has no files"))?;

    let out_path = out_dir.join(&file_to_download.filename);

    if out_path.exists() {
        println!("Skipped: File already exists: {}", file_to_download.filename);
        return Ok("skipped".into());
    }

    let mut response = client
        .get(&file_to_download.url)
        .send()
        .await?
        .error_for_status()?;

    let mut dest = fs::File::create(&out_path)
        .with_context(|| format!("Failed to create output file: {}", out_path.display()))?;

    while let Some(chunk) = response.chunk().await? {
        std::io::copy(&mut chunk.as_ref(), &mut dest)?;
    }

    Ok(file_to_download.filename.clone())
}

