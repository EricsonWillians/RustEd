// src/ui/file.rs

use std::fs;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use log::{error, info};
use rfd::FileDialog;

use crate::document::Document;
use crate::ui::dialog::{DialogResult, DialogManager};

/// Attempts to open a new map file.  
///
/// If `unsaved_changes` is true, the function first shows the “Unsaved Changes”
// dialog via the provided `dialog_manager`. If the user cancels, the operation
/// is aborted. Otherwise, the file–dialog is presented and, if a file is selected,
/// its contents are loaded and parsed into a Document.
///
/// Returns:
/// - `Some(Document)` if the file was successfully loaded and parsed.
/// - `None` if the user canceled (or if there was an error).
pub fn open_map(
    ctx: &eframe::egui::Context,
    dialog_manager: &mut DialogManager,
    unsaved_changes: bool,
) -> Option<Document> {
    // If there are unsaved changes, show the dialog.
    if unsaved_changes {
        // Trigger the unsaved changes dialog.
        dialog_manager.show_save_changes_dialog();

        // Call update() to try to obtain a response.
        if let Some(result) = dialog_manager.update(ctx) {
            match result {
                DialogResult::Cancel => {
                    // User canceled the file open operation.
                    info!("File open canceled by user after unsaved changes prompt.");
                    return None;
                }
                DialogResult::Save => {
                    // Here you could trigger a save operation.
                    info!("User chose to save changes before opening a new file.");
                    // For example: editor.save_document();
                }
                DialogResult::DontSave => {
                    info!("User chose not to save changes before opening a new file.");
                }
            }
        } else {
            // The dialog is still active (i.e. no response yet), so skip file open for now.
            return None;
        }
    }

    // No unsaved changes or user resolved the dialog—proceed with file selection.
    let file_path: Option<PathBuf> = FileDialog::new()
        .add_filter("WAD Files", &["wad"])
        .add_filter("Map Files", &["map"])
        .pick_file();

    if let Some(path) = file_path {
        info!("Selected file: {:?}", path);

        // Read the entire file synchronously.
        let buffer = match fs::read(&path) {
            Ok(buf) => buf,
            Err(e) => {
                error!("Failed to read file: {}", e);
                return None;
            }
        };

        // Determine the file extension.
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Dispatch to the appropriate parser.
        match ext.as_str() {
            "wad" => parse_wad(&buffer),
            "map" => parse_map(&buffer),
            other => {
                error!("Unsupported file format: {}", other);
                None
            }
        }
    } else {
        info!("File selection cancelled.");
        None
    }
}

/// Parses a WAD file buffer into a Document using your existing Document::load_wad logic.
fn parse_wad(buffer: &[u8]) -> Option<Document> {
    info!("Parsing WAD file ({} bytes)", buffer.len());
    let mut cursor = Cursor::new(buffer);
    let mut doc = Document::new();
    match doc.load_wad(&mut cursor) {
        Ok(_) => Some(doc),
        Err(e) => {
            error!("Error loading WAD file: {}", e);
            None
        }
    }
}

/// Parses a custom MAP file buffer into a Document.
/// Currently a stub; replace with your custom map parsing logic.
fn parse_map(buffer: &[u8]) -> Option<Document> {
    info!("Parsing MAP file ({} bytes)", buffer.len());
    // TODO: Implement your custom map parsing logic here.
    Some(Document::new())
}
