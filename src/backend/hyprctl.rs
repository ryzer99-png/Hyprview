use crate::models::{Monitor, RawHyprlandMonitor};
use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info, warn};

pub struct HyprctlClient;

impl HyprctlClient {
    /// Detects if current Hyprland session uses the new Lua parser
    pub fn is_lua_parser() -> bool {
        // Check if hyprland.lua exists in user config
        if let Some(cfg_dir) = dirs::config_dir() {
            if cfg_dir.join("hypr").join("hyprland.lua").exists() {
                return true;
            }
        }

        // Check if hyprctl eval works
        if let Ok(out) = Command::new("hyprctl").args(["eval", "return 1"]).output() {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.contains("ok") || stdout.contains('1') {
                    return true;
                }
            }
        }

        false
    }

    /// Queries Hyprland for all connected monitors (both active and disabled)
    pub fn get_monitors() -> Result<Vec<Monitor>> {
        let output = Command::new("hyprctl")
            .args(["-j", "monitors", "all"])
            .output()
            .context("Failed to execute `hyprctl -j monitors all`. Is Hyprland running?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`hyprctl monitors all` failed: {}", stderr);
        }

        let raw_monitors: Vec<RawHyprlandMonitor> = serde_json::from_slice(&output.stdout)
            .context("Failed to deserialize hyprctl monitors JSON output")?;

        let monitors = raw_monitors.into_iter().map(Monitor::from).collect();
        Ok(monitors)
    }

    /// Applies monitor configuration in real-time, automatically handling Lua and legacy syntax
    pub fn apply_monitors(monitors: &[Monitor]) -> Result<()> {
        let is_lua = Self::is_lua_parser();
        info!("Applying {} monitor configuration(s) (Lua mode: {})", monitors.len(), is_lua);

        for mon in monitors {
            if is_lua {
                let lua_cmd = mon.to_lua_directive();
                debug!("Executing: hyprctl eval '{}'", lua_cmd);

                let output = Command::new("hyprctl")
                    .args(["eval", &lua_cmd])
                    .output()
                    .with_context(|| format!("Failed to run `hyprctl eval '{}'`", lua_cmd))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("`hyprctl eval` error: {}", stderr);
                }
            } else {
                let directive = mon.to_hyprland_directive();
                if let Some(stripped) = directive.strip_prefix("monitor = ") {
                    debug!("Executing: hyprctl keyword monitor '{}'", stripped);
                    let output = Command::new("hyprctl")
                        .args(["keyword", "monitor", stripped])
                        .output()
                        .with_context(|| format!("Failed to run `hyprctl keyword monitor '{}'`", stripped))?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        warn!("`hyprctl keyword` error: {}", stderr);
                    }
                }
            }
        }

        Ok(())
    }

    /// Forces Hyprland to reload its display configuration
    pub fn reload() -> Result<()> {
        info!("Reloading Hyprland configuration...");
        let _ = Command::new("hyprctl").arg("reload").output();
        Ok(())
    }

    /// Emergency rescue: enables internal screen with preferred resolution
    pub fn emergency_enable_internal(internal_name: &str) -> Result<()> {
        info!("Executing emergency display recovery for {}", internal_name);

        if Self::is_lua_parser() {
            let lua = format!(
                r#"hl.monitor({{ output = "{}", mode = "preferred", position = "auto", scale = 1.0 }})"#,
                internal_name
            );
            let _ = Command::new("hyprctl").args(["eval", &lua]).output();
        } else {
            let _ = Command::new("hyprctl")
                .args(["keyword", "monitor", &format!("{},preferred,auto,1", internal_name)])
                .output();
        }

        Ok(())
    }
}
