use crate::models::DisplayConfiguration;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tracing::info;

pub struct ConfigPersistence;

impl ConfigPersistence {
    pub fn get_monitors_conf_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not find user config directory")?;
        let hypr_dir = config_dir.join("hypr");
        if !hypr_dir.exists() {
            fs::create_dir_all(&hypr_dir)
                .with_context(|| format!("Failed to create directory {:?}", hypr_dir))?;
        }
        Ok(hypr_dir.join("monitors.conf"))
    }

    /// Atomically writes the display configuration to `~/.config/hypr/monitors.conf`
    pub fn save(config: &DisplayConfiguration) -> Result<PathBuf> {
        let path = Self::get_monitors_conf_path()?;
        let temp_path = path.with_extension("conf.tmp");

        let content = config.serialize_hyprland_conf();
        fs::write(&temp_path, content.as_bytes())
            .with_context(|| format!("Failed to write temporary file {:?}", temp_path))?;

        fs::rename(&temp_path, &path)
            .with_context(|| format!("Failed to atomically rename {:?} to {:?}", temp_path, path))?;

        info!("Persisted display configuration to {:?}", path);
        Ok(path)
    }
}
