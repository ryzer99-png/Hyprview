use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use tokio::io::BufReader;
use tokio::net::UnixStream;
use tracing::info;

pub struct HyprlandSocket;

impl HyprlandSocket {
    /// Resolves the socket2 path from environment variables
    /// Default location: $XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock
    pub fn get_socket2_path() -> Result<PathBuf> {
        let xdg_runtime = env::var("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR not set")?;
        let his = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .context("HYPRLAND_INSTANCE_SIGNATURE not set. Is Hyprland running?")?;

        let socket_path = PathBuf::from(xdg_runtime)
            .join("hypr")
            .join(his)
            .join(".socket2.sock");

        if !socket_path.exists() {
            anyhow::bail!("Hyprland socket2 not found at: {:?}", socket_path);
        }

        Ok(socket_path)
    }

    /// Connects to socket2 and returns an asynchronous line reader
    pub async fn connect() -> Result<BufReader<UnixStream>> {
        let path = Self::get_socket2_path()?;
        info!("Connecting to Hyprland event socket: {:?}", path);
        let stream = UnixStream::connect(path)
            .await
            .context("Failed to connect to Hyprland Unix socket2")?;
        Ok(BufReader::new(stream))
    }
}
