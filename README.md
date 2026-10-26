# Hyprview (hypr-display-manager)

> Visual, modern, and safe display manager for **Hyprland** on Arch Linux / CachyOS built using **Rust**, **Relm4**, and **Libadwaita**.

---

## Features

1. **Interactive 2D Canvas (Libadwaita / Cairo):**
   - Proportional monitor boxes with real-time drag-and-drop.
   - Magnetic edge snapping (horizontal and vertical alignment against adjacent displays).
   - Cascading card stack, elevated drop shadows, and top quick-select pills for overlapping screens.
   - Pango font layout with high-DPI scaling.

2. **Display Mirroring (Modo Espejo 🪞):**
   - Easily duplicate or mirror any screen with automatic geometry adaptation.

3. **15-Second Safety Rollback Guard:**
   - AdwAlertDialog confirmation countdown on applying changes.
   - Automatically reverts to previous working configuration if not confirmed within 15 seconds (prevents black screens).
   - Persists changes to both `~/.config/hypr/monitors.lua` and `~/.config/hypr/monitors.conf`.

4. **Rescue Daemon Mode (`--daemon`):**
   - Background event watcher on Hyprland IPC socket (`.socket2.sock`).
   - Automatically re-enables laptop internal screen (`eDP-1`) if all external displays are disconnected.
   - Evaluates display state at boot time to prevent black-screen cold starts.

---

## Installation & Build

### Prerequisites (Arch / CachyOS)
```bash
sudo pacman -S base-devel rust libadwaita gtk4 cairo pango
```

### Build & Run
```bash
# Clone the repository
git clone https://github.com/ryzer99-png/Hyprview.git
cd Hyprview

# Run unit tests
cargo test

# Launch GUI
cargo run

# Or inspect connected monitors via CLI:
cargo run -- --test-ipc

# Run the rescue background daemon:
cargo run -- --daemon
```

---

## Hyprland Configuration Integration

To ensure your displays load at Hyprland startup:

### If using Lua configuration (`hyprland.lua` or `conf/autostart.lua`):
```lua
-- Rescue daemon: guarantees laptop screen is re-enabled if external monitor is disconnected
hl.exec_once("hypr-display-manager --daemon")
```

### If using legacy configuration (`hyprland.conf`):
```ini
source = ~/.config/hypr/monitors.conf
exec-once = hypr-display-manager --daemon
```

### Emergency Hotkey
You can also bind an instant recovery hotkey in Hyprland:
```ini
# Instantly recovers the laptop screen if all external displays are gone
bind = $mainMod SHIFT, Escape, exec, hypr-display-manager --rescue
```

---

## License
MIT OR Apache-2.0
