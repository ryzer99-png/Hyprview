use crate::models::monitor::Monitor;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DisplayConfiguration {
    pub monitors: HashMap<String, Monitor>,
}

impl DisplayConfiguration {
    pub fn new(mut monitors: Vec<Monitor>) -> Self {
        // Ensure at most one enabled monitor is primary; if none, designate the first active
        let has_primary = monitors.iter().any(|m| m.enabled && m.is_primary);
        if !has_primary {
            if let Some(first_active) = monitors.iter_mut().find(|m| m.enabled) {
                first_active.is_primary = true;
            }
        } else {
            let mut found = false;
            for m in monitors.iter_mut() {
                if m.enabled && m.is_primary {
                    if !found {
                        found = true;
                    } else {
                        m.is_primary = false;
                    }
                } else {
                    m.is_primary = false;
                }
            }
        }

        let map = monitors.into_iter().map(|m| (m.name.clone(), m)).collect();
        Self { monitors: map }
    }

    /// Returns the currently designated primary monitor, if any
    pub fn primary_monitor(&self) -> Option<&Monitor> {
        self.monitors.values().find(|m| m.enabled && m.is_primary)
    }

    /// Sets a monitor as the primary display, unsetting any previous primary monitor.
    /// Automatically re-anchors the coordinate origin so the primary monitor is at (0, 0),
    /// preserving the relative offset of all other extended monitors.
    pub fn set_primary(&mut self, name: &str) {
        if let Some(target) = self.monitors.get(name) {
            if !target.enabled {
                return;
            }
            let offset_x = target.position.x;
            let offset_y = target.position.y;

            for m in self.monitors.values_mut() {
                if m.mirror_of.is_none() {
                    m.position.x -= offset_x;
                    m.position.y -= offset_y;
                }
                m.is_primary = m.name == name;
            }
        }
    }

    /// Automatically ensures that when a monitor is enabled, it does not overlap with existing enabled monitors.
    pub fn resolve_placement_on_enable(&mut self, name: &str) {
        let (name_str, is_internal, mon_w) = match self.monitors.get(name) {
            Some(m) if m.enabled && m.mirror_of.is_none() => {
                let (w, _) = m.logical_size();
                (m.name.clone(), m.is_internal(), w)
            }
            _ => return,
        };

        // Check if it overlaps with any other enabled extended monitor
        let has_overlap = self.monitors.values().any(|other| {
            other.name != name_str
                && other.enabled
                && other.mirror_of.is_none()
                && self.monitors[&name_str].logical_rect().overlaps(&other.logical_rect())
        });

        if has_overlap {
            // Find current bounding box of the other active monitors
            let others: Vec<&Monitor> = self
                .monitors
                .values()
                .filter(|other| other.name != name_str && other.enabled && other.mirror_of.is_none())
                .collect();

            if others.is_empty() {
                return;
            }

            let mut min_x = f64::MAX;
            let mut max_x = f64::MIN;
            for other in &others {
                let rect = other.logical_rect();
                if (rect.origin.x as f64) < min_x {
                    min_x = rect.origin.x as f64;
                }
                if rect.right() > max_x {
                    max_x = rect.right();
                }
            }

            if let Some(target) = self.monitors.get_mut(&name_str) {
                if is_internal {
                    // Place laptop screen to the left of existing displays
                    target.position.x = (min_x - mon_w).round() as i32;
                    target.position.y = 0;
                } else {
                    // Place external screen to the right of existing displays
                    target.position.x = max_x.round() as i32;
                    target.position.y = 0;
                }
            }
        }
    }

    /// Checks if at least one screen is enabled to prevent complete blackouts
    pub fn has_active_display(&self) -> bool {
        self.monitors.values().any(|m| m.enabled)
    }

    /// Detects if any non-mirrored (extended) displays overlap each other
    pub fn find_extended_overlaps(&self) -> Vec<(String, String)> {
        let active: Vec<&Monitor> = self
            .monitors
            .values()
            .filter(|m| m.enabled && m.mirror_of.is_none())
            .collect();

        let mut overlaps = Vec::new();
        for i in 0..active.len() {
            for j in (i + 1)..active.len() {
                let r1 = active[i].logical_rect();
                let r2 = active[j].logical_rect();
                if r1.overlaps(&r2) {
                    overlaps.push((active[i].name.clone(), active[j].name.clone()));
                }
            }
        }
        overlaps
    }

