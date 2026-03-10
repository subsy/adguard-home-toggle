# AdGuard Home Toggle

A lightweight Linux CLI and system tray app to control [AdGuard Home](https://adguard.com/en/adguard-home/overview.html) protection. Built in Rust with a GTK4 layer-shell OSD for Wayland compositors.

Originally inspired by the [Raycast AdGuard Home extension](https://www.raycast.com/theplgeek/adguard-home), rebuilt for Linux desktops.

## Features

- **Toggle protection** on/off via CLI command (bind to a keyboard shortcut)
- **Snooze protection** for a set duration (1m, 10m, 1h, 8h, etc.)
- **System tray icon** showing current protection status via StatusNotifierItem (SNI)
- **OSD overlay** with shield icon and status text, rendered as a Wayland layer-shell surface
- **Open web UI** in your default browser
- **Status polling** in tray mode keeps the icon in sync with server state

## Dependencies

### Build dependencies

- Rust toolchain (1.85+, edition 2024)
- `pkg-config`
- GTK4 development libraries
- `gtk4-layer-shell` development libraries
- D-Bus development libraries (for `ksni` / system tray)

### Arch Linux

```sh
sudo pacman -S gtk4 gtk4-layer-shell dbus
```

### Ubuntu / Debian

```sh
sudo apt install libgtk-4-dev libgtk4-layer-shell-dev libdbus-1-dev pkg-config
```

### Fedora

```sh
sudo dnf install gtk4-devel gtk4-layer-shell-devel dbus-devel pkg-config
```

## Installation

### From source

```sh
git clone https://github.com/subsy/adguard-home-toggle.git
cd adguard-home-toggle
cargo build --release
```

The binary is at `target/release/adguard-toggle`. Copy it somewhere on your `$PATH`:

```sh
cp target/release/adguard-toggle ~/.local/bin/
```

Or install directly via cargo:

```sh
cargo install --path .
```

Icons are embedded in the binary (rendered from SVG via resvg at runtime), so no separate icon installation is needed.

## Configuration

Create `~/.config/adguard-home-toggle/config.toml`:

```toml
server_url = "http://192.168.1.1:3000"
username = "admin"
password = "your-password"
```

| Field | Description |
|---|---|
| `server_url` | Full URL to your AdGuard Home instance (including port) |
| `username` | AdGuard Home admin username |
| `password` | AdGuard Home admin password |

## Usage

### CLI commands

```sh
# Toggle protection on/off (shows OSD overlay)
adguard-toggle toggle

# Check current status
adguard-toggle status

# Snooze protection for a duration
adguard-toggle snooze 10m    # 10 minutes
adguard-toggle snooze 1h     # 1 hour
adguard-toggle snooze 30s    # 30 seconds

# Open the AdGuard Home web UI
adguard-toggle open

# Run as a system tray icon
adguard-toggle tray
```

### Keyboard shortcut

Bind `adguard-toggle toggle` to a key in your compositor. For example, in Niri:

```kdl
binds {
    Mod+Shift+A { spawn "adguard-toggle" "toggle"; }
}
```

### Autostart tray

Add the tray to your compositor's startup. For Niri:

```kdl
spawn-at-startup "adguard-toggle" "tray"
```

### System tray

The tray icon reflects the current protection state:

- **Green shield** (checkmark) — protection enabled
- **Gray shield** (X) — protection disabled

Left-click the tray icon to toggle protection. Right-click for the menu:

- Enable / Disable Protection
- Snooze 1 minute / 10 minutes / 1 hour / 8 hours
- Open Web UI
- Quit

The tray polls the AdGuard Home API every 10 seconds to stay in sync with changes made elsewhere (e.g. the web UI or mobile app).

### OSD overlay

When you toggle or snooze protection via the CLI, a translucent overlay appears centered on screen for 1.5 seconds showing the shield icon and new status. This uses GTK4 layer-shell, so it works on Wayland compositors that support `wlr-layer-shell-unstable-v1` (Niri, Sway, Hyprland, etc.).

## Project structure

```
src/
├── main.rs      # CLI entry point (clap)
├── api.rs       # AdGuard Home HTTP API client
├── config.rs    # TOML config loader
├── icons.rs     # Embedded SVG icons + resvg ARGB32 renderer
├── osd.rs       # GTK4 layer-shell OSD overlay
└── tray.rs      # SNI system tray (ksni)
```

## AdGuard Home API

This tool uses the following AdGuard Home API endpoints:

| Endpoint | Method | Purpose |
|---|---|---|
| `/control/status` | GET | Fetch protection state and snooze duration |
| `/control/protection` | POST | Enable/disable protection, with optional snooze duration |

Authentication is HTTP Basic Auth using the credentials from your config file.

## License

MIT
