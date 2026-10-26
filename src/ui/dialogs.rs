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

        dialog.choose(Some(parent), None::<&gio::Cancellable>, move |response| {
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
