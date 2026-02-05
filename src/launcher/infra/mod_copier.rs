use anyhow::{Context, Result};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use zip::ZipArchive;

pub struct ModCopier;

#[derive(Debug, Clone)]
pub struct ModCopyProgress {
    pub current: usize,
    pub total: usize,
    pub mod_name: String,
}

#[derive(Debug, Clone)]
pub struct ModValidationSpec {
    pub name: String,
    pub allow_incompatible: bool,
}

impl ModCopier {
    /// Copy mods and emit progress updates over a channel
    pub async fn copy_mods_to_minecraft_with_progress(
        source_dir: &Path,
        minecraft_mods_dir: &Path,
        mod_names: &[String],
        progress_tx: Option<mpsc::Sender<ModCopyProgress>>,
    ) -> Result<Vec<String>> {
        // Ensure destination directory exists
        tokio::fs::create_dir_all(minecraft_mods_dir)
            .await
            .context("Failed to create mods directory")?;

        let mut copied_mods = Vec::new();
        let total = mod_names.len();

        for (index, mod_name) in mod_names.iter().enumerate() {
            // Find the mod file in source directory
            if let Some(mod_file) = Self::find_mod_file(source_dir, mod_name).await? {
                let file_name = mod_file
                    .file_name()
                    .context("Invalid mod filename")?
                    .to_string_lossy()
                    .to_string();

                let metadata = tokio::fs::metadata(&mod_file)
                    .await
                    .context("Failed to get file metadata")?;

                if metadata.len() == 0 {
                    log::warn!("Skipping empty mod file: {}", file_name);
                    if let Some(tx) = progress_tx.as_ref() {
                        let _ = tx
                            .send(ModCopyProgress {
                                current: index + 1,
                                total,
                                mod_name: mod_name.clone(),
                            })
                            .await;
                    }
                    continue;
                }

                let dest_path = minecraft_mods_dir.join(&file_name);

                let same_file = match (
                    tokio::fs::canonicalize(&mod_file).await,
                    tokio::fs::canonicalize(&dest_path).await,
                ) {
                    (Ok(a), Ok(b)) => a == b,
                    _ => mod_file == dest_path,
                };

                if same_file {
                    log::info!("Mod already in place: {}", dest_path.display());
                    copied_mods.push(file_name);
                } else {
                    // Copy the file
                    match tokio::fs::copy(&mod_file, &dest_path).await {
                        Ok(_) => {
                            log::info!("Copied mod: {} to {}", file_name, dest_path.display());
                            copied_mods.push(file_name);
                        }
                        Err(e) => {
                            log::warn!("Failed to copy mod {}: {}", file_name, e);
                        }
                    }
                }
            } else {
                log::warn!("Mod file not found for: {}", mod_name);
            }

            if let Some(tx) = progress_tx.as_ref() {
                let _ = tx
                    .send(ModCopyProgress {
                        current: index + 1,
                        total,
                        mod_name: mod_name.clone(),
                    })
                    .await;
            }
        }

        Ok(copied_mods)
    }

