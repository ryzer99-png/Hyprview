use crate::backend::{HyprctlClient, HyprlandSocket};
use anyhow::Result;
use tokio::io::AsyncBufReadExt;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

pub struct RescueDaemon;

impl RescueDaemon {
    pub async fn run() -> Result<()> {
        info!("Starting Hyprland display rescue daemon...");

        loop {
            match HyprlandSocket::connect().await {
                Ok(mut reader) => {
                    info!("Rescue daemon actively listening for Hyprland events");
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
                                    info!("Received event: {}", event);
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

    /// Evaluates whether an emergency display recovery is needed
    pub async fn evaluate_rescue() -> Result<()> {
        let monitors = HyprctlClient::get_monitors()?;

        let internal_monitors: Vec<_> = monitors.iter().filter(|m| m.is_internal()).collect();
        let external_enabled: Vec<_> = monitors
            .iter()
            .filter(|m| !m.is_internal() && m.enabled)
            .collect();

        // If there are NO enabled external screens remaining
        if external_enabled.is_empty() {
            // Check if we have an internal screen that is disabled
            for internal in internal_monitors {
                if !internal.enabled {
                    warn!(
                        "CRITICAL: All external displays disconnected and internal display '{}' is disabled! Triggering automatic rescue...",
                        internal.name
                    );
                    HyprctlClient::emergency_enable_internal(&internal.name)?;
                }
            }
        }

        Ok(())
    }
}
