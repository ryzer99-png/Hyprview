use crate::models::geometry::Transform;
use crate::models::Monitor;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct MonitorSettingsPanel {
    pub widget: gtk4::Box,
    update_fn: Rc<RefCell<Option<Box<dyn Fn(Monitor)>>>>,
    current_monitor: Rc<RefCell<Option<Monitor>>>,
    suppress_events: Rc<RefCell<bool>>,

    // Widgets
    title_label: gtk4::Label,
    enabled_switch: adw::SwitchRow,
    modes_combo: adw::ComboRow,
    scale_scale: gtk4::Scale,
    scale_label: gtk4::Label,
    transform_combo: adw::ComboRow,
    vrr_switch: adw::SwitchRow,
}

impl MonitorSettingsPanel {
    pub fn new<FUpdate>(on_update: FUpdate) -> Self
    where
        FUpdate: Fn(Monitor) + 'static,
    {
        let root = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .width_request(340)
            .build();

        let title_label = gtk4::Label::builder()
            .label("Select a display")
            .css_classes(["title-3"])
            .halign(gtk4::Align::Start)
            .build();
        root.append(&title_label);

        let group = adw::PreferencesGroup::builder()
            .title("Display Settings")
            .build();

        // 1. Enable Switch
        let enabled_switch = adw::SwitchRow::builder()
            .title("Enable Display")
            .subtitle("Turn this output on or off")
            .active(true)
            .build();
        group.add(&enabled_switch);

        // 2. Mode / Resolution
        let modes_combo = adw::ComboRow::builder()
            .title("Resolution & Refresh")
            .build();
        group.add(&modes_combo);

        // 3. Fractional Scale
        let scale_row = adw::ActionRow::builder()
            .title("Scale")
            .subtitle("Adjust UI scaling")
            .build();

        let scale_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .valign(gtk4::Align::Center)
            .build();

        let scale_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 1.0, 3.0, 0.25);
        scale_scale.set_width_request(140);
        scale_scale.set_value(1.0);
        scale_scale.add_mark(1.0, gtk4::PositionType::Bottom, Some("100%"));
        scale_scale.add_mark(1.25, gtk4::PositionType::Bottom, None);
        scale_scale.add_mark(1.5, gtk4::PositionType::Bottom, Some("150%"));
        scale_scale.add_mark(2.0, gtk4::PositionType::Bottom, Some("200%"));

        let scale_label = gtk4::Label::builder()
            .label("100%")
            .width_chars(5)
            .build();

        scale_box.append(&scale_scale);
        scale_box.append(&scale_label);
        scale_row.add_suffix(&scale_box);
        group.add(&scale_row);

        // 4. Transform / Orientation
        let transform_model = gtk4::StringList::new(&[
            "Standard (Landscape)",
            "Rotated 90° (Portrait)",
            "Inverted 180°",
            "Rotated 270° (Portrait)",
        ]);
        let transform_combo = adw::ComboRow::builder()
            .title("Orientation")
            .model(&transform_model)
            .selected(0)
            .build();
        group.add(&transform_combo);

        // 5. VRR / Adaptive Sync
        let vrr_switch = adw::SwitchRow::builder()
            .title("Variable Refresh Rate (VRR)")
            .subtitle("FreeSync / G-Sync support")
            .active(false)
            .build();
        group.add(&vrr_switch);

        root.append(&group);

        let update_fn = Rc::new(RefCell::new(Some(Box::new(on_update) as Box<dyn Fn(Monitor)>)));
        let current_monitor = Rc::new(RefCell::new(None));
        let suppress_events = Rc::new(RefCell::new(false));

        let panel = Self {
            widget: root,
            update_fn,
            current_monitor,
            suppress_events,
            title_label,
            enabled_switch,
            modes_combo,
            scale_scale,
            scale_label,
            transform_combo,
            vrr_switch,
        };

