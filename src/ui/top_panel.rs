use super::app_state::AppState;
use egui::{TopBottomPanel, Ui};
use rfd::FileDialog;
use walkdir::WalkDir;
// use std::path::Path; // Unused

/// Renders the top navigation bar.
pub fn show(ctx: &egui::Context, state: &mut AppState) {
    // Process packing status updates
    let mut done_packing = false;
    if let Some(rx) = &state.pack_status_receiver {
        while let Ok(status) = rx.try_recv() {
            state.is_packing = status.is_packing;
            state.pack_progress = status.progress;
            state.status_message = if let Some(err) = status.error {
                format!("Error: {}", err)
            } else {
                format!(
                    "Packed: {} ({:.0}%)",
                    status.current_file,
                    status.progress * 100.0
                )
            };

            if !state.is_packing {
                done_packing = true;
            }

            // Force repaint to show progress
            ctx.request_repaint();
        }
    }

    if done_packing {
        state.pack_status_receiver = None;
    }

    // Process extraction status updates
    let mut done_extracting = false;
    if let Some(rx) = &state.extract_status_receiver {
        while let Ok(status) = rx.try_recv() {
            state.is_extracting = status.is_extracting;
            state.extract_progress = status.progress;
            state.status_message = if let Some(err) = status.error {
                format!("Extraction Error: {}", err)
            } else {
                format!(
                    "Extracted: {} ({:.0}%)",
                    status.current_file,
                    status.progress * 100.0
                )
            };

            if !state.is_extracting {
                done_extracting = true;
            }

            // Force repaint to show progress
            ctx.request_repaint();
        }
    }

    if done_extracting {
        state.extract_status_receiver = None;
    }

    TopBottomPanel::top("top_nav").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            render_left_menu(ui, state);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                render_right_toolbar(ui, state);
            });
        });
    });
}

fn render_left_menu(ui: &mut Ui, state: &mut AppState) {
    ui.menu_button("File", |ui| {
        if ui.button("Open Folder...").clicked() {
            if let Some(path) = FileDialog::new().pick_folder() {
                state.current_root_dir = Some(path.clone());
                state.status_message = format!("Opened folder: {}", path.display());

                // Scan for files immediately
                state.loaded_files.clear();
                for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        if let Ok(relative) = entry.path().strip_prefix(&path) {
                            state.loaded_files.push(relative.to_path_buf());
                        }
                    }
                }
            }
            ui.close();
        }

        if ui.button("Open Game Install...").clicked() {
            state.status_message = "Opening Game Install Dialog...".to_owned();
            ui.close();
        }
        if ui.button("Open Mod Project...").clicked() {
            state.status_message = "Opening Mod Project Dialog...".to_owned();
            ui.close();
        }
        if ui.button("New Mod Project...").clicked() {
            state.status_message = "Creating New Mod Project...".to_owned();
            ui.close();
        }
        ui.separator();
        if ui.button("Pack Folder...").clicked() {
            if let Some(root) = &state.current_root_dir {
                if let Some(output) = FileDialog::new()
                    .add_filter("PSARC Archive", &["psarc"])
                    .save_file()
                {
                    // Start packing
                    let (tx, rx) = crossbeam_channel::unbounded();
                    state.pack_status_receiver = Some(rx);
                    state.is_packing = true;

                    let root_clone = root.clone();

                    // Call the PSARC module
                    let _ = crate::psarc::pack_directory(&root_clone, &output, move |status| {
                        let _ = tx.send(status);
                    });
                } else {
                    state.status_message = "No output file selected!".to_string();
                }
            } else {
                state.status_message = "No folder opened!".to_string();
            }
            ui.close();
        }
        if ui.button("Extract PSARC...").clicked() {
            if let Some(psarc_file) = FileDialog::new()
                .add_filter("PSARC Archive", &["psarc"])
                .pick_file()
            {
                if let Some(output_dir) = FileDialog::new().pick_folder() {
                    // Start extraction
                    let (tx, rx) = crossbeam_channel::unbounded();
                    state.extract_status_receiver = Some(rx);
                    state.is_extracting = true;

                    let psarc_clone = psarc_file.clone();
                    let output_clone = output_dir.clone();

                    // Call the PSARC extraction module
                    let _ = crate::psarc::extract_psarc(&psarc_clone, &output_clone, move |status| {
                        let _ = tx.send(status);
                    });
                } else {
                    state.status_message = "No output directory selected!".to_string();
                }
            } else {
                state.status_message = "No PSARC file selected!".to_string();
            }
            ui.close();
        }
        ui.separator();
        if ui.button("Save Project").clicked() {
            state.status_message = "Project Saved.".to_owned();
            ui.close();
        }
        if ui.button("Exit").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });

    ui.menu_button("View", |ui| {
        if ui
            .checkbox(&mut state.left_panel_visible, "Toggle Left Panel")
            .clicked()
        {
            ui.close();
        }
        if ui
            .checkbox(&mut state.right_panel_visible, "Toggle Right Panel")
            .clicked()
        {
            ui.close();
        }
        if ui
            .checkbox(&mut state.show_popup, "Toggle Floating Window")
            .clicked()
        {
            ui.close();
        }
        ui.separator();
        if ui.button("Reset Layout").clicked() {
            state.left_panel_visible = true;
            state.right_panel_visible = true;
            ui.close();
        }
    });

    ui.menu_button("Mods", |ui| {
        if ui.button("Build Mod").clicked() {
            state.status_message = "Building mod...".to_owned();
            ui.close();
        }
        if ui.button("Run In Game").clicked() {
            state.status_message = "Launching game...".to_owned();
            ui.close();
        }
        ui.separator();
        if ui.button("Validate").clicked() {
            state.status_message = "Validating mod assets...".to_owned();
            ui.close();
        }
    });

    ui.menu_button("Tools", |ui| {
        if ui
            .checkbox(&mut state.show_popup, "Open Floating Window")
            .clicked()
        {
            ui.close();
        }
        if ui.button("Batch Operations").clicked() {
            state.status_message = "Opening Batch Tools...".to_owned();
            state.show_popup = true;
            ui.close();
        }
        if ui.button("Settings").clicked() {
            state.status_message = "Opening Settings...".to_owned();
            ui.close();
        }
    });

    ui.menu_button("Help", |ui| {
        if ui.button("View Docs").clicked() {
            state.status_message = "Opening Documentation...".to_owned();
            ui.close();
        }
        if ui.button("About").clicked() {
            state.status_message = "Showing About Info...".to_owned();
            ui.close();
        }
    });
}

fn render_right_toolbar(ui: &mut Ui, state: &mut AppState) {
    if state.is_packing {
        ui.add(egui::ProgressBar::new(state.pack_progress).show_percentage());
        ui.label("Packing...");
    } else if state.is_extracting {
        ui.add(egui::ProgressBar::new(state.extract_progress).show_percentage());
        ui.label("Extracting...");
    }

    ui.separator();
    ui.label(format!("Status: {}", state.status_message));
    ui.separator();
    if ui.button("Refresh").clicked() {
        state.status_message = "Refreshing file list...".to_owned();
        // Re-scan if folder is open
        if let Some(path) = &state.current_root_dir {
            state.loaded_files.clear();
            for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Ok(relative) = entry.path().strip_prefix(&path) {
                        state.loaded_files.push(relative.to_path_buf());
                    }
                }
            }
        }
    }
    if ui.button("Reset Camera").clicked() {
        state.status_message = "Camera reset.".to_owned();
    }
}
