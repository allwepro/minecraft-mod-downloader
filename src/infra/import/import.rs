use std::fs;
use std::path::Path;

/// Importiert Mod-IDs aus einer TXT-Datei.
///
/// Regeln:
/// - Eine Mod-ID pro Zeile
/// - Leere Zeilen werden ignoriert
/// - Zeilen mit `#` sind Kommentare
///
/// Beispiel:
/// ```txt
/// # Performance
/// sodium
/// lithium
/// fabric-api
/// ```
pub fn import_mod_ids<P: AsRef<Path>>(path: P) -> Result<Vec<String>, ImportError> {
    let content = fs::read_to_string(path).map_err(ImportError::Io)?;

    let mod_ids: Vec<String> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#'))
        .map(String::from)
        .collect();

    if mod_ids.is_empty() {
        return Err(ImportError::EmptyFile);
    }

    Ok(mod_ids)
}

/// Fehler, die beim Import auftreten können
#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    EmptyFile,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Io(err) => write!(f, "Dateifehler: {}", err),
            ImportError::EmptyFile => write!(f, "Keine Mod-IDs in der Datei gefunden"),
        }
    }
}

impl std::error::Error for ImportError {}