        panel.wire_signals();
        panel
    }

    fn wire_signals(&self) {
        // Enabled switch signal
        let current = Rc::clone(&self.current_monitor);
        let update = Rc::clone(&self.update_fn);
        let suppress = Rc::clone(&self.suppress_events);
        self.enabled_switch.connect_active_notify(move |switch| {
            if *suppress.borrow() { return; }
            let mut opt = current.borrow_mut();
            if let Some(ref mut mon) = *opt {
                mon.enabled = switch.is_active();
                if let Some(ref f) = *update.borrow() {
                    f(mon.clone());
                }
            }
        });

        // Mode combo signal
        let current = Rc::clone(&self.current_monitor);
        let update = Rc::clone(&self.update_fn);
        let suppress = Rc::clone(&self.suppress_events);
        self.modes_combo.connect_selected_notify(move |combo| {
            if *suppress.borrow() { return; }
            let mut opt = current.borrow_mut();
            if let Some(ref mut mon) = *opt {
                let idx = combo.selected() as usize;
                if idx < mon.available_modes.len() {
                    mon.active_mode = mon.available_modes[idx];
                    if let Some(ref f) = *update.borrow() {
                        f(mon.clone());
                    }
                }
            }
        });

        // Scale slider signal
        let current = Rc::clone(&self.current_monitor);
        let update = Rc::clone(&self.update_fn);
        let suppress = Rc::clone(&self.suppress_events);
        let label_clone = self.scale_label.clone();
        self.scale_scale.connect_value_changed(move |scale| {
            let val = (scale.value() * 100.0).round() / 100.0;
            label_clone.set_text(&format!("{:.0}%", val * 100.0));
            if *suppress.borrow() { return; }
            let mut opt = current.borrow_mut();
            if let Some(ref mut mon) = *opt {
                mon.scale = val;
                if let Some(ref f) = *update.borrow() {
                    f(mon.clone());
                }
            }
        });

        // Transform combo signal
        let current = Rc::clone(&self.current_monitor);
        let update = Rc::clone(&self.update_fn);
        let suppress = Rc::clone(&self.suppress_events);
        self.transform_combo.connect_selected_notify(move |combo| {
            if *suppress.borrow() { return; }
            let mut opt = current.borrow_mut();
            if let Some(ref mut mon) = *opt {
                mon.transform = match combo.selected() {
                    1 => Transform::Rotate90,
                    2 => Transform::Rotate180,
                    3 => Transform::Rotate270,
                    _ => Transform::Normal,
                };
                if let Some(ref f) = *update.borrow() {
                    f(mon.clone());
                }
            }
        });

        // VRR switch signal
        let current = Rc::clone(&self.current_monitor);
        let update = Rc::clone(&self.update_fn);
        let suppress = Rc::clone(&self.suppress_events);
        self.vrr_switch.connect_active_notify(move |switch| {
            if *suppress.borrow() { return; }
            let mut opt = current.borrow_mut();
            if let Some(ref mut mon) = *opt {
                mon.vrr = switch.is_active();
                if let Some(ref f) = *update.borrow() {
                    f(mon.clone());
                }
            }
        });
    }

    pub fn set_monitor(&self, monitor: Option<Monitor>) {
        *self.suppress_events.borrow_mut() = true;
        *self.current_monitor.borrow_mut() = monitor.clone();

        if let Some(m) = monitor {
            self.widget.set_visible(true);
            self.title_label.set_label(&format!("{} ({})", m.name, if m.description.is_empty() { "Display" } else { &m.description }));
            self.enabled_switch.set_active(m.enabled);

            // Populate modes
            let mut mode_strings: Vec<String> = m.available_modes.iter().map(|mode| mode.to_string()).collect();
            if mode_strings.is_empty() {
                mode_strings.push(m.active_mode.to_string());
            }
            let str_slices: Vec<&str> = mode_strings.iter().map(|s| s.as_str()).collect();
            let string_list = gtk4::StringList::new(&str_slices);
            self.modes_combo.set_model(Some(&string_list));

            // Select active mode
            if let Some(idx) = m.available_modes.iter().position(|mode| *mode == m.active_mode) {
                self.modes_combo.set_selected(idx as u32);
            } else {
                self.modes_combo.set_selected(0);
            }

            // Set scale
            self.scale_scale.set_value(m.scale);
            self.scale_label.set_text(&format!("{:.0}%", m.scale * 100.0));

            // Set transform
            let trans_idx = match m.transform {
                Transform::Rotate90 => 1,
                Transform::Rotate180 => 2,
                Transform::Rotate270 => 3,
                _ => 0,
            };
            self.transform_combo.set_selected(trans_idx);

            // Set VRR
            self.vrr_switch.set_active(m.vrr);
        } else {
            self.title_label.set_label("No display selected");
            self.widget.set_visible(false);
        }

        *self.suppress_events.borrow_mut() = false;
    }
}
