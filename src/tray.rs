use crate::api::AdGuardClient;
use crate::config::Config;
use ksni::menu::StandardItem;
use ksni::{MenuItem, Tray, TrayService};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct AdGuardTray {
    protected: Arc<Mutex<bool>>,
    config: Arc<Config>,
    icon_dir: String,
}

impl Tray for AdGuardTray {
    fn id(&self) -> String {
        "adguard-home-toggle".to_string()
    }

    fn icon_theme_path(&self) -> String {
        self.icon_dir.clone()
    }

    fn icon_name(&self) -> String {
        let protected = *self.protected.lock().unwrap();
        if protected {
            "adguard-shield-on".to_string()
        } else {
            "adguard-shield-off".to_string()
        }
    }

    fn title(&self) -> String {
        "AdGuard Home".to_string()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let protected = *self.protected.lock().unwrap();
        let status = if protected { "Protection: ON" } else { "Protection: OFF" };
        ksni::ToolTip {
            title: status.to_string(),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let config = self.config.clone();
        let protected = self.protected.clone();
        let current = *protected.lock().unwrap();
        let client = AdGuardClient::new(&config);
        if client.set_protection(!current).is_ok() {
            *protected.lock().unwrap() = !current;
            notify_status(!current, None);
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let protected = *self.protected.lock().unwrap();
        let toggle_label = if protected {
            "Disable Protection"
        } else {
            "Enable Protection"
        };

        vec![
            MenuItem::Standard(StandardItem {
                label: toggle_label.to_string(),
                activate: Box::new(|tray: &mut Self| {
                    let config = tray.config.clone();
                    let protected = tray.protected.clone();
                    let current = *protected.lock().unwrap();
                    let client = AdGuardClient::new(&config);
                    if client.set_protection(!current).is_ok() {
                        *protected.lock().unwrap() = !current;
                        notify_status(!current, None);
                    }
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Snooze 1 minute".to_string(),
                activate: Box::new(snooze_handler(60_000)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Snooze 10 minutes".to_string(),
                activate: Box::new(snooze_handler(10 * 60_000)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Snooze 1 hour".to_string(),
                activate: Box::new(snooze_handler(60 * 60_000)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Snooze 8 hours".to_string(),
                activate: Box::new(snooze_handler(8 * 60 * 60_000)),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Open Web UI".to_string(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = open::that(&tray.config.server_url);
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(|_: &mut Self| {
                    std::process::exit(0);
                }),
                ..Default::default()
            }),
        ]
    }
}

fn snooze_handler(duration_ms: u64) -> impl Fn(&mut AdGuardTray) {
    move |tray: &mut AdGuardTray| {
        let client = AdGuardClient::new(&tray.config);
        if client.snooze(duration_ms).is_ok() {
            *tray.protected.lock().unwrap() = false;
            let label = format_duration(duration_ms);
            notify_status(false, Some(&label));
        }
    }
}

fn format_duration(ms: u64) -> String {
    let secs = ms / 1000;
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

fn notify_status(enabled: bool, snooze_label: Option<&str>) {
    let (summary, body) = if enabled {
        ("AdGuard Protection Enabled".to_string(), "DNS filtering is active".to_string())
    } else if let Some(dur) = snooze_label {
        ("AdGuard Protection Snoozed".to_string(), format!("Disabled for {dur}"))
    } else {
        ("AdGuard Protection Disabled".to_string(), "DNS filtering is off".to_string())
    };
    let _ = notify_rust::Notification::new()
        .summary(&summary)
        .body(&body)
        .icon(if enabled { "adguard-shield-on" } else { "adguard-shield-off" })
        .timeout(3000)
        .show();
}

fn find_icon_dir() -> String {
    // Check installed location first, then fall back to next to the binary
    let candidates = [
        dirs::data_dir().map(|d| d.join("adguard-home-toggle/icons")),
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("icons"))),
        Some(PathBuf::from("icons")),
    ];
    for candidate in candidates.iter().flatten() {
        if candidate.join("hicolor/scalable/apps/adguard-shield-on.svg").exists() {
            return candidate.to_string_lossy().to_string();
        }
    }
    "icons".to_string()
}

pub fn run_tray() -> Result<(), String> {
    let config = Arc::new(Config::load()?);
    let client = AdGuardClient::new(&config);
    let status = client.get_status()?;
    let protected = Arc::new(Mutex::new(status.protection_enabled));

    let icon_dir = find_icon_dir();

    let tray = AdGuardTray {
        protected: protected.clone(),
        config: config.clone(),
        icon_dir,
    };

    let service = TrayService::new(tray);
    let handle = service.handle();

    let poll_config = config.clone();
    let poll_protected = protected.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(10));
            let client = AdGuardClient::new(&poll_config);
            if let Ok(status) = client.get_status() {
                let mut p = poll_protected.lock().unwrap();
                if *p != status.protection_enabled {
                    *p = status.protection_enabled;
                    handle.update(|_| {});
                }
            }
        }
    });

    service.run().map_err(|e| format!("Tray service failed: {e}"))?;
    Ok(())
}
