use crate::backend::{ConfigPersistence, HyprctlClient, HyprlandSocket};
use crate::models::DisplayConfiguration;
use anyhow::Result;
use std::process::Command;
use tokio::io::AsyncBufReadExt;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

pub struct RescueDaemon;

impl RescueDaemon {
    pub async fn run() -> Result<()> {
        info!("Starting Hyprland display rescue daemon...");

        // 1. Immediate evaluation upon daemon launch (crucial for cold boot with no external screen)
        if let Err(e) = Self::evaluate_rescue().await {
            error!("Error during startup rescue evaluation: {:#}", e);
        }

        // 2. Loop and stream compositor events from socket2
        loop {
            match HyprlandSocket::connect().await {
                Ok(mut reader) => {
                    info!("Rescue daemon actively listening for Hyprland events");

                    // Re-evaluate whenever connected/reconnected
                    let _ = Self::evaluate_rescue().await;

                    let mut line = String::new();
                    loop {
                        line.clear();
                        match reader.read_line(&mut line).await {
                            Ok(0) => {
                                warn!("Hyprland socket closed, attempting reconnect in 2s...");
                                break;
                            }
                            Ok(_) => {
                                let event = line.trim();
                                if event.starts_with("monitorremoved>>") {
                                    info!("Received monitor removal event: {}", event);
                                    // Small delay to allow compositor state to settle
                                    sleep(Duration::from_millis(150)).await;
                                    if let Err(e) = Self::evaluate_rescue().await {
                                        error!("Error during rescue evaluation: {:#}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error reading from socket: {:#}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to connect to Hyprland socket2: {:#}. Retrying in 3s...", e);
                }
            }

            sleep(Duration::from_secs(3)).await;
        }
    }

    /// Evaluates whether an emergency display recovery is needed.
    /// Returns Ok(true) if rescue was triggered and executed.
    pub async fn evaluate_rescue() -> Result<bool> {
        let monitors = HyprctlClient::get_monitors()?;

        let internal_monitors: Vec<_> = monitors.iter().filter(|m| m.is_internal()).collect();
        let external_enabled: Vec<_> = monitors
            .iter()
            .filter(|m| !m.is_internal() && m.enabled)
            .collect();

        // If there are NO enabled external screens remaining on the system
        if external_enabled.is_empty() {
            let mut rescued = false;

            for internal in internal_monitors {
                if !internal.enabled {
                    warn!(
                        "CRITICAL: No external display active and internal display '{}' is disabled! Triggering automatic rescue...",
                        internal.name
                    );

                    // 1. Turn on the internal screen immediately via Hyprland IPC
                    HyprctlClient::emergency_enable_internal(&internal.name)?;

                    // 2. Persist the change to disk (monitors.lua and monitors.conf)
                    // so future boots/reloads keep the screen turned on
                    let mut updated_monitors = monitors.clone();
                    if let Some(m) = updated_monitors.iter_mut().find(|m| m.name == internal.name) {
                        m.enabled = true;
                        m.position.x = 0;
                        m.position.y = 0;
                    }
                    let config = DisplayConfiguration::new(updated_monitors);
                    if let Err(e) = ConfigPersistence::save(&config) {
                        error!("Failed to persist rescued config: {:#}", e);
                    }

                    // 3. Emit a compositor native notification
                    let _ = Command::new("hyprctl")
                        .args([
                            "notify",
                            "1",
                            "6000",
                            "0",
                            &format!(
                                "Hyprview Rescue: No external display found. Restored laptop screen ({})",
                                internal.name
                            ),
                        ])
                        .output();

                    info!("Successfully rescued internal display '{}'", internal.name);
                    rescued = true;
                }
            }

            return Ok(rescued);
        }

        Ok(false)
    }
}
