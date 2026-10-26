use crate::backend::{ConfigPersistence, HyprctlClient};
use crate::models::{DisplayConfiguration, Monitor};
use crate::safety::RollbackGuard;
use crate::ui::canvas::DisplayCanvas;
use crate::ui::dialogs::{SafetyRollbackDialog, OverlapWarningDialog};
use crate::ui::settings::MonitorSettingsPanel;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{error, info};

pub struct AppState {
    pub baseline_config: DisplayConfiguration,
    pub draft_config: DisplayConfiguration,
    pub selected_monitor: Option<String>,
}

#[derive(Clone)]
pub struct MainAppWindow {
    pub window: adw::ApplicationWindow,
    toast_overlay: adw::ToastOverlay,
    warning_banner: adw::Banner,
    canvas: DisplayCanvas,
    settings_panel: MonitorSettingsPanel,
    state: Rc<RefCell<AppState>>,
}

impl MainAppWindow {
    pub fn new(app: &adw::Application) -> Self {
        let state = Rc::new(RefCell::new(AppState {
            baseline_config: DisplayConfiguration::default(),
            draft_config: DisplayConfiguration::default(),
            selected_monitor: None,
        }));

        let toast_overlay = adw::ToastOverlay::new();

        // Warning banner for invalid layout / overlaps
        let warning_banner = adw::Banner::builder()
            .button_label("Auto-Align")
            .use_markup(false)
            .revealed(false)
            .build();

        // Header bar setup
        let header_bar = adw::HeaderBar::new();


        let reload_btn = gtk4::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Reload current monitor layout")
            .build();
        header_bar.pack_start(&reload_btn);

        let auto_align_btn = gtk4::Button::builder()
            .icon_name("view-grid-symbolic")
            .tooltip_text("Auto-align displays side-by-side (resolves overlaps)")
            .build();
        header_bar.pack_start(&auto_align_btn);

        let sidebar_toggle_btn = gtk4::ToggleButton::builder()
            .icon_name("view-right-pane-symbolic")
            .tooltip_text("Toggle Display Settings Panel")
            .active(true)
            .visible(false) // Only visible when collapsed by breakpoint
            .build();
        header_bar.pack_end(&sidebar_toggle_btn);

        let apply_btn = gtk4::Button::builder()
            .label("Apply Changes")
            .css_classes(["suggested-action"])
            .tooltip_text("Apply display configuration with 15s safety rollback")
            .build();
        header_bar.pack_end(&apply_btn);

        // Create Canvas with selection & movement callbacks
        let state_select = Rc::clone(&state);
        let settings_select_cell: Rc<RefCell<Option<MonitorSettingsPanel>>> = Rc::new(RefCell::new(None));
        let settings_select_ref = Rc::clone(&settings_select_cell);
        let banner_cell: Rc<RefCell<Option<adw::Banner>>> = Rc::new(RefCell::new(Some(warning_banner.clone())));
        let banner_move_ref = Rc::clone(&banner_cell);

        let canvas = DisplayCanvas::new(
            move |name| {
                let mut s = state_select.borrow_mut();
                s.selected_monitor = Some(name.clone());
                if let Some(ref panel) = *settings_select_ref.borrow() {
                    let mon = s.draft_config.monitors.get(&name).cloned();
                    let all: Vec<Monitor> = s.draft_config.monitors.values().cloned().collect();
                    panel.set_monitor(mon, &all);
                }
            },
            {
                let state_move = Rc::clone(&state);
                let settings_move_ref = Rc::clone(&settings_select_cell);
                let banner_ref = Rc::clone(&banner_move_ref);
                move |name, x, y| {
                    let mut s = state_move.borrow_mut();
                    let is_selected = s.selected_monitor.as_deref() == Some(&name);
                    let mut updated_mon = None;

                    if let Some(mon) = s.draft_config.monitors.get_mut(&name) {
                        mon.position.x = x;
                        mon.position.y = y;
                        if is_selected {
                            updated_mon = Some(mon.clone());
                        }
                    }

                    if let Some(mon) = updated_mon {
                        if let Some(ref panel) = *settings_move_ref.borrow() {
                            let all: Vec<Monitor> = s.draft_config.monitors.values().cloned().collect();
                            panel.set_monitor(Some(mon), &all);
                        }
                    }

                    if let Some(ref banner) = *banner_ref.borrow() {
                        let overlaps = s.draft_config.find_extended_overlaps();
                        if !overlaps.is_empty() {
                            let names: Vec<String> = overlaps
                                .iter()
                                .map(|(a, b)| format!("'{}' and '{}'", a, b))
                                .collect();
                            banner.set_title(&format!(
                                "Displays {} overlap! Extended monitors cannot share coordinates in Hyprland.",
                                names.join(", ")
                            ));
                            banner.set_revealed(true);
                        } else {
                            banner.set_revealed(false);
                        }
                    }
                }
            },
        );

        // Create settings panel
        let state_update = Rc::clone(&state);
        let canvas_update = canvas.clone();
        let banner_settings_ref = Rc::clone(&banner_cell);
        let settings_panel = MonitorSettingsPanel::new(move |updated_mon| {
            let mut s = state_update.borrow_mut();
            s.draft_config.monitors.insert(updated_mon.name.clone(), updated_mon);
            let mon_list: Vec<Monitor> = s.draft_config.monitors.values().cloned().collect();
            canvas_update.set_monitors(mon_list);

            if let Some(ref banner) = *banner_settings_ref.borrow() {
                let overlaps = s.draft_config.find_extended_overlaps();
                if !overlaps.is_empty() {
                    let names: Vec<String> = overlaps
                        .iter()
                        .map(|(a, b)| format!("'{}' and '{}'", a, b))
                        .collect();
                    banner.set_title(&format!(
                        "Displays {} overlap! Extended monitors cannot share coordinates in Hyprland.",
                        names.join(", ")
                    ));
                    banner.set_revealed(true);
                } else {
                    banner.set_revealed(false);
                }
            }
        });

        *settings_select_cell.borrow_mut() = Some(settings_panel.clone());

        // Adaptive OverlaySplitView (Libadwaita 1.4+)
        let split_view = adw::OverlaySplitView::builder()
            .sidebar_position(gtk4::PackType::End)
            .min_sidebar_width(340.0)
            .max_sidebar_width(420.0)
            .enable_show_gesture(true)
            .enable_hide_gesture(true)
            .content(&canvas.widget)
            .sidebar(&settings_panel.widget)
            .build();

        // Wire sidebar toggle button
        let split_view_toggle = split_view.clone();
        sidebar_toggle_btn.connect_toggled(move |btn| {
            split_view_toggle.set_show_sidebar(btn.is_active());
        });

        let btn_sync = sidebar_toggle_btn.clone();
        split_view.connect_show_sidebar_notify(move |view| {
            btn_sync.set_active(view.shows_sidebar());
        });

        let main_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();

        main_box.append(&header_bar);
        main_box.append(&warning_banner);
        main_box.append(&split_view);

        toast_overlay.set_child(Some(&main_box));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Hyprland Display Manager")
            .default_width(980)
            .default_height(640)
            .content(&toast_overlay)
            .build();

        // Responsive Breakpoint for screen widths < 880px
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            880.0,
            adw::LengthUnit::Px,
        ));
        breakpoint.add_setter(&split_view, "collapsed", Some(&true.to_value()));
        breakpoint.add_setter(&sidebar_toggle_btn, "visible", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        let app_window = Self {
            window,
            toast_overlay,
            warning_banner,
            canvas,
            settings_panel,
            state,
        };

        app_window.wire_actions(reload_btn, apply_btn, auto_align_btn);
        app_window.reload_monitors();

        app_window
    }

    fn wire_actions(&self, reload_btn: gtk4::Button, apply_btn: gtk4::Button, auto_align_btn: gtk4::Button) {
        // Reload action
        let app_reload = self.clone();
        reload_btn.connect_clicked(move |_| {
            app_reload.reload_monitors();
            app_reload.show_toast("Reloaded display configuration from Hyprland");
        });

        // Auto-align button action (resolves overlaps)
        let app_align = self.clone();
        auto_align_btn.connect_clicked(move |_| {
            app_align.auto_align();
        });

        // Banner Auto-Align button action
        let app_banner_align = self.clone();
        self.warning_banner.connect_button_clicked(move |_| {
            app_banner_align.auto_align();
        });

        // Apply action with 15-second safety rollback
        let app_apply = self.clone();
        apply_btn.connect_clicked(move |_| {
            app_apply.apply_changes();
        });
    }

    pub fn auto_align(&self) {
        {
            let mut s = self.state.borrow_mut();
            s.draft_config.auto_arrange_side_by_side();
            let mon_list: Vec<Monitor> = s.draft_config.monitors.values().cloned().collect();
            self.canvas.set_monitors(mon_list.clone());
            if let Some(ref sel) = s.selected_monitor {
                if let Some(mon) = s.draft_config.monitors.get(sel).cloned() {
                    self.settings_panel.set_monitor(Some(mon), &mon_list);
                }
            }
        }
        self.update_validation_status();
        self.show_toast("Displays aligned side-by-side with zero overlap");
    }

    pub fn update_validation_status(&self) {
        let s = self.state.borrow();
        let overlaps = s.draft_config.find_extended_overlaps();
        if !overlaps.is_empty() {
            let names: Vec<String> = overlaps
                .iter()
                .map(|(a, b)| format!("'{}' and '{}'", a, b))
                .collect();
            self.warning_banner.set_title(&format!(
                "Displays {} overlap! Extended monitors cannot share coordinates in Hyprland.",
                names.join(", ")
            ));
            self.warning_banner.set_revealed(true);
        } else {
            self.warning_banner.set_revealed(false);
        }
    }

    pub fn reload_monitors(&self) {
        match HyprctlClient::get_monitors() {
            Ok(monitors) => {
                let config = DisplayConfiguration::new(monitors.clone());
                let mut s = self.state.borrow_mut();
                s.baseline_config = config.clone();
                s.draft_config = config;

                let first_name = monitors.first().map(|m| m.name.clone());
                s.selected_monitor = first_name.clone();

                self.canvas.set_monitors(monitors.clone());
                if let Some(name) = first_name {
                    self.canvas.set_selected(name.clone());
                    let mon = s.draft_config.monitors.get(&name).cloned();
                    self.settings_panel.set_monitor(mon, &monitors);
                }
                drop(s);
                self.update_validation_status();
            }
            Err(e) => {
                error!("Failed to query monitors: {:#}", e);
                self.show_toast(&format!("Error querying Hyprland: {}", e));
            }
        }
    }

    pub fn apply_changes(&self) {
        let (baseline, draft) = {
            let s = self.state.borrow();
            (s.baseline_config.clone(), s.draft_config.clone())
        };

        if !draft.has_active_display() {
            self.show_toast("Safety Error: At least one display must remain enabled!");
            return;
        }

        let overlaps = draft.find_extended_overlaps();
        if !overlaps.is_empty() {
            let app_align = self.clone();
            OverlapWarningDialog::show(
                &self.window,
                &overlaps,
                move || {
                    app_align.auto_align();
                },
            );
            return;
        }

        let changed_monitors: Vec<Monitor> = draft
            .monitors
            .values()
            .filter(|target_mon| {
                match baseline.monitors.get(&target_mon.name) {
                    Some(curr) => curr != *target_mon,
                    None => true,
                }
            })
            .cloned()
            .collect();

        if changed_monitors.is_empty() {
            self.show_toast("No changes detected.");
            return;
        }

        info!("Applying candidate display configuration to Hyprland...");
        if let Err(e) = HyprctlClient::apply_monitors(&changed_monitors) {
            error!("Failed to apply display configuration: {:#}", e);
            self.show_toast(&format!("Failed to apply: {}", e));
            return;
        }

        // Display 15-second safety countdown dialog
        let mut guard = RollbackGuard::new(baseline.clone());
        guard.apply_candidate(draft.clone());

        let app_confirm = self.clone();
        let app_revert = self.clone();
        let revert_monitors: Vec<Monitor> = baseline
            .monitors
            .values()
            .filter(|target_mon| {
                match draft.monitors.get(&target_mon.name) {
                    Some(curr) => curr != *target_mon,
                    None => true,
                }
            })
            .cloned()
            .collect();

        SafetyRollbackDialog::show(
            &self.window,
            move || {
                // User confirmed: persist to monitors.lua / monitors.conf
                match ConfigPersistence::save(&draft) {
                    Ok(path) => {
                        info!("Configuration persisted to {:?}", path);
                        let mut s = app_confirm.state.borrow_mut();
                        s.baseline_config = draft;
                        drop(s);
                        app_confirm.update_validation_status();
                        app_confirm.show_toast("Display settings kept and saved!");
                    }
                    Err(e) => {
                        error!("Failed to save configuration: {:#}", e);
                        app_confirm.show_toast(&format!("Failed to save: {}", e));
                    }
                }
            },
            move || {
                // User canceled or timeout expired: rollback immediately
                info!("Executing rollback to previous baseline configuration...");
                if let Err(e) = HyprctlClient::apply_monitors(&revert_monitors) {
                    error!("Critical error during rollback: {:#}", e);
                }
                app_revert.reload_monitors();
                app_revert.show_toast("Display settings reverted to previous state.");
            },
        );
    }

    pub fn show_toast(&self, message: &str) {
        let toast = adw::Toast::builder()
            .title(message)
            .use_markup(false)
            .timeout(3)
            .build();
        self.toast_overlay.add_toast(toast);
    }
}

