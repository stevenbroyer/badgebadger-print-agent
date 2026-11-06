// The agent's own settings, kept as settings.json in the app's local data
// folder next to the pairing token. Today that's only the badge printer:
// where a badge goes when BadgeBadger doesn't name a printer. Without it the
// agent falls back to the OS default, which on a shared office PC is usually
// the paper printer.

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub badge_printer: Option<String>,
}

fn path(app: &tauri::AppHandle) -> anyhow::Result<std::path::PathBuf> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| anyhow::anyhow!("no app-local data dir: {e}"))?;
    Ok(dir.join("settings.json"))
}

/// A missing or unreadable file reads as the defaults, so a bad write can
/// never stop the agent from starting.
pub fn load(app: &tauri::AppHandle) -> Settings {
    let Ok(path) = path(app) else {
        return Settings::default();
    };
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|err| {
            tracing::warn!(?err, "settings.json is unreadable; using defaults");
            Settings::default()
        }),
        Err(_) => Settings::default(),
    }
}

/// Writes a temp file and renames it over the old one, so a crash mid-write
/// leaves the previous settings in place.
pub fn save(app: &tauri::AppHandle, settings: &Settings) -> anyhow::Result<()> {
    let path = path(app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}
