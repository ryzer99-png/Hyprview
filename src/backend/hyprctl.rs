use crate::models::{Monitor, RawHyprlandMonitor};
use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info};

pub struct HyprctlClient;

impl HyprctlClient {
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

    /// Applies a batch of `keyword monitor ...` directives to Hyprland
    pub fn apply_directives(commands: &[String]) -> Result<()> {
        for cmd in commands {
            debug!("Applying hyprctl command: {}", cmd);
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let output = Command::new("hyprctl")
                .args(&parts)
                .output()
                .with_context(|| format!("Failed to run `hyprctl {}`", cmd))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Command `hyprctl {}` failed: {}", cmd, stderr);
            }
        }
        info!("Successfully applied {} monitor directives", commands.len());
        Ok(())
    }

    /// Emergency rescue: enables internal screen with preferred resolution
    pub fn emergency_enable_internal(internal_name: &str) -> Result<()> {
        info!("Executing emergency display recovery for {}", internal_name);
        let output = Command::new("hyprctl")
            .args(["keyword", "monitor", &format!("{},preferred,auto,1", internal_name)])
            .output()
            .context("Failed to run emergency recovery hyprctl command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Emergency recovery failed: {}", stderr);
        }
        Ok(())
    }
}
