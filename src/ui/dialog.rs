// src/ui/dialog.rs

use eframe::egui::{self, Context};

/// The types of dialogs you may show.
/// Extend this enum as you add more dialog types.
#[derive(Debug, PartialEq, Eq)]
pub enum Dialog {
    SaveChanges,
}

/// The possible outcomes when a dialog is closed.
#[derive(Debug, PartialEq, Eq)]
pub enum DialogResult {
    Save,
    DontSave,
    Cancel,
}

/// Manages the currently active dialog (if any) and its result.
pub struct DialogManager {
    active_dialog: Option<Dialog>,
    result: Option<DialogResult>,
}

impl Default for DialogManager {
    fn default() -> Self {
        Self {
            active_dialog: None,
            result: None,
        }
    }
}

impl DialogManager {
    /// Create a new DialogManager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Show a dialog by setting it as active.
    pub fn show_dialog(&mut self, dialog: Dialog) {
        self.active_dialog = Some(dialog);
    }

    /// Convenience method specifically for the "Save Changes" dialog.
    pub fn show_save_changes_dialog(&mut self) {
        self.show_dialog(Dialog::SaveChanges);
    }

    /// Call this method on every UI frame to render the active dialog (if any).
    /// When the user responds, the method returns `Some(DialogResult)` and clears the active dialog.
    pub fn update(&mut self, ctx: &Context) -> Option<DialogResult> {
        if let Some(dialog) = &self.active_dialog {
            match dialog {
                Dialog::SaveChanges => {
                    // Render a centered modal window for "Unsaved Changes"
                    egui::Window::new("Unsaved Changes")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        // Optionally, you can force the window to always appear on top.
                        .show(ctx, |ui| {
                            ui.label("You have unsaved changes. Do you want to save before exiting?");
                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    self.result = Some(DialogResult::Save);
                                }
                                if ui.button("Don't Save").clicked() {
                                    self.result = Some(DialogResult::DontSave);
                                }
                                if ui.button("Cancel").clicked() {
                                    self.result = Some(DialogResult::Cancel);
                                }
                            });
                        });
                }
            }
            // If the user has made a selection, retrieve the result and clear the dialog.
            if let Some(result) = self.result.take() {
                self.active_dialog = None;
                return Some(result);
            }
        }
        None
    }
}
