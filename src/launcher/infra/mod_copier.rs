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
    pub fn new() -> Self {
        Self
    }

    /// Copy mods from source directory to Minecraft mods folder
    /// Returns list of successfully copied mod filenames
    pub async fn copy_mods_to_minecraft(
        source_dir: &Path,
        minecraft_mods_dir: &Path,
        mod_names: &[String],
    ) -> Result<Vec<String>> {
        Self::copy_mods_to_minecraft_with_progress(source_dir, minecraft_mods_dir, mod_names, None)
            .await
    }

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

    /// Backup existing mods directory
    pub async fn backup_mods_directory(minecraft_mods_dir: &Path) -> Result<PathBuf> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let parent = minecraft_mods_dir.parent().context("No parent directory")?;

        let backup_dir = parent.join(format!("mods_backup_{}", timestamp));

        if minecraft_mods_dir.exists() {
            Self::copy_directory(minecraft_mods_dir.to_path_buf(), backup_dir.clone()).await?;
            log::info!("Backed up mods to: {}", backup_dir.display());
        }

        Ok(backup_dir)
    }

    /// Recursively copy a directory
    fn copy_directory(
        src: PathBuf,
        dst: PathBuf,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            tokio::fs::create_dir_all(&dst).await?;

            let mut entries = tokio::fs::read_dir(&src).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let dest_path = dst.join(entry.file_name());

                if path.is_dir() {
                    Self::copy_directory(path, dest_path).await?;
                } else {
                    tokio::fs::copy(&path, &dest_path).await?;
                }
            }

            Ok(())
        })
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

#[cfg(test)]
mod tests {
    use super::{ModCopier, ModValidationSpec};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::sync::mpsc;

