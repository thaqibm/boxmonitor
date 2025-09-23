mod config;
mod monitor;
mod ui;
mod ui_failure_charts;

use clap::Parser;
use color_eyre::Result;
use config::{load_config, load_targets_from_simple_list, parse_targets_from_args};
use monitor::Monitor;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "boxmonitor")]
#[command(about = "A network monitoring tool with TUI interface")]
struct Args {
    #[arg(
        short,
        long,
        help = "Use simple host list format instead of JSON config"
    )]
    simple: bool,

    #[arg(short, long, help = "Show configuration and exit")]
    config: bool,

    #[arg(long, help = "Comma-separated list of hosts (IP or domain) to monitor")]
    ip: Option<String>,

    #[arg(long, help = "Print ping diagnostics to stderr for debugging")]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Check if running as root (required for ICMP ping)
    if !is_root() {
        eprintln!("Error: This program requires root privileges to send ICMP ping packets.");
        eprintln!(
            "Please run with sudo: sudo {}",
            std::env::args().next().unwrap()
        );
        std::process::exit(1);
    }

    let args = Args::parse();

    if args.config {
        show_config().await?;
        return Ok(());
    }

    let config = if args.ip.is_some() {
        let targets = parse_targets_from_args(args.ip)?;
        config::Config {
            targets,
            ..Default::default()
        }
    } else if args.simple {
        let targets = load_targets_from_simple_list()?;
        config::Config {
            targets,
            ..Default::default()
        }
    } else {
        load_config()?
    };

    if config.targets.is_empty() {
        eprintln!("No targets configured. Please add hosts to ~/.config/box/.iplist");
        return Ok(());
    }

    let mut monitor = Monitor::new(
        config.targets.clone(),
        config.ping_interval_ms,
        config.history_size,
        args.debug,
    );

    let targets = Arc::new(Mutex::new(monitor.get_targets().to_vec()));
    let targets_clone = Arc::clone(&targets);

    let monitoring_task = tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(config.ping_interval_ms));

        loop {
            interval.tick().await;

            if let Err(e) = monitor.run_ping_cycle().await {
                eprintln!("Ping cycle error: {}", e);
            }

            let mut targets_guard = targets_clone.lock().await;
            *targets_guard = monitor.get_targets().to_vec();
        }
    });

    let ui_task = tokio::spawn(async move {
        if let Err(e) = ui::run_ui(targets).await {
            eprintln!("UI error: {}", e);
        }
    });

    tokio::select! {
        _ = monitoring_task => {},
        _ = ui_task => {},
    }

    Ok(())
}

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

async fn show_config() -> Result<()> {
    let config = load_config()?;
    println!("Current configuration:");
    println!("{}", serde_json::to_string_pretty(&config)?);

    let config_dir = config::get_config_dir()?;
    println!(
        "\nConfig file location: {}",
        config_dir.join(".iplist").display()
    );

    Ok(())
}
