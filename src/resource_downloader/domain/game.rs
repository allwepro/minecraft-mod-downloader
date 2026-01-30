use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct GameVersion {
    pub name: String,
    pub channel: String,
}

impl GameVersion {
    pub fn release(name: String) -> Self {
        Self {
            name: name.trim_end_matches(".0").to_string(),
            channel: "release".parse().unwrap(),
        }
    }

    fn get_components(&self) -> [u32; 3] {
        if !self.is_release() {
            return [0, 0, 0];
        }
        let mut parts = self.name.split('.').map(|s| s.parse::<u32>().unwrap_or(0));

        [
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        ]
    }

    pub fn as_u64(&self) -> u64 {
        let components = self.get_components();
        (components[0] as u64 * 1_000_000) + (components[1] as u64 * 1_000) + (components[2] as u64)
    }

    pub fn distance_int(&self, other: &Self) -> i64 {
        (self.as_u64() as i64) - (other.as_u64() as i64)
    }

    pub fn is_release(&self) -> bool {
        self.channel == "release"
    }
    pub fn is_snapshot(&self) -> bool {
        self.channel == "snapshot"
    }
}

impl From<&String> for GameVersion {
    fn from(v: &String) -> Self {
        let v_l = v.to_lowercase();

        let channel = if v_l.contains("pre-release") || v_l.contains("-pre") {
            "pre-release"
        } else if v_l.contains("release candidate") || v_l.contains("-rc") {
            "release-candidate"
        } else if v_l.starts_with("inf-") || v_l.contains("infdev") {
            "inf-dev"
        } else if v_l.starts_with('c')
            && v_l
                .chars()
                .nth(1)
                .is_some_and(|c| c.is_ascii_digit() || c == '.')
        {
            "classic"
        } else if v_l.starts_with("rd-") || v_l.contains("pre-classic") {
            "pre-classic"
        } else if v.chars().all(|c| c.is_ascii_digit() || c == '.') {
            "release"
        } else if v_l.contains("snapshot")
            || v.len() >= 5
                && v.contains('w')
                && v.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            "snapshot"
        } else if v_l.starts_with('b') || v_l.contains("beta") {
            "beta"
        } else if v_l.starts_with('a') || v_l.contains("alpha") {
            "alpha"
        } else if v_l.contains("experimental")
            || v_l.contains("test")
            || v_l.contains("unobfuscated")
        {
            "experimental"
        } else {
            "unknown"
        };

        Self {
            name: v.clone().trim_end_matches(".0").to_string(),
            channel: channel.to_string(),
        }
    }
}

impl PartialOrd for GameVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GameVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        if !self.is_release() {
            return Ordering::Less;
        }

        let v1_parts: [u32; 3] = self.get_components();
        let v2_parts: [u32; 3] = other.get_components();

        let iterations = std::cmp::max(v1_parts.len(), v2_parts.len());

        for i in 0..iterations {
            let part1 = v1_parts.get(i).unwrap_or(&0);
            let part2 = v2_parts.get(i).unwrap_or(&0);

            match part1.cmp(part2) {
                Ordering::Equal => continue,
                non_equal => return non_equal,
            }
        }

        Ordering::Equal
    }
}

impl Display for GameVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.channel)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GameLoader {
    pub id: String,
    pub name: String,
}

impl Display for GameLoader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
