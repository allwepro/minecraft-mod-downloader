use anyhow::{anyhow, Result};
use std::collections::HashMap;

/*
optional utils
    error helpers
    reusable async helpers
    file path helpers
    config loading
 */

// A simple value in INI can be a String or a nested table
#[derive(Debug, Clone)]
enum IniValue {
    String(String),
    Table(HashMap<String, IniValue>),
}

pub fn parse_ini(content: &str) -> Result<HashMap<String, IniValue>> {
    let mut root = HashMap::new();
    let mut current_section_path: Vec<String> = Vec::new();

    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        // Handle [section.subsection]
        if line.starts_with('[') && line.ends_with(']') {
            current_section_path = line[1..line.len() - 1].split('.').map(String::from).collect();
            continue;
        }

        // Find the current table to insert into
        let mut current_table = &mut root;
        for part in &current_section_path {
            current_table = match current_table
                .entry(part.clone())
                .or_insert_with(|| IniValue::Table(HashMap::new()))
            {
                IniValue::Table(table) => table,
                _ => return Err(anyhow!("Config format error: Expected a table for '{}'", part)),
            };
        }

        // Handle key = value or just key
        let parts: Vec<&str> = line.splitn(2, '=').map(str::trim).collect();
        let key = parts[0].to_string();
        let value = parts.get(1).map(|s| s.to_string()).unwrap_or_else(|| key.clone());

        current_table.insert(key, IniValue::String(value));
    }

    Ok(root)
}

fn get_ini_str<'a>(table: &'a HashMap<String, IniValue>, path: &[&str]) -> Option<&'a str> {
    let mut current = table;
    for (i, key) in path.iter().enumerate() {
        match current.get(*key) {
            Some(IniValue::Table(t)) if i < path.len() - 1 => current = t,
            Some(IniValue::String(s)) if i == path.len() - 1 => return Some(s),
            _ => return None,
        }
    }
    None
}