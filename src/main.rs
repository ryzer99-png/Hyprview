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

    /// Trigger an immediate one-shot rescue check and exit (ideal for emergency keybindings)
    #[arg(short, long)]
    rescue: bool,

    /// Test Hyprland IPC connection and print detected monitors
    #[arg(long)]
    test_ipc: bool,

    /// Install and enable the background rescue daemon as a systemd user service
    #[arg(long)]
    install_service: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hypr_display_manager=debug,info".into()),
        )
        .init();

    let args = Args::parse();

    if args.install_service {
        info!("Installing systemd user service for rescue daemon...");
        match daemon::RescueDaemon::install_user_service() {
            Ok(path) => {
                println!("Successfully installed and enabled rescue daemon at: {:?}", path);
                println!("The daemon will automatically recover your internal laptop display whenever external displays disconnect.");
            }
            Err(e) => {
                eprintln!("Failed to install systemd user service: {:#}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if args.daemon {
        info!("Starting in rescue daemon mode...");
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(daemon::RescueDaemon::run())?;
        return Ok(());
    }

    if args.rescue {
        info!("Running one-shot display rescue check...");
        let rt = tokio::runtime::Runtime::new()?;
        let rescued = rt.block_on(daemon::RescueDaemon::evaluate_rescue())?;
        if rescued {
            println!("Rescue triggered: Restored disabled internal display.");
        } else {
            println!("Rescue check complete: No emergency display recovery needed.");
        }
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