    /// Find mod file in directory by mod name
    /// Looks for files with the mod name (ignoring spaces and case)
    async fn find_mod_file(dir: &Path, mod_name: &str) -> Result<Option<PathBuf>> {
        if !dir.exists() {
            return Ok(None);
        }

        let normalized_name: String = mod_name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        if normalized_name.is_empty() {
            return Ok(None);
        }

        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy().to_lowercase();

                    // Check if filename contains the mod name and ends with .jar
                    if file_name_str.ends_with(".jar") {
                        let normalized_file: String = file_name_str
                            .chars()
                            .filter(|c| c.is_ascii_alphanumeric())
                            .map(|c| c.to_ascii_lowercase())
                            .collect();
                        if normalized_file.contains(&normalized_name) {
                            return Ok(Some(path));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn find_mod_file_sync(dir: &Path, mod_name: &str) -> Result<Option<PathBuf>> {
        if !dir.exists() {
            return Ok(None);
        }

        let normalized_name: String = mod_name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        if normalized_name.is_empty() {
            return Ok(None);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy().to_lowercase();
                    if file_name_str.ends_with(".jar") {
                        let normalized_file: String = file_name_str
                            .chars()
                            .filter(|c| c.is_ascii_alphanumeric())
                            .map(|c| c.to_ascii_lowercase())
                            .collect();
                        if normalized_file.contains(&normalized_name) {
                            return Ok(Some(path));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn read_fabric_mod_json(mod_path: &Path) -> Result<Value> {
        let file = std::fs::File::open(mod_path)
            .with_context(|| format!("Failed to open {}", mod_path.display()))?;
        let mut archive =
            ZipArchive::new(file).context("Failed to read mod jar as zip")?;
        let mut entry = archive
            .by_name("fabric.mod.json")
            .context("Missing fabric.mod.json (not a Fabric mod)")?;

        let mut contents = String::new();
        entry
            .read_to_string(&mut contents)
            .context("Failed to read fabric.mod.json")?;
        let json: Value =
            serde_json::from_str(&contents).context("Failed to parse fabric.mod.json")?;
        Ok(json)
    }

    fn get_dependency(meta: &Value, key: &str) -> Option<String> {
        let depends = meta.get("depends")?.as_object()?;
        let value = depends.get(key)?;
        if let Some(s) = value.as_str() {
            return Some(s.to_string());
        }
        if let Some(arr) = value.as_array() {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(s) = item.as_str() {
                    parts.push(s.to_string());
                }
            }
            if !parts.is_empty() {
                return Some(parts.join(" "));
            }
        }
        None
    }

    fn matches_version_requirement(version: &str, requirement: &str) -> Option<bool> {
        let version_nums = Self::parse_numeric_version(version);
        let mut unknown = false;

        for group in requirement.split("||") {
            let mut group_ok = true;
            for token in group
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|t| !t.is_empty())
            {
                match Self::eval_requirement_token(version, &version_nums, token) {
                    Some(true) => {}
                    Some(false) => {
                        group_ok = false;
                        break;
                    }
                    None => {
                        unknown = true;
                        group_ok = false;
                        break;
                    }
                }
            }
            if group_ok {
                return Some(true);
            }
        }

        if unknown {
            None
        } else {
            Some(false)
        }
    }

    fn eval_requirement_token(
        version: &str,
        version_nums: &Option<Vec<i32>>,
        token: &str,
    ) -> Option<bool> {
        let trimmed = token.trim();
        if trimmed == "*" {
            return Some(true);
        }

        let (op, rest) = if let Some(rest) = trimmed.strip_prefix(">=") {
            (">=", rest)
        } else if let Some(rest) = trimmed.strip_prefix("<=") {
            ("<=", rest)
        } else if let Some(rest) = trimmed.strip_prefix(">") {
            (">", rest)
        } else if let Some(rest) = trimmed.strip_prefix("<") {
            ("<", rest)
        } else if let Some(rest) = trimmed.strip_prefix("^") {
            ("^", rest)
        } else if let Some(rest) = trimmed.strip_prefix("~") {
            ("~", rest)
        } else {
            ("=", trimmed)
        };

        if rest.contains('x') || rest.contains('*') {
            let prefix = rest.replace('x', "").replace('*', "");
            let prefix_nums = Self::parse_numeric_version(prefix.trim_end_matches('.'))?;
            let version_nums = version_nums.as_ref()?;
            return Some(Self::starts_with_version(version_nums, &prefix_nums));
        }

        let target_nums = Self::parse_numeric_version(rest)?;
        let version_nums = version_nums.as_ref()?;
        let cmp = Self::compare_versions(version_nums, &target_nums);

        match op {
            ">=" => Some(cmp >= 0),
            ">" => Some(cmp > 0),
            "<=" => Some(cmp <= 0),
            "<" => Some(cmp < 0),
            "^" | "~" | "=" => {
                if rest == version {
                    Some(true)
                } else {
                    Some(Self::starts_with_version(version_nums, &target_nums))
                }
            }
            _ => None,
        }
    }

    fn parse_numeric_version(version: &str) -> Option<Vec<i32>> {
        let mut parts = Vec::new();
        for part in version.split('.') {
            if part.is_empty() {
                continue;
            }
            let mut digits = String::new();
            for ch in part.chars() {
                if ch.is_ascii_digit() {
                    digits.push(ch);
                } else {
                    break;
                }
            }
            if digits.is_empty() {
                return None;
            }
            parts.push(digits.parse::<i32>().ok()?);
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts)
        }
    }

    fn compare_versions(a: &[i32], b: &[i32]) -> i32 {
        let max_len = a.len().max(b.len());
        for i in 0..max_len {
            let av = *a.get(i).unwrap_or(&0);
            let bv = *b.get(i).unwrap_or(&0);
            if av != bv {
                return av.cmp(&bv) as i32;
            }
        }
        0
    }

    fn starts_with_version(full: &[i32], prefix: &[i32]) -> bool {
        if prefix.len() > full.len() {
            return false;
        }
        full.iter().take(prefix.len()).eq(prefix.iter())
    }

    /// Clear all mods from Minecraft mods directory
    pub async fn clear_mods_directory(minecraft_mods_dir: &Path) -> Result<usize> {
        if !minecraft_mods_dir.exists() {
            return Ok(0);
        }

        let mut removed_count = 0;
        let mut entries = tokio::fs::read_dir(minecraft_mods_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "jar" {
                        match tokio::fs::remove_file(&path).await {
                            Ok(_) => {
                                removed_count += 1;
                                log::info!("Removed mod: {}", path.display());
                            }
                            Err(e) => {
                                log::warn!("Failed to remove {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }

        Ok(removed_count)
    }

    pub fn validate_mods_for_launch(
        mods_dir: &Path,
        mods: &[ModValidationSpec],
        loader: &str,
        mc_version: &str,
        loader_version: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut errors = Vec::new();
        let require_fabric = loader.eq_ignore_ascii_case("fabric");

        for spec in mods {
            let mod_file = match Self::find_mod_file_sync(mods_dir, &spec.name)? {
                Some(path) => path,
                None => continue,
            };

            let metadata = std::fs::metadata(&mod_file)
                .with_context(|| format!("Failed to read {}", mod_file.display()))?;
            if metadata.len() == 0 {
                errors.push(format!("Mod file is empty: {}", mod_file.display()));
                continue;
            }

            if require_fabric {
                let fabric_meta = match Self::read_fabric_mod_json(&mod_file) {
                    Ok(meta) => meta,
                    Err(e) => {
                        if !spec.allow_incompatible {
                            errors.push(format!("{}: {}", spec.name, e));
                        }
                        continue;
                    }
                };

                if !spec.allow_incompatible {
                    if let Some(dep) = Self::get_dependency(&fabric_meta, "minecraft") {
                        if let Some(result) = Self::matches_version_requirement(mc_version, &dep) {
                            if !result {
                                errors.push(format!(
                                    "{} requires minecraft {} (current {})",
                                    spec.name, dep, mc_version
                                ));
                            }
                        } else {
                            errors.push(format!(
                                "{} has unverified minecraft requirement {}",
                                spec.name, dep
                            ));
                        }
                    }

                    if let (Some(loader_version), Some(dep)) =
                        (loader_version, Self::get_dependency(&fabric_meta, "fabricloader"))
                    {
                        if let Some(result) = Self::matches_version_requirement(loader_version, &dep)
                        {
                            if !result {
                                errors.push(format!(
                                    "{} requires fabricloader {} (current {})",
                                    spec.name, dep, loader_version
                                ));
                            }
                        } else {
                            errors.push(format!(
                                "{} has unverified fabricloader requirement {}",
                                spec.name, dep
                            ));
                        }
                    }
                }
            }
        }

        Ok(errors)
    }
}
