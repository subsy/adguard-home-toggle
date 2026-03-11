mod api;
mod config;
mod icons;
mod osd;
mod tray;

use api::AdGuardClient;
use clap::{Parser, Subcommand};
use config::Config;

#[derive(Parser)]
#[command(name = "adguard-toggle", about = "Control AdGuard Home protection")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show protection status
    Status,
    /// Toggle protection on/off
    Toggle,
    /// Disable protection for a duration (e.g. 1m, 10m, 1h, 8h)
    Snooze {
        /// Duration like 1m, 10m, 1h, 8h
        duration: String,
    },
    /// Open AdGuard Home web UI in browser
    Open,
    /// Run as system tray icon
    Tray,
}

fn parse_duration(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let (num, multiplier) = if let Some(n) = s.strip_suffix('h') {
        (n, 3_600_000u64)
    } else if let Some(n) = s.strip_suffix('m') {
        (n, 60_000u64)
    } else if let Some(n) = s.strip_suffix('s') {
        (n, 1_000u64)
    } else {
        return Err(format!("Invalid duration '{s}'. Use format like 1m, 10m, 1h, 8h"));
    };
    let n: u64 = num.parse().map_err(|_| format!("Invalid number in duration '{s}'"))?;
    Ok(n * multiplier)
}

fn main() {
    let cli = Cli::parse();

    let run = || -> Result<(), String> {
        match cli.command {
            Commands::Tray => return tray::run_tray(),
            _ => {}
        }

        let config = Config::load()?;
        let client = AdGuardClient::new(&config);

        match cli.command {
            Commands::Status => {
                let status = client.get_status()?;
                if status.protection_enabled {
                    println!("Protection: ENABLED");
                } else if status.protection_disabled_duration > 0 {
                    let remaining_secs = status.protection_disabled_duration / 1000;
                    let mins = remaining_secs / 60;
                    let secs = remaining_secs % 60;
                    println!("Protection: SNOOZED ({mins}m {secs}s remaining)");
                } else {
                    println!("Protection: DISABLED");
                }
            }
            Commands::Toggle => {
                let status = client.get_status()?;
                let new_state = !status.protection_enabled;
                client.set_protection(new_state)?;
                let label = if new_state { "ENABLED" } else { "DISABLED" };
                println!("Protection: {label}");
                tray::signal_tray_refresh();
                osd::show(
                    new_state,
                    if new_state { "DNS filtering is active" } else { "DNS filtering is off" },
                );
            }
            Commands::Snooze { duration } => {
                let ms = parse_duration(&duration)?;
                client.snooze(ms)?;
                println!("Protection snoozed for {duration}");
                tray::signal_tray_refresh();
                osd::show(false, &format!("Snoozed for {duration}"));
            }
            Commands::Open => {
                open::that(&config.server_url)
                    .map_err(|e| format!("Failed to open browser: {e}"))?;
            }
            Commands::Tray => unreachable!(),
        }
        Ok(())
    };

    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
