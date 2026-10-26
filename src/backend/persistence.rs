use crate::backend::HyprctlClient;
use crate::models::DisplayConfiguration;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tracing::info;

pub struct ConfigPersistence;

impl ConfigPersistence {
    pub fn get_hypr_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not find user config directory")?;
        let hypr_dir = config_dir.join("hypr");
        if !hypr_dir.exists() {
            fs::create_dir_all(&hypr_dir)
                .with_context(|| format!("Failed to create directory {:?}", hypr_dir))?;
        }
        Ok(hypr_dir)
    }

    /// Atomically writes the display configuration to `monitors.lua` and/or `monitors.conf`
    pub fn save(config: &DisplayConfiguration) -> Result<PathBuf> {
        let hypr_dir = Self::get_hypr_dir()?;
        let is_lua = HyprctlClient::is_lua_parser();

        // 1. If using Lua parser, write ~/.config/hypr/monitors.lua
        if is_lua || hypr_dir.join("monitors.lua").exists() || hypr_dir.join("hyprland.lua").exists() {
            let lua_path = hypr_dir.join("monitors.lua");
            let temp_lua = lua_path.with_extension("lua.tmp");
            let lua_content = config.serialize_hyprland_lua();

            fs::write(&temp_lua, lua_content.as_bytes())
                .with_context(|| format!("Failed to write {:?}", temp_lua))?;
            fs::rename(&temp_lua, &lua_path)
                .with_context(|| format!("Failed to rename {:?} to {:?}", temp_lua, lua_path))?;

            info!("Persisted Lua display configuration to {:?}", lua_path);
        }

        // 2. Also write ~/.config/hypr/monitors.conf for legacy parser & compatibility
        let conf_path = hypr_dir.join("monitors.conf");
        let temp_conf = conf_path.with_extension("conf.tmp");
        let conf_content = config.serialize_hyprland_conf();

        fs::write(&temp_conf, conf_content.as_bytes())
            .with_context(|| format!("Failed to write {:?}", temp_conf))?;
        fs::rename(&temp_conf, &conf_path)
            .with_context(|| format!("Failed to rename {:?} to {:?}", temp_conf, conf_path))?;

        info!("Persisted display configuration to {:?}", conf_path);

        // 3. Trigger reload so changes are refreshed
        let _ = HyprctlClient::reload();

        Ok(if is_lua { hypr_dir.join("monitors.lua") } else { conf_path })
    }
}
