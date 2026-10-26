use crate::models::geometry::Transform;
use crate::models::Monitor;
use gtk4::pango;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct MonitorSettingsPanel {
    pub widget: gtk4::ScrolledWindow,
    update_fn: Rc<RefCell<Option<Box<dyn Fn(Monitor)>>>>,
    current_monitor: Rc<RefCell<Option<Monitor>>>,
    suppress_events: Rc<RefCell<bool>>,
    other_monitors_list: Rc<RefCell<Vec<String>>>,

    // Widgets
    title_label: gtk4::Label,
    subtitle_label: gtk4::Label,
    enabled_switch: adw::SwitchRow,
    mirror_combo: adw::ComboRow,
    modes_combo: adw::ComboRow,
    scale_row: adw::ActionRow,
    scale_box_row: adw::PreferencesRow,
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
            .spacing(16)
            .margin_start(18)
            .margin_end(18)
            .margin_top(18)
            .margin_bottom(18)
            .build();

        let scrolled_window = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_width(320)
            .child(&root)
            .build();

        // Header section: Title + Subtitle
        let header_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();

        let title_label = gtk4::Label::builder()
            .label("Select a display")
            .css_classes(["title-2"])
            .halign(gtk4::Align::Start)
            .wrap(true)
            .wrap_mode(pango::WrapMode::WordChar)
            .build();

        let subtitle_label = gtk4::Label::builder()
            .label("")
            .css_classes(["caption", "dim-label"])
            .halign(gtk4::Align::Start)
            .wrap(true)
            .wrap_mode(pango::WrapMode::WordChar)
            .build();

        header_box.append(&title_label);
        header_box.append(&subtitle_label);
        root.append(&header_box);

        let group = adw::PreferencesGroup::builder()
            .title("Display Configuration")
            .build();

        // 1. Enable Switch
        let enabled_switch = adw::SwitchRow::builder()
            .title("Enable Display")
            .subtitle("Turn this output on or off")
            .active(true)
            .build();
        group.add(&enabled_switch);

        // 2. Mirror Mode (Duplicar Pantalla)
        let mirror_combo = adw::ComboRow::builder()
            .title("Display Mode")
            .subtitle("Extend desktop or duplicate another screen")
            .build();
        group.add(&mirror_combo);

        // 3. Mode / Resolution
        let modes_combo = adw::ComboRow::builder()
            .title("Resolution & Refresh")
            .build();
        group.add(&modes_combo);

        // 4. Fractional Scale
        let scale_row = adw::ActionRow::builder()
            .title("Scale")
            .subtitle("UI scaling factor")
            .build();

        let scale_label = gtk4::Label::builder()
            .label("100%")
            .css_classes(["heading", "accent"])
            .valign(gtk4::Align::Center)
            .build();
        scale_row.add_suffix(&scale_label);
        group.add(&scale_row);

        let scale_box_row = adw::PreferencesRow::builder()
            .activatable(false)
            .selectable(false)
            .build();

        let slider_container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_start(16)
            .margin_end(16)
            .margin_top(8)
            .margin_bottom(12)
            .build();

        let scale_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 1.0, 2.0, 0.25);
        scale_scale.set_hexpand(true);
        scale_scale.set_value(1.0);
        scale_scale.add_mark(1.0, gtk4::PositionType::Bottom, Some("100%"));
        scale_scale.add_mark(1.25, gtk4::PositionType::Bottom, Some("125%"));
        scale_scale.add_mark(1.5, gtk4::PositionType::Bottom, Some("150%"));
        scale_scale.add_mark(1.75, gtk4::PositionType::Bottom, Some("175%"));
        scale_scale.add_mark(2.0, gtk4::PositionType::Bottom, Some("200%"));

        slider_container.append(&scale_scale);
        scale_box_row.set_child(Some(&slider_container));
        group.add(&scale_box_row);

        // 5. Transform / Orientation
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

        // 6. VRR / Adaptive Sync
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
        let other_monitors_list = Rc::new(RefCell::new(Vec::new()));

        let panel = Self {
            widget: scrolled_window,
            update_fn,
            current_monitor,
            suppress_events,
            other_monitors_list,
            title_label,
            subtitle_label,
            enabled_switch,
            mirror_combo,
            modes_combo,
            scale_row,
            scale_box_row,
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

        // Mirror combo signal
        let current = Rc::clone(&self.current_monitor);
        let update = Rc::clone(&self.update_fn);
        let suppress = Rc::clone(&self.suppress_events);
        let others = Rc::clone(&self.other_monitors_list);
        let modes_clone = self.modes_combo.clone();
        let scale_row_clone = self.scale_row.clone();
        let scale_box_clone = self.scale_box_row.clone();

        self.mirror_combo.connect_selected_notify(move |combo| {
            if *suppress.borrow() { return; }
            let mut opt = current.borrow_mut();
            if let Some(ref mut mon) = *opt {
                let idx = combo.selected() as usize;
                let others_ref = others.borrow();

                if idx == 0 || others_ref.is_empty() {
                    // Extended mode
                    let was_mirrored = mon.mirror_of.is_some();
                    mon.mirror_of = None;
                    modes_clone.set_sensitive(true);
                    scale_row_clone.set_sensitive(true);
                    scale_box_clone.set_sensitive(true);

                    if was_mirrored {
                        let (w, _) = mon.logical_size();
                        if mon.position.x == 0 {
                            mon.position.x = w.round() as i32;
                        } else {
                            mon.position.x = 0;
                        }
                    }
                } else if idx - 1 < others_ref.len() {
                    // Mirror of another monitor
                    let target_name = others_ref[idx - 1].clone();
                    mon.mirror_of = Some(target_name);
                    modes_clone.set_sensitive(false);
                    scale_row_clone.set_sensitive(false);
                    scale_box_clone.set_sensitive(false);
                }

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

    pub fn set_monitor(&self, monitor: Option<Monitor>, all_monitors: &[Monitor]) {
        *self.suppress_events.borrow_mut() = true;
        *self.current_monitor.borrow_mut() = monitor.clone();

        if let Some(m) = monitor {
            self.widget.set_visible(true);
            self.title_label.set_label(&m.name);

            let desc = if !m.make.is_empty() || !m.model.is_empty() {
                format!("{} {}", m.make, m.model).trim().to_string()
            } else if !m.description.is_empty() {
                m.description.clone()
            } else {
                "Connected Display".to_string()
            };
            self.subtitle_label.set_label(&desc);

            self.enabled_switch.set_active(m.enabled);

            // Populate mirror options (Extended Desktop vs Mirror of <Other>)
            let candidate_targets: Vec<String> = all_monitors
                .iter()
                .filter(|other| other.name != m.name && other.enabled && other.mirror_of.is_none())
                .map(|other| other.name.clone())
                .collect();

            *self.other_monitors_list.borrow_mut() = candidate_targets.clone();

            let mut mirror_labels = vec!["Extended Desktop (Normal)".to_string()];
            for target in &candidate_targets {
                mirror_labels.push(format!("🪞 Duplicate / Mirror of {}", target));
            }

            let str_slices: Vec<&str> = mirror_labels.iter().map(|s| s.as_str()).collect();
            let mirror_model = gtk4::StringList::new(&str_slices);
            self.mirror_combo.set_model(Some(&mirror_model));

            if let Some(ref target) = m.mirror_of {
                if let Some(pos) = candidate_targets.iter().position(|t| t == target) {
                    self.mirror_combo.set_selected((pos + 1) as u32);
                } else {
                    self.mirror_combo.set_selected(0);
                }
                self.modes_combo.set_sensitive(false);
                self.scale_row.set_sensitive(false);
                self.scale_box_row.set_sensitive(false);
            } else {
                self.mirror_combo.set_selected(0);
                self.modes_combo.set_sensitive(true);
                self.scale_row.set_sensitive(true);
                self.scale_box_row.set_sensitive(true);
            }

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
            if m.scale > 2.0 {
                self.scale_scale.set_range(1.0, m.scale);
            } else {
                self.scale_scale.set_range(1.0, 2.0);
            }
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
            self.subtitle_label.set_label("Click a monitor in the canvas to adjust its settings");
            self.widget.set_visible(false);
        }

        *self.suppress_events.borrow_mut() = false;
    }
}
