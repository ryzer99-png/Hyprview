# hypr-display-manager

> Visual, modern, and safe display manager for **Hyprland** on Arch Linux / CachyOS built using **Rust**, **Relm4**, and **Libadwaita**.

---

## Features

1. **Interactive 2D Canvas (Libadwaita / Cairo):**
   - Proportional monitor boxes with real-time drag-and-drop.
   - Magnetic edge snapping (horizontal and vertical alignment against adjacent displays).
   - Crisp rendering with Pango font layout and high-DPI scaling.

2. **15-Second Safety Rollback Guard:**
   - AdwAlertDialog confirmation countdown on applying changes.
   - Automatically reverts to previous working configuration if not confirmed within 15 seconds (prevents black screens).
   - Persists changes to `~/.config/hypr/monitors.conf` upon confirmation.

3. **Rescue Daemon Mode (`--daemon`):**
   - Listens to Hyprland's IPC socket (`$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock`).
   - If the internal laptop display (`eDP-1`) was disabled and all external monitors disconnect, it automatically re-enables `eDP-1` at preferred settings.

---

## Installation & Build

### Prerequisites (Arch / CachyOS)
```bash
sudo pacman -S base-devel rust libadwaita gtk4 cairo pango
```

### Build & Run
```bash
# Clone the repository
git clone https://github.com/your-username/hypr-display-manager.git
cd hypr-display-manager

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

To ensure your layout loads at Hyprland startup, add this line to your `~/.config/hypr/hyprland.conf`:

```ini
source = ~/.config/hypr/monitors.conf
```

And to run the rescue daemon in the background on login:

```ini
exec-once = hypr-display-manager --daemon
```

---

## License
MIT OR Apache-2.0
