use libadwaita as adw;
use adw::prelude::*;
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use tracing::info;

pub struct SafetyRollbackDialog;

impl SafetyRollbackDialog {
    /// Presents a 15-second safety rollback dialog adhering strictly to GNOME HIG
    pub fn show<FConfirm, FRevert>(
        parent: &impl IsA<gtk4::Widget>,
        on_confirm: FConfirm,
        on_revert: FRevert,
    ) where
        FConfirm: FnOnce() + 'static,
        FRevert: FnOnce() + 'static,
    {
        let countdown = Rc::new(RefCell::new(15));
        let finished = Rc::new(RefCell::new(false));

        let dialog = adw::AlertDialog::builder()
            .heading("Keep New Display Settings?")
            .body("The new display configuration has been applied. Reverting back to previous settings in 15 seconds...")
            .build();

        dialog.add_response("revert", "Revert Now");
        dialog.set_response_appearance("revert", adw::ResponseAppearance::Destructive);

        dialog.add_response("confirm", "Keep Changes");
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("confirm"));
        dialog.set_close_response("revert");

        let dialog_weak = dialog.downgrade();
        let finished_timer = Rc::clone(&finished);
        let on_revert_shared = Rc::new(RefCell::new(Some(on_revert)));
        let on_revert_timer = Rc::clone(&on_revert_shared);

        // Run 1-second countdown tick
        glib::timeout_add_local(Duration::from_secs(1), move || {
            if *finished_timer.borrow() {
                return glib::ControlFlow::Break;
            }

            let mut count = countdown.borrow_mut();
            if *count > 1 {
                *count -= 1;
                if let Some(d) = dialog_weak.upgrade() {
                    d.set_body(&format!(
                        "The new display configuration has been applied. Reverting back to previous settings in {} seconds...",
                        *count
                    ));
                }
                glib::ControlFlow::Continue
            } else {
                // Timeout expired!
                *finished_timer.borrow_mut() = true;
                if let Some(d) = dialog_weak.upgrade() {
                    d.close();
                }
                if let Some(revert) = on_revert_timer.borrow_mut().take() {
                    info!("Rollback timeout expired. Automatically reverting display settings.");
                    revert();
                }
                glib::ControlFlow::Break
            }
        });

        let finished_response = Rc::clone(&finished);
        let on_confirm_cell = Rc::new(RefCell::new(Some(on_confirm)));
        let on_revert_choose = Rc::clone(&on_revert_shared);

        dialog.choose(Some(parent), None::<&gtk4::gio::Cancellable>, move |response| {
            if *finished_response.borrow() {
                return;
            }
            *finished_response.borrow_mut() = true;

            if response == "confirm" {
                info!("Display settings confirmed by user.");
                if let Some(confirm) = on_confirm_cell.borrow_mut().take() {
                    confirm();
                }
            } else {
                info!("Display settings rejected by user. Reverting...");
                if let Some(revert) = on_revert_choose.borrow_mut().take() {
                    revert();
                }
            }
        });
    }
}

pub struct OverlapWarningDialog;

impl OverlapWarningDialog {
    /// Presents a modal alert dialog when extended displays overlap, preventing invalid layout apply.
    pub fn show<FAlign>(
        parent: &impl IsA<gtk4::Widget>,
        overlap_pairs: &[(String, String)],
        on_auto_align: FAlign,
    ) where
        FAlign: FnOnce() + 'static,
    {
        let pairs: Vec<String> = overlap_pairs
            .iter()
            .map(|(a, b)| format!("'{}' and '{}'", a, b))
            .collect();
        let display_names = pairs.join(", ");

        let body = format!(
            "Displays {} are currently overlapping.\n\n\
            In Extended Mode, Hyprland requires each monitor to have its own independent coordinates. Overlapping extended monitors breaks cursor movement and window positioning.\n\n\
            How to resolve:\n\
            • Click 'Auto-Align' to position them neatly side-by-side with zero overlap.\n\
            • Or drag them apart manually on the visual canvas.\n\
            • Or enable Mirror Mode in display settings to duplicate your screen.",
            display_names
        );

        let dialog = adw::AlertDialog::builder()
            .heading("Displays Cannot Overlap")
            .body(&body)
            .build();

        dialog.add_response("cancel", "Dismiss");
        dialog.add_response("auto_align", "Auto-Align Side-by-Side");
        dialog.set_response_appearance("auto_align", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("auto_align"));
        dialog.set_close_response("cancel");

        let on_auto_align_cell = Rc::new(RefCell::new(Some(on_auto_align)));

        dialog.choose(Some(parent), None::<&gtk4::gio::Cancellable>, move |response| {
            if response == "auto_align" {
                if let Some(align) = on_auto_align_cell.borrow_mut().take() {
                    align();
                }
            }
        });
    }
}

pub struct InternalDisplayWarningDialog;

impl InternalDisplayWarningDialog {
    /// Warns the user when disabling an internal screen and offers automatic rescue daemon activation
    pub fn show<FProceed, FEnableService>(
        parent: &impl IsA<gtk4::Widget>,
        internal_name: &str,
        on_proceed: FProceed,
        on_enable_service: FEnableService,
    ) where
        FProceed: FnOnce() + 'static,
        FEnableService: FnOnce() + 'static,
    {
        let body = format!(
            "You are disabling the internal laptop/handheld display ('{}').\n\n\
            ⚠️ Blackout Risk: If you disconnect your external monitor while the internal screen is disabled, Hyprland will have no active screens left, causing a black screen!\n\n\
            Hyprview includes a background Rescue Daemon that listens for monitor disconnects and automatically recovers your internal screen.\n\n\
            What would you like to do?",
            internal_name
        );

        let dialog = adw::AlertDialog::builder()
            .heading("Disabling Integrated Display")
            .body(&body)
            .build();

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("enable_service", "Enable Rescue Daemon (systemd)");
        dialog.add_response("proceed", "Apply Anyway");

        dialog.set_response_appearance("enable_service", adw::ResponseAppearance::Suggested);
        dialog.set_response_appearance("proceed", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("enable_service"));
        dialog.set_close_response("cancel");

        let on_proceed_cell = Rc::new(RefCell::new(Some(on_proceed)));
        let on_enable_service_cell = Rc::new(RefCell::new(Some(on_enable_service)));

        dialog.choose(Some(parent), None::<&gtk4::gio::Cancellable>, move |response| {
            if response == "enable_service" {
                if let Some(enable) = on_enable_service_cell.borrow_mut().take() {
                    enable();
                }
            } else if response == "proceed" {
                if let Some(proceed) = on_proceed_cell.borrow_mut().take() {
                    proceed();
                }
            }
        });
    }
}


