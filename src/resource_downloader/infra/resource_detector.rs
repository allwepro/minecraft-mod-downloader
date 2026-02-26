use crate::resource_downloader::domain::{GameLoader, GameVersion, ResourceType};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub type ResourceResult = (
    Vec<(String, String, GameVersion, GameLoader)>, // (filename, cleaned_name, version, loader)
    Option<GameVersion>,
    Option<GameLoader>,
);

pub struct ResourceDetector;

impl ResourceDetector {
    pub fn detect_resources_from_dir(
        &self,
        path: PathBuf,
        rt: ResourceType,
        available_loaders: Vec<GameLoader>,
        available_versions: Vec<GameVersion>,
    ) -> ResourceResult {
        let extension = rt.file_extension();

        let entries: Vec<String> = fs::read_dir(path)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_file()
                    && path
                        .extension()
                        .is_some_and(|ext| ext == extension.as_str())
                {
                    return Some(entry.file_name().to_str().unwrap().to_string());
                }
                None
            })
            .collect();

        self.detect_resources(entries, rt, available_loaders, available_versions)
    }

    pub fn detect_resources(
        &self,
        entries: Vec<String>,
        rt: ResourceType,
        available_loaders: Vec<GameLoader>,
        available_versions: Vec<GameVersion>,
    ) -> ResourceResult {
        let mut results = Vec::new();
        let mut loader_counts: HashMap<String, usize> = HashMap::new();

        let mut sorted_gv = available_versions.clone();
        sorted_gv.sort_by(|a, b| b.name.len().cmp(&a.name.len()));

        let ext = rt.file_extension();

        let mut file_compatibilities: Vec<Vec<GameVersion>> = Vec::new();

        for raw_name in entries {
            let decoded = self.url_decode(&raw_name);
            let stem = decoded
                .strip_suffix(&format!(".{}", ext))
                .unwrap_or(&decoded)
                .to_string();

            let mut metadata_start_index = stem.len();
            let mut primary_gv: Option<GameVersion> = None;
            let mut supported_range: Vec<GameVersion> = Vec::new();

            for gv in &sorted_gv {
                if let Some(idx) = self.find_token_boundary(&stem, &gv.name) {
                    let end_idx = idx + gv.name.len() + 1;
                    let suffix = stem.get(end_idx..).unwrap_or("");

                    if suffix.starts_with('+')
                        || suffix.to_lowercase().starts_with("-plus")
                        || suffix.to_lowercase().starts_with("plus")
                    {
                        supported_range = available_versions
                            .iter()
                            .filter(|v| v.as_u64() >= gv.as_u64())
                            .cloned()
                            .collect();
                    } else if let Some(remaining) = suffix.strip_prefix('-') {
                        if let Some(end_gv) = sorted_gv.iter().find(|v| remaining.contains(&v.name))
                        {
                            supported_range = available_versions
                                .iter()
                                .filter(|v| {
                                    v.as_u64() >= gv.as_u64() && v.as_u64() <= end_gv.as_u64()
                                })
                                .cloned()
                                .collect();
                        }
                    }

                    if supported_range.is_empty() {
                        supported_range = vec![gv.clone()];
                    }

                    if idx < metadata_start_index {
                        metadata_start_index = idx;
                    }
                    primary_gv = Some(gv.clone());
                    break;
                }
            }

            let mut file_loader = None;
            for loader in &available_loaders {
                if let Some(idx) = self.find_token_boundary(&stem, &loader.id) {
                    if idx > 0 && idx < metadata_start_index {
                        metadata_start_index = idx;
                    }
                    file_loader = Some(loader.clone());
                    *loader_counts.entry(loader.id.clone()).or_insert(0) += 1;
                    break;
                }
            }

            if let Some(idx) = self.find_generic_version_start(&stem) {
                if idx < metadata_start_index {
                    metadata_start_index = idx;
                }
            }

            let name_part = &stem[..metadata_start_index];
            let cleaned_name = self.clean_project_name(name_part);

            let final_gv = primary_gv.clone().unwrap_or_else(|| GameVersion {
                name: "Unknown".to_string(),
                channel: "unknown".to_string(),
            });

            let final_loader = file_loader.unwrap_or_else(|| GameLoader {
                id: "unknown".to_string(),
                name: "Unknown".to_string(),
            });

            if !supported_range.is_empty() {
                file_compatibilities.push(supported_range);
            }

            results.push((raw_name, cleaned_name, final_gv, final_loader));
        }

        let mut global_version_scores: HashMap<String, usize> = HashMap::new();
        for range in &file_compatibilities {
            for ver in range {
                *global_version_scores.entry(ver.name.clone()).or_insert(0) += 1;
            }
        }

        let best_version = global_version_scores
            .into_iter()
            .filter(|(_, count)| *count > (results.len() / 10))
            .max_by(
                |(name_a, count_a), (name_b, count_b)| match count_a.cmp(count_b) {
                    std::cmp::Ordering::Equal => {
                        let va = available_versions
                            .iter()
                            .find(|v| &v.name == name_a)
                            .map(|v| v.as_u64())
                            .unwrap_or(0);
                        let vb = available_versions
                            .iter()
                            .find(|v| &v.name == name_b)
                            .map(|v| v.as_u64())
                            .unwrap_or(0);
                        va.cmp(&vb)
                    }
                    other => other,
                },
            )
            .and_then(|(name, _)| available_versions.iter().find(|v| v.name == name).cloned());

        let best_loader = loader_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .and_then(|(id, _)| available_loaders.iter().find(|l| l.id == id).cloned());

        (results, best_version, best_loader)
    }

    fn url_decode(&self, input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut it = input.chars();
        while let Some(c) = it.next() {
            if c == '%' {
                let mut hex = String::new();
                if let Some(h1) = it.next() {
                    hex.push(h1);
                }
                if let Some(h2) = it.next() {
                    hex.push(h2);
                }
                if let Ok(n) = u8::from_str_radix(&hex, 16) {
                    out.push(n as char);
                } else {
                    out.push('%');
                    out.push_str(&hex);
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn find_token_boundary(&self, stem: &str, token: &str) -> Option<usize> {
        let lower_stem = stem.to_lowercase();
        let lower_token = token.to_lowercase();
        if let Some(idx) = lower_stem.find(&lower_token) {
            if idx == 0 {
                return Some(0);
            }
            let prev_char = stem.chars().nth(idx - 1)?;
            if matches!(prev_char, '-' | '_' | '+' | ' ' | '[' | '(') {
                return Some(idx - 1);
            }
            if idx >= 2 && lower_stem[idx - 2..].starts_with("mc") {
                return Some(idx - 2);
            }
        }
        None
    }

    fn find_generic_version_start(&self, stem: &str) -> Option<usize> {
        let chars: Vec<char> = stem.chars().collect();
        for i in 1..chars.len() {
            let c = chars[i];
            let prev = chars[i - 1];
            if matches!(prev, '-' | '_' | '+' | ' ') {
                if c.is_ascii_digit() {
                    return Some(i - 1);
                }
                if (c == 'v' || c == 'V') && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit()) {
                    return Some(i - 1);
                }
            }
        }
        None
    }

    fn clean_project_name(&self, input: &str) -> String {
        let mut cleaned = input
            .split(['-', '_', '+', '.'])
            .filter(|s| !s.is_empty() && !s.chars().all(|c| c.is_numeric()))
            .map(|word| {
                let mut c = word.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<String>>()
            .join(" ");

        if cleaned.is_empty() {
            cleaned = input.trim().to_string();
        }

        let cleaned = cleaned.trim().to_string();

        self.apply_name_remapping(&cleaned)
    }

    fn apply_name_remapping(&self, name: &str) -> String {
        let remappings = [
            ("Iris", "Iris Shaders"),
            ("Cloth Config", "Cloth Config API"),
            ("Entity Model Features", "[EMF] Entity Model Features"),
            ("Entity Texture Features", "[ETF] Entity Texture Features"),
            ("Entitytexturefeatures", "[ETF] Entity Texture Features"),
            ("Entitymodelfeatures", "[EMF] Entity Model Features"),
            ("Emf", "[EMF] Entity Model Features"),
            ("Etf", "[ETF] Entity Texture Features"),
            ("Firstperson", "First-person Model"),
            ("Nochatreports", "No Chat Reports"),
            ("Voicechat", "Simple Voice Chat"),
            ("Worldedit Mod", "WorldEdit"),
            ("Bettermounthud", "Better Mount HUD"),
            ("Horsestatsmod", "Horse Statistics"),
            ("NBTac", "NBT Autocomplete"),
            ("Notenoughanimations", "Not Enough Animations"),
            ("Skinlayers3d", "3D Skin Layers"),
            ("Yet Another Config Lib", "YetAnotherConfigLib (YACL)"),
            ("SoundPhysicsRemastered", "Sound Physics Remastered"),
            ("Fullbrightnesstoggle", "Full Brightness Toggle"),
            ("Xaerominimap", "Xaero's Minimap"),
            ("Xaerominimap", "Xaero's Minimap"),
            ("ClothConfigAPI", "Cloth Config API"),
            // Doing a space insertion at uppercase boundaries to catch more cases,
            // but this causes some false positives that need to be remapped back
            // like "TweakerMore" cant be found with a space
            ("XaerosWorldMap", "Xaero's World Map"),
            (
                "DetailArmorBarReconstructed",
                "Detail Armor Bar Reconstructed",
            ),
            ("ForgeConfigAPIPort", "Forge Config API Port"),
            ("LibrarianTradeFinder", "Librarian Trade Finder"),
            ("PickUpNotifier", "Pick Up Notifier"),
        ];

        let name_lower = name.to_lowercase();
        for (pattern, replacement) in &remappings {
            if name_lower == pattern.to_lowercase() {
                return replacement.to_string();
            }
        }

        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_downloader::domain::{GameLoader, GameVersion, ResourceType};

    fn setup() -> (ResourceDetector, Vec<GameLoader>, Vec<GameVersion>) {
        let detector = ResourceDetector;
        let loaders = vec![
            GameLoader {
                id: "fabric".to_string(),
                name: "Fabric".to_string(),
            },
            GameLoader {
                id: "forge".to_string(),
                name: "Forge".to_string(),
            },
            GameLoader {
                id: "quilt".to_string(),
                name: "Quilt".to_string(),
            },
        ];
        let versions = vec![
            GameVersion::release("1.21.1".to_string()),
            GameVersion::release("1.20.1".to_string()),
            GameVersion::release("1.20".to_string()),
            GameVersion::release("1.19.4".to_string()),
            GameVersion::release("1.19.2".to_string()),
            GameVersion::release("1.18.2".to_string()),
        ];
        (detector, loaders, versions)
    }

    #[test]
    fn test_simple_detection() {
        let (detector, loaders, versions) = setup();
        let entries = vec!["MouseWheelie-fabric-1.6.4+mc1.20.1.jar".to_string()];
        let (results, _, _) =
            detector.detect_resources(entries, ResourceType::Mod, loaders, versions);

        assert_eq!(results.len(), 1);
        let (_, cleaned_name, version, loader) = &results[0];
        assert_eq!(cleaned_name, "MouseWheelie");
        assert_eq!(version.name, "1.20.1");
        assert_eq!(loader.id, "fabric");
    }

    #[test]
    fn test_version_range_plus() {
        let (detector, loaders, versions) = setup();
        let entries = vec!["Sodium-1.20.1-plus.jar".to_string()];
        let (results, _, _) =
            detector.detect_resources(entries, ResourceType::Mod, loaders, versions);

        assert_eq!(results.len(), 1);
        let (_, _, version, _) = &results[0];
        assert_eq!(version.name, "1.20.1");
    }

    #[test]
    fn test_name_remapping() {
        let (detector, loaders, versions) = setup();
        let entries = vec!["iris-mc1.20.1-1.6.9.jar".to_string()];
        let (results, _, _) =
            detector.detect_resources(entries, ResourceType::Mod, loaders, versions);

        assert_eq!(results.len(), 1);
        let (_, cleaned_name, _, _) = &results[0];
        assert_eq!(cleaned_name, "Iris Shaders");
    }

    #[test]
    fn test_url_encoded_names() {
        let (detector, loaders, versions) = setup();
        let entries = vec!["%5BEMF%5D%20Entity%20Model%20Features-1.20.1.jar".to_string()];
        let (results, _, _) =
            detector.detect_resources(entries, ResourceType::Mod, loaders, versions);

        assert_eq!(results.len(), 1);
        let (_, cleaned_name, _, _) = &results[0];
        assert_eq!(cleaned_name, "[EMF] Entity Model Features");
    }

    #[test]
    fn test_suggested_version_and_loader() {
        let (detector, loaders, versions) = setup();
        let entries = vec![
            "mod-a-fabric-1.20.1.jar".to_string(),
            "mod-b-fabric-1.20.1.jar".to_string(),
            "mod-c-forge-1.19.4.jar".to_string(),
        ];
        let (_, best_v, best_l) =
            detector.detect_resources(entries, ResourceType::Mod, loaders, versions);

        assert_eq!(best_v.unwrap().name, "1.20.1");
        assert_eq!(best_l.unwrap().id, "fabric");
    }

    #[test]
    fn test_unknown_detection() {
        let (detector, loaders, versions) = setup();
        let entries = vec!["mystery-mod-v1.jar".to_string()];
        let (results, _, _) =
            detector.detect_resources(entries, ResourceType::Mod, loaders, versions);

        assert_eq!(results.len(), 1);
        let (_, cleaned_name, version, loader) = &results[0];
        assert_eq!(cleaned_name, "Mystery Mod");
        assert_eq!(version.name, "Unknown");
        assert_eq!(loader.id, "unknown");
    }

    #[test]
    fn test_detect_resources_from_dir() {
        let (detector, loaders, versions) = setup();
        let temp_dir = std::env::temp_dir().join("flux_launcher_test_import");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let files = vec![
            "Sodium-1.20.1.jar",
            "Lithium-1.20.1.jar",
            "Indium-1.20.1.jar",
            "not-a-mod.txt",
        ];

        for file in files {
            std::fs::File::create(temp_dir.join(file)).unwrap();
        }

        let (results, best_v, _best_l) = detector.detect_resources_from_dir(
            temp_dir.clone(),
            ResourceType::Mod,
            loaders,
            versions,
        );

        assert_eq!(results.len(), 3);
        assert!(results.iter().any(|(_, name, _, _)| name == "Sodium"));
        assert!(results.iter().any(|(_, name, _, _)| name == "Lithium"));
        assert!(results.iter().any(|(_, name, _, _)| name == "Indium"));

        assert_eq!(best_v.unwrap().name, "1.20.1");

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
