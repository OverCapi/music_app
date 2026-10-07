//! JSON persistence shared by the library and the playlists.

use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Reads `file`, falling back to the default value if it is missing or corrupt.
pub fn load_json<T: DeserializeOwned + Default>(file: &Path) -> T {
    let Ok(json) = fs::read_to_string(file) else {
        return T::default();
    };
    serde_json::from_str(&json).unwrap_or_else(|e| {
        eprintln!("storage: ignoring corrupt {}: {e}", file.display());
        T::default()
    })
}

/// Writes to a temp file then renames, so a crash never leaves a half-written file.
pub fn save_json<T: Serialize + ?Sized>(file: &Path, value: &T) -> Result<(), String> {
    let err = |e: std::io::Error| format!("Impossible d'enregistrer {} : {e}", file.display());
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(err)?;
    }
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(err)?;
    fs::rename(&tmp, file).map_err(err)
}
