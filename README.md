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

3. **Primary Display (Pantalla Principal ⭐):**
   - Designate any active monitor as the primary display.
   - Automatically re-anchors desktop coordinates so the primary monitor sits at `(0, 0)`.
   - Visual star indicators on canvas cards and top quick-selection pills (`⭐ PRIMARY`).
   - Prioritizes output in Hyprland configuration and dispatches focus to the primary monitor on apply.

4. **Overlap Protection & Banner Alert:**
   - Detects invalid overlapping layouts in Extended Mode before applying.
   - Dynamic `AdwBanner` with 1-click **Auto-Align Side-by-Side** button.
   - Blocks invalid configurations with informative `AdwAlertDialog` explaining Hyprland layout restrictions.

5. **15-Second Safety Rollback Guard:**
   - AdwAlertDialog confirmation countdown on applying changes.
   - Automatically reverts to previous working configuration if not confirmed within 15 seconds (prevents black screens).
   - Persists changes to both `~/.config/hypr/monitors.lua` and `~/.config/hypr/monitors.conf`.

6. **Blackout Prevention & Universal Fallback:**
   - Universal internal display recognition across Linux devices: laptops (`eDP`, `LVDS`), handheld gaming PCs (`DSI` on Steam Deck / ROG Ally), and convertibles/embedded panels.
   - Automatically injects an emergency fallback rule (`monitor = , preferred, auto, 1` / Lua `hl.monitor({ output = "", mode = "preferred", position = "auto" })`) into generated configs.
   - Warns users with an informative `AdwAlertDialog` if they attempt to disable their internal display without an active rescue daemon.

7. **Rescue Daemon Mode (`--daemon`) & Systemd User Service:**
   - Background event watcher on Hyprland IPC socket (`.socket2.sock`).
   - Automatically detects monitor disconnection (`monitorremoved>>*`) and restores the built-in screen if all external displays are unplugged.
   - Evaluates display state at boot time to prevent black-screen cold starts.
   - One-click systemd service installer: `hypr-display-manager --install-service` (or directly from the GUI warning dialog).

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

# Install and enable the rescue daemon systemd service:
cargo run -- --install-service

# Or run the rescue daemon directly:
cargo run -- --daemon
```

---

## Hyprland Configuration Integration

To ensure your displays load at Hyprland startup and protect against blackouts on cable disconnect:

### Option A: Systemd User Service (Recommended)
```bash
hypr-display-manager --install-service
```

### Option B: If using Lua configuration (`hyprland.lua` or `conf/autostart.lua`):
```lua
-- Rescue daemon: guarantees laptop screen is re-enabled if external monitor is disconnected
hl.exec_once("hypr-display-manager --daemon")
```

### Option C: If using legacy configuration (`hyprland.conf`):
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
