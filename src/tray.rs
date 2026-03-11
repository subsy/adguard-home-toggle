use crate::api::AdGuardClient;
use crate::config::Config;
use crate::icons;
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, Tray, TrayService};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct AdGuardTray {
    protected: Arc<Mutex<bool>>,
    config: Arc<Config>,
    icon_on: Vec<Icon>,
    icon_off: Vec<Icon>,
}

impl Tray for AdGuardTray {
    fn id(&self) -> String {
        "adguard-home-toggle".to_string()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let protected = *self.protected.lock().unwrap();
        if protected {
            self.icon_on.clone()
        } else {
            self.icon_off.clone()
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
        .timeout(3000)
        .show();
}

fn render_icon(svg: &str) -> Vec<Icon> {
    // Render at multiple sizes for different DPI displays
    [24, 32, 48, 64]
        .iter()
        .filter_map(|&size| {
            icons::render_svg_to_argb(svg, size).map(|(w, h, data)| Icon {
                width: w,
                height: h,
                data,
            })
        })
        .collect()
}

pub fn run_tray() -> Result<(), String> {
    let config = Arc::new(Config::load()?);
    let client = AdGuardClient::new(&config);
    let status = client.get_status()?;
    let protected = Arc::new(Mutex::new(status.protection_enabled));

    let icon_on = render_icon(icons::shield_on_svg());
    let icon_off = render_icon(icons::shield_off_svg());

    if icon_on.is_empty() || icon_off.is_empty() {
        return Err("Failed to render tray icons".to_string());
    }

    let tray = AdGuardTray {
        protected: protected.clone(),
        config: config.clone(),
        icon_on,
        icon_off,
    };

    let service = TrayService::new(tray);
    let handle = service.handle();

    // Write PID file so CLI can signal us
    let pid_path = pid_file_path();
    let _ = std::fs::create_dir_all(pid_path.parent().unwrap());
    let _ = std::fs::write(&pid_path, std::process::id().to_string());

    // Set up SIGUSR1 handler to trigger immediate refresh
    unsafe {
        libc::signal(libc::SIGUSR1, sigusr1_handler as *const () as libc::sighandler_t);
    }

    let poll_config = config.clone();
    let poll_protected = protected.clone();
    thread::spawn(move || {
        loop {
            // Check every 100ms if we got a signal, full poll every 10s
            for _ in 0..100 {
                thread::sleep(Duration::from_millis(100));
                if SIGUSR1_FLAG.swap(false, Ordering::AcqRel) {
                    // Optimistically apply the state the CLI wrote
                    if let Ok(state) = std::fs::read_to_string(state_file_path()) {
                        let new_state = state.trim() == "1";
                        let mut p = poll_protected.lock().unwrap();
                        if *p != new_state {
                            *p = new_state;
                            handle.update(|_| {});
                        }
                    }
                    // Verify against the server shortly after
                    thread::sleep(Duration::from_secs(1));
                    break;
                }
            }
            // Sync with actual server state
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

    let result = service.run().map_err(|e| format!("Tray service failed: {e}"));
    let _ = std::fs::remove_file(&pid_path);
    result
}

static SIGUSR1_FLAG: AtomicBool = AtomicBool::new(false);

extern "C" fn sigusr1_handler(_sig: libc::c_int) {
    SIGUSR1_FLAG.store(true, Ordering::Release);
}

pub fn pid_file_path() -> std::path::PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("adguard-toggle.pid")
}

pub fn state_file_path() -> std::path::PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("adguard-toggle.state")
}

pub fn signal_tray_refresh(protection_enabled: bool) {
    // Write the new state so the tray can read it without an API call
    let _ = std::fs::write(state_file_path(), if protection_enabled { "1" } else { "0" });
    let pid_path = pid_file_path();
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            unsafe {
                libc::kill(pid, libc::SIGUSR1);
            }
        }
    }
}
