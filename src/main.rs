use anyhow::Result;
use clap::Parser;
use gtk4::prelude::*;
use libadwaita as adw;
use tracing::info;

pub mod backend;
pub mod daemon;
pub mod models;
pub mod safety;
pub mod ui;

#[derive(Parser, Debug)]
#[command(
    name = "hypr-display-manager",
    author = "Hyprview Contributors",
    version,
    about = "Visual, modern, and safe display manager for Hyprland"
)]
struct Args {
    /// Run in background rescue daemon mode (auto-recovers eDP-1 if external screens disconnect)
    #[arg(short, long)]
    daemon: bool,

    /// Test Hyprland IPC connection and print detected monitors
    #[arg(long)]
    test_ipc: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hypr_display_manager=debug,info".into()),
        )
        .init();

    let args = Args::parse();

    if args.daemon {
        info!("Starting in rescue daemon mode...");
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(daemon::RescueDaemon::run())?;
        return Ok(());
    }

    if args.test_ipc {
        info!("Testing Hyprland IPC connection...");
        let monitors = backend::HyprctlClient::get_monitors()?;
        println!("Detected {} monitor(s):", monitors.len());
        for m in monitors {
            let (w, h) = m.logical_size();
            println!(
                " - [{}] {} (enabled: {}, mode: {}, pos: {}x{}, scale: {}, logical: {:.0}x{:.0})",
                if m.is_internal() { "Internal" } else { "External" },
                m.name,
                m.enabled,
                m.active_mode,
                m.position.x,
                m.position.y,
                m.scale,
                w,
                h
            );
        }
        return Ok(());
    }

    info!("Starting hypr-display-manager GUI...");
    let app = adw::Application::builder()
        .application_id("org.hyprland.DisplayManager")
        .build();

    app.connect_activate(|app| {
        let app_window = ui::MainAppWindow::new(app);
        app_window.window.present();
    });

    app.run_with_args(&Vec::<String>::new());
    Ok(())
}
