use crate::models::geometry::{LogicalRect, Point, Transform};
use crate::models::resolution::Mode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Monitor {
    pub id: i64,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub make: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub serial: String,

    #[serde(default = "default_true")]
    pub enabled: bool,
    pub active_mode: Mode,
    pub position: Point,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default)]
    pub vrr: bool,

    #[serde(default)]
    pub available_modes: Vec<Mode>,
    #[serde(default)]
    pub mirror_of: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_scale() -> f64 {
    1.0
}

/// Raw representation from `hyprctl -j monitors all`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawHyprlandMonitor {
    pub id: i64,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub make: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub serial: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f64,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: i32,
    #[serde(default)]
    pub vrr: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub mirror_of: String,
    #[serde(default)]
    pub available_modes: Vec<String>,
}

impl From<RawHyprlandMonitor> for Monitor {
    fn from(raw: RawHyprlandMonitor) -> Self {
        let active_mode = Mode::new(raw.width, raw.height, raw.refresh_rate);

        let available_modes: Vec<Mode> = raw
            .available_modes
            .iter()
            .filter_map(|m_str| m_str.parse().ok())
            .collect();

        let mirror_of = if raw.mirror_of.is_empty() || raw.mirror_of == "none" {
            None
        } else {
            Some(raw.mirror_of)
        };

        Self {
            id: raw.id,
            name: raw.name,
            description: raw.description,
            make: raw.make,
            model: raw.model,
            serial: raw.serial,
            enabled: !raw.disabled,
            active_mode,
            position: Point::new(raw.x, raw.y),
            scale: if raw.scale > 0.0 { raw.scale } else { 1.0 },
            transform: Transform::from_i32(raw.transform),
            vrr: raw.vrr,
            available_modes,
            mirror_of,
        }
    }
}

impl Monitor {
    /// Detects if this output is an integrated laptop display
    pub fn is_internal(&self) -> bool {
        let name_lower = self.name.to_lowercase();
        name_lower.starts_with("edp")
            || name_lower.starts_with("lvds")
            || self.description.to_lowercase().contains("embedded")
    }

    /// Computes the scaled logical size on the desktop
    pub fn logical_size(&self) -> (f64, f64) {
        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        let (raw_w, raw_h) = if self.transform.is_rotated_90() {
            (
                self.active_mode.resolution.height as f64,
                self.active_mode.resolution.width as f64,
            )
        } else {
            (
                self.active_mode.resolution.width as f64,
                self.active_mode.resolution.height as f64,
            )
        };

        (raw_w / scale, raw_h / scale)
    }

    /// Computes the logical bounding box
    pub fn logical_rect(&self) -> LogicalRect {
        let (w, h) = self.logical_size();
        LogicalRect::new(self.position.x, self.position.y, w, h)
    }

    /// Formats as a Hyprland monitor rule directive:
    /// Example: `monitor = DP-1, 2560x1440@144.00, 1920x0, 1.00, transform, 0, vrr, 1`
    /// or `monitor = eDP-1, disable`
    pub fn to_hyprland_directive(&self) -> String {
        if !self.enabled {
            return format!("monitor = {}, disable", self.name);
        }

        if let Some(ref target) = self.mirror_of {
            return format!("monitor = {}, preferred, auto, 1, mirror, {}", self.name, target);
        }

        let transform_idx = self.transform as i32;
        let vrr_idx = if self.vrr { 1 } else { 0 };

        format!(
            "monitor = {}, {}x{}@{:.2}, {}x{}, {:.2}, transform, {}, vrr, {}",
            self.name,
            self.active_mode.resolution.width,
            self.active_mode.resolution.height,
            self.active_mode.refresh_rate,
            self.position.x,
            self.position.y,
            self.scale,
            transform_idx,
            vrr_idx
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_display_detection() {
        let internal = Monitor {
            id: 0,
            name: "eDP-1".to_string(),
            description: "Internal Display".to_string(),
            make: "BOE".to_string(),
            model: "0x123".to_string(),
            serial: "".to_string(),
            enabled: true,
            active_mode: Mode::new(1920, 1080, 144.0),
            position: Point::new(0, 0),
            scale: 1.0,
            transform: Transform::Normal,
            vrr: false,
            available_modes: vec![],
            mirror_of: None,
        };
        assert!(internal.is_internal());

        let external = Monitor {
            name: "DP-2".to_string(),
            description: "LG Electronics 27GL850".to_string(),
            ..internal.clone()
        };
        assert!(!external.is_internal());
    }

    #[test]
    fn test_logical_size_and_rect() {
        let mon = Monitor {
            id: 1,
            name: "DP-1".to_string(),
            description: "Monitor".to_string(),
            make: "".to_string(),
            model: "".to_string(),
            serial: "".to_string(),
            enabled: true,
            active_mode: Mode::new(3840, 2160, 60.0),
            position: Point::new(1920, 0),
            scale: 2.0,
            transform: Transform::Normal,
            vrr: false,
            available_modes: vec![],
            mirror_of: None,
        };

        let (w, h) = mon.logical_size();
        assert_eq!(w, 1920.0);
        assert_eq!(h, 1080.0);

        let rect = mon.logical_rect();
        assert_eq!(rect.origin.x, 1920);
        assert_eq!(rect.origin.y, 0);
        assert_eq!(rect.width, 1920.0);
        assert_eq!(rect.height, 1080.0);
    }

    #[test]
    fn test_hyprland_directive_formatting() {
        let mon = Monitor {
            id: 1,
            name: "DP-1".to_string(),
            description: "Monitor".to_string(),
            make: "".to_string(),
            model: "".to_string(),
            serial: "".to_string(),
            enabled: true,
            active_mode: Mode::new(2560, 1440, 144.0),
            position: Point::new(1920, 0),
            scale: 1.25,
            transform: Transform::Normal,
            vrr: true,
            available_modes: vec![],
            mirror_of: None,
        };

        assert_eq!(
            mon.to_hyprland_directive(),
            "monitor = DP-1, 2560x1440@144.00, 1920x0, 1.25, transform, 0, vrr, 1"
        );

        let mut disabled = mon.clone();
        disabled.enabled = false;
        assert_eq!(disabled.to_hyprland_directive(), "monitor = DP-1, disable");
    }
}