    /// Automatically aligns all enabled extended monitors side-by-side with zero overlap
    pub fn auto_arrange_side_by_side(&mut self) {
        let mut sorted_keys: Vec<String> = self.monitors.keys().cloned().collect();
        // Place internal laptop screen on left, or sort by current position
        sorted_keys.sort_by(|a, b| {
            let mon_a = &self.monitors[a];
            let mon_b = &self.monitors[b];
            let a_internal = mon_a.is_internal();
            let b_internal = mon_b.is_internal();
            if a_internal != b_internal {
                b_internal.cmp(&a_internal)
            } else {
                mon_a.position.x.cmp(&mon_b.position.x)
            }
        });

        let mut current_x = 0;
        for key in &sorted_keys {
            if let Some(m) = self.monitors.get_mut(key) {
                if m.enabled && m.mirror_of.is_none() {
                    m.position.x = current_x;
                    m.position.y = 0;
                    let (w, _) = m.logical_size();
                    current_x += w.round() as i32;
                }
            }
        }

        // Re-anchor to the primary monitor so it sits at (0, 0)
        let primary_name = self.primary_monitor().map(|m| m.name.clone());
        if let Some(ref name) = primary_name {
            if let Some(primary_mon) = self.monitors.get(name) {
                let offset_x = primary_mon.position.x;
                let offset_y = primary_mon.position.y;
                for m in self.monitors.values_mut() {
                    if m.mirror_of.is_none() {
                        m.position.x -= offset_x;
                        m.position.y -= offset_y;
                    }
                }
            }
        }
    }


    /// Calculates virtual canvas bounding rectangle: (min_x, min_y, max_x, max_y)
    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        let active: Vec<_> = self.monitors.values().filter(|m| m.enabled).collect();
        if active.is_empty() {
            return (0.0, 0.0, 1920.0, 1080.0);
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for m in active {
            let rect = m.logical_rect();
            if (rect.origin.x as f64) < min_x {
                min_x = rect.origin.x as f64;
            }
            if (rect.origin.y as f64) < min_y {
                min_y = rect.origin.y as f64;
            }
            if rect.right() > max_x {
                max_x = rect.right();
            }
            if rect.bottom() > max_y {
                max_y = rect.bottom();
            }
        }

        (min_x, min_y, max_x, max_y)
    }

    /// Computes diff commands needed to transition Hyprland state to `target`
    pub fn calculate_hyprctl_commands(&self, target: &DisplayConfiguration) -> Vec<String> {
        let mut commands = Vec::new();
        for (name, target_mon) in &target.monitors {
            let changed = match self.monitors.get(name) {
                Some(current_mon) => current_mon != target_mon,
                None => true,
            };
            if changed {
                let directive = target_mon.to_hyprland_directive();
                if let Some(stripped) = directive.strip_prefix("monitor = ") {
                    commands.push(format!("keyword monitor {}", stripped));
                }
            }
        }
        commands
    }

    /// Serializes entire configuration into Hyprland format for `monitors.conf`
    pub fn serialize_hyprland_conf(&self) -> String {
        let mut lines = Vec::new();
        lines.push("# -----------------------------------------------------".to_string());
        lines.push("# Auto-generated by hypr-display-manager. DO NOT EDIT.".to_string());
        lines.push("# -----------------------------------------------------".to_string());
        lines.push("".to_string());

        if let Some(primary) = self.primary_monitor() {
            lines.push(format!("# Primary Monitor: {}", primary.name));
            lines.push("".to_string());
        }

        let mut sorted_monitors: Vec<_> = self.monitors.values().collect();
        sorted_monitors.sort_by(|a, b| b.is_primary.cmp(&a.is_primary).then_with(|| a.name.cmp(&b.name)));

        for mon in sorted_monitors {
            lines.push(mon.to_hyprland_directive());
        }

        lines.push("".to_string());
        lines.join("\n")
    }