    fn test_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "mmd_mod_copier_{}_{}_{}",
            name,
            std::process::id(),
            nonce
        ));
        std::fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    #[test]
    fn parse_numeric_version_handles_common_values() {
        assert_eq!(
            ModCopier::parse_numeric_version("1.20.5"),
            Some(vec![1, 20, 5])
        );
        assert_eq!(
            ModCopier::parse_numeric_version("1.20.5-beta"),
            Some(vec![1, 20, 5])
        );
        assert_eq!(ModCopier::parse_numeric_version("abc"), None);
    }

    #[test]
    fn compare_versions_handles_different_lengths() {
        assert_eq!(ModCopier::compare_versions(&[1, 20], &[1, 20, 0]), 0);
        assert!(ModCopier::compare_versions(&[1, 20, 1], &[1, 20]) > 0);
        assert!(ModCopier::compare_versions(&[1, 19], &[1, 20]) < 0);
    }

    #[test]
    fn starts_with_version_checks_prefix() {
        assert!(ModCopier::starts_with_version(&[1, 20, 1], &[1, 20]));
        assert!(!ModCopier::starts_with_version(&[1, 20], &[1, 20, 1]));
    }

    #[test]
    fn eval_requirement_token_supports_operators_and_wildcards() {
        let nums = ModCopier::parse_numeric_version("1.20.1");
        assert_eq!(
            ModCopier::eval_requirement_token("1.20.1", &nums, ">=1.20"),
            Some(true)
        );
        assert_eq!(
            ModCopier::eval_requirement_token("1.20.1", &nums, "<1.20"),
            Some(false)
        );
        assert_eq!(
            ModCopier::eval_requirement_token("1.20.1", &nums, "1.20.x"),
            Some(true)
        );
        assert_eq!(
            ModCopier::eval_requirement_token("1.20.1", &nums, "*"),
            Some(true)
        );
    }

    #[test]
    fn matches_version_requirement_handles_groups() {
        assert_eq!(
            ModCopier::matches_version_requirement("1.20.1", ">=1.20 <1.21"),
            Some(true)
        );
        assert_eq!(
            ModCopier::matches_version_requirement("1.20.1", ">=1.21 || 1.20.x"),
            Some(true)
        );
        assert_eq!(
            ModCopier::matches_version_requirement("1.20.1", "<1.20"),
            Some(false)
        );
        assert_eq!(
            ModCopier::matches_version_requirement("1.20.1", "not-a-rule"),
            None
        );
    }

    #[test]
    fn get_dependency_supports_string_and_array() {
        let json = serde_json::json!({
            "depends": {
                "minecraft": ">=1.20",
                "fabricloader": [">=0.15.0", "<0.16.0"]
            }
        });

        assert_eq!(
            ModCopier::get_dependency(&json, "minecraft"),
            Some(">=1.20".to_string())
        );
        assert_eq!(
            ModCopier::get_dependency(&json, "fabricloader"),
            Some(">=0.15.0 <0.16.0".to_string())
        );
        assert_eq!(ModCopier::get_dependency(&json, "missing"), None);
    }

    #[test]
    fn find_mod_file_sync_matches_normalized_name() {
        let dir = test_temp_dir("find_sync");
        let jar = dir.join("Awesome-Mod_1.0.0.jar");
        std::fs::write(&jar, b"dummy").expect("failed to create test jar");

        let found = ModCopier::find_mod_file_sync(&dir, "awesome mod")
            .expect("find_mod_file_sync should not fail");
        assert_eq!(found.as_deref(), Some(jar.as_path()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn clear_mods_directory_removes_only_jars() {
        let dir = test_temp_dir("clear_mods");
        let jar = dir.join("a.jar");
        let txt = dir.join("notes.txt");
        std::fs::write(&jar, b"dummy").expect("failed to create jar");
        std::fs::write(&txt, b"dummy").expect("failed to create txt");

        let removed = ModCopier::clear_mods_directory(&dir)
            .await
            .expect("clear_mods_directory failed");

        assert_eq!(removed, 1);
        assert!(!jar.exists());
        assert!(txt.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_mods_for_launch_reports_empty_mod_file() {
        let mods_dir = test_temp_dir("validate_empty");
        let empty_mod = mods_dir.join("Bobby.jar");
        std::fs::write(&empty_mod, b"").expect("failed to create empty mod");

        let specs = vec![ModValidationSpec {
            name: "Bobby".to_string(),
            allow_incompatible: false,
        }];

        let errors = ModCopier::validate_mods_for_launch(
            &mods_dir,
            &specs,
            "fabric",
            "1.20.1",
            Some("0.15.0"),
        )
        .expect("validation should not fail unexpectedly");

        assert!(errors.iter().any(|e| e.contains("Mod file is empty")));

        let _ = std::fs::remove_dir_all(&mods_dir);
    }

    #[test]
    fn validate_mods_for_launch_respects_allow_incompatible() {
        let mods_dir = test_temp_dir("validate_allow");
        let mod_file = mods_dir.join("ParticleEffects.jar");
        std::fs::write(&mod_file, b"not-a-zip").expect("failed to create mod file");

        let strict_specs = vec![ModValidationSpec {
            name: "ParticleEffects".to_string(),
            allow_incompatible: false,
        }];
        let strict_errors = ModCopier::validate_mods_for_launch(
            &mods_dir,
            &strict_specs,
            "fabric",
            "1.20.1",
            Some("0.15.0"),
        )
        .expect("strict validation failed unexpectedly");
        assert!(!strict_errors.is_empty());

        let lax_specs = vec![ModValidationSpec {
            name: "ParticleEffects".to_string(),
            allow_incompatible: true,
        }];
        let lax_errors = ModCopier::validate_mods_for_launch(
            &mods_dir,
            &lax_specs,
            "fabric",
            "1.20.1",
            Some("0.15.0"),
        )
        .expect("lax validation failed unexpectedly");
        assert!(lax_errors.is_empty());

        let _ = std::fs::remove_dir_all(&mods_dir);
    }

    #[tokio::test]
    async fn copy_mods_copies_non_empty_and_reports_progress() {
        let source_dir = test_temp_dir("copy_with_progress_src");
        let dest_dir = test_temp_dir("copy_with_progress_dest");

        let bobby = source_dir.join("Bobby.jar");
        let particle = source_dir.join("ParticleEffects.jar");
        std::fs::write(&bobby, b"valid-mod").expect("failed to write bobby mod");
        std::fs::write(&particle, b"").expect("failed to write empty particle mod");

        let mod_names = vec![
            "Bobby".to_string(),
            "ParticleEffects".to_string(),
            "MissingMod".to_string(),
        ];

        let (tx, mut rx) = mpsc::channel(16);
        let copied = ModCopier::copy_mods_to_minecraft_with_progress(
            &source_dir,
            &dest_dir,
            &mod_names,
            Some(tx),
        )
        .await
        .expect("copy_mods_to_minecraft_with_progress failed");

        let mut progress_messages = Vec::new();
        while let Some(progress) = rx.recv().await {
            progress_messages.push(progress);
        }

        assert_eq!(copied, vec!["Bobby.jar".to_string()]);
        assert!(dest_dir.join("Bobby.jar").exists());
        assert!(!dest_dir.join("ParticleEffects.jar").exists());

        assert_eq!(progress_messages.len(), 3);
        assert_eq!(progress_messages[0].current, 1);
        assert_eq!(progress_messages[0].total, 3);
        assert_eq!(progress_messages[2].current, 3);

        let _ = std::fs::remove_dir_all(&source_dir);
        let _ = std::fs::remove_dir_all(&dest_dir);
    }

    #[tokio::test]
    async fn copy_mods_same_source_and_destination_keeps_file_in_place() {
        let mods_dir = test_temp_dir("copy_same_dir");
        let jar = mods_dir.join("WorldEdit.jar");
        std::fs::write(&jar, b"worldedit").expect("failed to write worldedit mod");

        let copied = ModCopier::copy_mods_to_minecraft_with_progress(
            &mods_dir,
            &mods_dir,
            &["WorldEdit".to_string()],
            None,
        )
        .await
        .expect("copy_mods_to_minecraft_with_progress failed");

        assert_eq!(copied, vec!["WorldEdit.jar".to_string()]);
        assert!(jar.exists());

        let _ = std::fs::remove_dir_all(&mods_dir);
    }
}
