use crate::resource_downloader::domain::ResourceType;
use lazy_static::lazy_static;
use strsim::levenshtein;

pub fn is_for_other_type(
    query: String,
    current_resource_type: &ResourceType,
) -> Option<(ResourceType, String)> {
    let normalized_query = normalize(query);

    if normalized_query.is_empty() {
        return None;
    }

    let mut potential_matches: Vec<&SupplyItemDefinition> = Vec::new();

    for def in ALL_SUPPLY_ITEMS.iter() {
        if def.matches_query(&normalized_query) {
            potential_matches.push(def);
        }
    }

    for matched_def in potential_matches {
        if matched_def
            .primary_resource_types
            .contains(current_resource_type)
        {
            continue;
        }

        if let Some(&suggested_rt) = matched_def.primary_resource_types.first() {
            return Some((suggested_rt, matched_def.main_name.clone()));
        }
    }

    None
}

lazy_static! {
    static ref ALL_SUPPLY_ITEMS: Vec<SupplyItemDefinition> = {
        vec![
            // --- Mods ---
            SupplyItemDefinition::new("Sodium", vec!["optifine alternative", "sodium mod", "sodium extra"], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("OptiFine", vec!["optifabric", "optiforge"], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("Fabric API", vec!["fabric", "fabric loader api"], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("Iris", vec!["iris shaders"], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("Mod Menu", vec![], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("Xaero's Minimap", vec![], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("Xaero's World Map", vec![], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("Entity Texture Features", vec!["etf"], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("Entity Model Features", vec!["emf"], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("Not Enough Animations", vec![], vec![ResourceType::Mod]),
            SupplyItemDefinition::new("Simple Voice Chat", vec![], vec![ResourceType::Mod, ResourceType::Plugin]),

            // --- Resource Packs ---
            SupplyItemDefinition::new("Faithful", vec!["faithful texture pack"], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Fresh Animations", vec!["fresh anims"], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Translations for Sodium", vec!["sodium translations"], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Dramatic Skys", vec!["Dramatic Skys texture pack"], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Fast Better Grass", vec![], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Better Leaves", vec!["Better Leaves texture pack"], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Fresh Moves", vec![], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Low Fire", vec![], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Default HD", vec![], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Fullbright UB", vec!["fullbright"], vec![ResourceType::ResourcePack]),
            SupplyItemDefinition::new("Barebones", vec![], vec![ResourceType::ResourcePack]),

            // --- Shaders ---
            SupplyItemDefinition::new("Complementary", vec!["complementary shaders"], vec![ResourceType::Shader]),
            SupplyItemDefinition::new("BSL", vec!["bsl shaders"], vec![ResourceType::Shader]),
            SupplyItemDefinition::new("bliss", vec!["bliss shaders"], vec![ResourceType::Shader]),
            SupplyItemDefinition::new("Photon", vec!["photon shaders"], vec![ResourceType::Shader]),
            SupplyItemDefinition::new("Solas", vec!["solas shaders"], vec![ResourceType::Shader]),
            SupplyItemDefinition::new("Rethinking Voxels", vec!["voxels"], vec![ResourceType::Shader]),

            // --- Datapacks ---
            SupplyItemDefinition::new("Veinminer", vec!["Veinminer Enchantment"], vec![ResourceType::Datapack, ResourceType::Mod, ResourceType::Plugin]),
            SupplyItemDefinition::new("Terralith", vec![], vec![ResourceType::Datapack, ResourceType::Mod]),
            SupplyItemDefinition::new("Dungeons and Taverns", vec![], vec![ResourceType::Datapack, ResourceType::Mod]),
            SupplyItemDefinition::new("Tectonic", vec![], vec![ResourceType::Datapack, ResourceType::Mod]),
            SupplyItemDefinition::new("Geophilic", vec![], vec![ResourceType::Datapack, ResourceType::Mod]),
            SupplyItemDefinition::new("Incendium", vec![], vec![ResourceType::Datapack, ResourceType::Mod]),
            SupplyItemDefinition::new("Explorify", vec![], vec![ResourceType::Datapack, ResourceType::Mod]),
            SupplyItemDefinition::new("Spawn Animations", vec![], vec![ResourceType::Datapack, ResourceType::Mod]),
            SupplyItemDefinition::new("Dynamic Lights", vec![], vec![ResourceType::Datapack, ResourceType::Mod]),

            // --- Plugins ---
            SupplyItemDefinition::new("Timer Plugin", vec![], vec![ResourceType::Plugin]),
            SupplyItemDefinition::new("CustomDeaths", vec![], vec![ResourceType::Plugin]),
            SupplyItemDefinition::new("Essential", vec![], vec![ResourceType::Plugin]),
            SupplyItemDefinition::new("Multiplayer Gameplay", vec![], vec![ResourceType::Plugin]),
            SupplyItemDefinition::new("Image Previewer", vec![], vec![ResourceType::Plugin]),
            SupplyItemDefinition::new("Worldedit", vec![], vec![ResourceType::Plugin, ResourceType::Mod]),
            SupplyItemDefinition::new("worldguard", vec![], vec![ResourceType::Plugin]),
            SupplyItemDefinition::new("viaversion", vec!["via backwards", "via rewind"], vec![ResourceType::Plugin, ResourceType::Mod]),
        ]
    };
}

#[derive(Debug, Clone)]
struct SupplyItemDefinition {
    main_name: String,
    normalized_main_name: String,
    normalized_synonyms: Vec<String>,
    primary_resource_types: Vec<ResourceType>,
}

impl SupplyItemDefinition {
    fn new(main_name: &str, synonyms: Vec<&str>, primary_types: Vec<ResourceType>) -> Self {
        let normalized_main_name = normalize(main_name.to_string());
        let normalized_synonyms = synonyms
            .clone()
            .into_iter()
            .map(|s| normalize(s.to_string()))
            .collect();

        SupplyItemDefinition {
            main_name: main_name.to_string(),
            normalized_main_name,
            normalized_synonyms,
            primary_resource_types: primary_types,
        }
    }

    fn matches_query(&self, normalized_query: &str) -> bool {
        if is_approximately_similar(normalized_query, &self.normalized_main_name) {
            return true;
        }
        for normalized_synonym in &self.normalized_synonyms {
            if is_approximately_similar(normalized_query, normalized_synonym) {
                return true;
            }
        }
        false
    }
}

fn is_approximately_similar(s1: &str, s2: &str) -> bool {
    let len1 = s1.len();
    let len2 = s2.len();

    if len1 == 0 || len2 == 0 {
        return false;
    }

    let (shorter, longer) = if len1 <= len2 { (s1, s2) } else { (s2, s1) };
    let shorter_len = shorter.len();
    let longer_len = longer.len();

    if shorter == longer {
        return true;
    }

    const MIN_CONFIDENT_SUBSTRING_LENGTH: usize = 3;
    const MIN_SUBSTRING_LENGTH_RATIO: f32 = 0.5;

    if shorter_len >= MIN_CONFIDENT_SUBSTRING_LENGTH
        && longer.contains(shorter)
        && (shorter_len as f32 / longer_len as f32) >= MIN_SUBSTRING_LENGTH_RATIO
    {
        return true;
    }

    let max_allowed_distance = match longer_len {
        0..=3 => 0,
        4..=6 => 2,
        7..=10 => 3,
        _ => 4,
    };

    let distance = levenshtein(s1, s2);
    if distance <= max_allowed_distance {
        let relative_distance_threshold = (longer_len as f32 * 0.25).ceil() as usize;
        if distance <= relative_distance_threshold {
            return true;
        }
    }

    false
}

fn normalize(query: String) -> String {
    query
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}