    /// Serializes entire configuration into Lua format for `monitors.lua` (Hyprland 0.50+)
    pub fn serialize_hyprland_lua(&self) -> String {
        let mut lines = Vec::new();
        lines.push("-- -----------------------------------------------------".to_string());
        lines.push("-- Auto-generated by hypr-display-manager. DO NOT EDIT.".to_string());
        lines.push("-- -----------------------------------------------------".to_string());
        lines.push("".to_string());

        if let Some(primary) = self.primary_monitor() {
            lines.push(format!("-- Primary Monitor: {}", primary.name));
            lines.push("".to_string());
        }

        let mut sorted_monitors: Vec<_> = self.monitors.values().collect();
        sorted_monitors.sort_by(|a, b| b.is_primary.cmp(&a.is_primary).then_with(|| a.name.cmp(&b.name)));

        for mon in sorted_monitors {
            lines.push(mon.to_lua_directive());
        }

        lines.push("".to_string());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::geometry::{Point, Transform};
    use crate::models::resolution::Mode;

    fn dummy_monitor(name: &str, x: i32, enabled: bool) -> Monitor {
        Monitor {
            id: 0,
            name: name.to_string(),
            description: "Display".to_string(),
            make: "Generic".to_string(),
            model: "Display".to_string(),
            serial: "".to_string(),
            enabled,
            is_primary: x == 0 && enabled,
            active_mode: Mode::new(1920, 1080, 60.0),
            position: Point::new(x, 0),
            scale: 1.0,
            transform: Transform::Normal,
            vrr: false,
            available_modes: vec![],
            mirror_of: None,
        }
    }


    #[test]
    fn test_has_active_display() {
        let empty = DisplayConfiguration::default();
        assert!(!empty.has_active_display());

        let disabled = DisplayConfiguration::new(vec![dummy_monitor("eDP-1", 0, false)]);
        assert!(!disabled.has_active_display());

        let active = DisplayConfiguration::new(vec![dummy_monitor("eDP-1", 0, true)]);
        assert!(active.has_active_display());
    }

    #[test]
    fn test_calculate_hyprctl_commands() {
        let initial = DisplayConfiguration::new(vec![
            dummy_monitor("eDP-1", 0, true),
            dummy_monitor("DP-1", 1920, true),
        ]);

        let mut modified = initial.clone();
        if let Some(m) = modified.monitors.get_mut("DP-1") {
            m.position.x = 2560;
        }

        let cmds = initial.calculate_hyprctl_commands(&modified);
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].contains("keyword monitor DP-1, 1920x1080@60.00, 2560x0, 1.00"));
    }

    #[test]
    fn test_serialize_hyprland_conf() {
        let config = DisplayConfiguration::new(vec![
            dummy_monitor("eDP-1", 0, true),
            dummy_monitor("DP-1", 1920, false),
        ]);

        let content = config.serialize_hyprland_conf();
        assert!(content.contains("monitor = DP-1, disable"));
        assert!(content.contains("monitor = eDP-1, 1920x1080@60.00, 0x0, 1.00"));
    }

    #[test]
    fn test_find_extended_overlaps_and_auto_arrange() {
        // Two overlapping monitors at (0, 0)
        let mut config = DisplayConfiguration::new(vec![
            dummy_monitor("eDP-1", 0, true),
            dummy_monitor("HDMI-A-1", 500, true), // overlaps eDP-1 (width 1920)
        ]);

        let overlaps = config.find_extended_overlaps();
        assert_eq!(overlaps.len(), 1);
        let pair = &overlaps[0];
        assert!(
            (pair.0 == "eDP-1" && pair.1 == "HDMI-A-1") || (pair.0 == "HDMI-A-1" && pair.1 == "eDP-1"),
            "Expected overlap between eDP-1 and HDMI-A-1"
        );


        // Now auto-arrange side by side
        config.auto_arrange_side_by_side();
        let remaining_overlaps = config.find_extended_overlaps();
        assert!(remaining_overlaps.is_empty(), "Auto-arrange must eliminate all overlaps");

        // Verify mirrored monitors do not trigger overlap error
        let mut mirrored_config = DisplayConfiguration::new(vec![
            dummy_monitor("eDP-1", 0, true),
            dummy_monitor("HDMI-A-1", 0, true),
        ]);
        mirrored_config.monitors.get_mut("HDMI-A-1").unwrap().mirror_of = Some("eDP-1".to_string());
        assert!(mirrored_config.find_extended_overlaps().is_empty(), "Mirrored displays should not count as invalid overlaps");
    }

    #[test]
    fn test_set_primary_reanchoring() {
        let mut config = DisplayConfiguration::new(vec![
            dummy_monitor("eDP-1", 0, true),
            dummy_monitor("HDMI-A-1", 1920, true),
        ]);

        assert!(config.monitors["eDP-1"].is_primary);
        assert!(!config.monitors["HDMI-A-1"].is_primary);

        // Switch primary to HDMI-A-1: should become (0,0) and eDP-1 should become (-1920, 0)
        config.set_primary("HDMI-A-1");
        assert!(config.monitors["HDMI-A-1"].is_primary);
        assert!(!config.monitors["eDP-1"].is_primary);
        assert_eq!(config.monitors["HDMI-A-1"].position.x, 0);
        assert_eq!(config.monitors["eDP-1"].position.x, -1920);

        // Primary monitor is serialized first
        let conf = config.serialize_hyprland_conf();
        let hdmi_pos = conf.find("monitor = HDMI-A-1").unwrap();
        let edp_pos = conf.find("monitor = eDP-1").unwrap();
        assert!(hdmi_pos < edp_pos, "Primary monitor must be listed first in configuration");
    }

    #[test]
    fn test_resolve_placement_on_enable() {
        let mut config = DisplayConfiguration::new(vec![
            dummy_monitor("HDMI-A-1", 0, true),
            dummy_monitor("eDP-1", 0, false), // eDP-1 disabled, sitting at (0, 0)
        ]);

        // Enable eDP-1 (which currently shares 0,0 with HDMI-A-1)
        config.monitors.get_mut("eDP-1").unwrap().enabled = true;
        // Resolve placement: eDP-1 is internal laptop screen, so it should move to left of HDMI-A-1 (-1920)
        config.resolve_placement_on_enable("eDP-1");

        assert_eq!(config.monitors["eDP-1"].position.x, -1920);
        assert_eq!(config.monitors["HDMI-A-1"].position.x, 0);
        assert!(config.find_extended_overlaps().is_empty());
    }
}



