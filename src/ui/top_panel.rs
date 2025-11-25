use super::app_state::AppState;
use egui::{TopBottomPanel, Ui};
use rfd::FileDialog;
use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use notify::{Watcher, RecursiveMode, Config};

/// Extracts the arc folder name from a file path.
/// Arc folders are named like "arc_X_ep_Y_Z" (e.g., "arc_1_ep_8_11").
/// Returns the first path component that matches the arc folder pattern.
fn get_arc_folder_from_path(path: &PathBuf) -> Option<String> {
    for component in path.components() {
        if let std::path::Component::Normal(os_str) = component {
            if let Some(name) = os_str.to_str() {
                // Check if it matches arc_*_ep_*_* pattern
                if name.starts_with("arc_") && name.contains("_ep_") {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Collects all unique arc folders that contain modified files.
fn get_modified_arc_folders(modified_files: &HashSet<PathBuf>) -> Vec<String> {
    let mut arc_folders: HashSet<String> = HashSet::new();
    
    for file_path in modified_files {
        if let Some(arc_folder) = get_arc_folder_from_path(file_path) {
            arc_folders.insert(arc_folder);
        }
    }
    
    let mut result: Vec<String> = arc_folders.into_iter().collect();
    result.sort(); // Sort for consistent display order
    result
}

/// Scans a directory and updates the loaded files list
fn scan_directory(state: &mut AppState, path: &Path) {
    state.loaded_files.clear();
    state.initial_file_timestamps.clear();
    state.modified_files.clear();
    
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(relative) = entry.path().strip_prefix(path) {
                state.loaded_files.push(relative.to_path_buf());
                
                // Record initial modification time
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        state.initial_file_timestamps.insert(relative.to_path_buf(), modified);
                    }
                }
            }
        }
    }
}

/// Starts file system watching for the given directory
fn start_file_watcher(state: &mut AppState, path: &Path) {
    // Stop existing watcher if any
    state.file_watcher = None;
    state.file_events_receiver = None;

    // Create channel for file system events
    let (tx, rx) = crossbeam_channel::unbounded();

    // Create watcher configuration
    let config = Config::default()
        .with_poll_interval(std::time::Duration::from_secs(1))
        .with_compare_contents(false);

    // Create watcher with configuration
    match notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let _ = tx.send(event);
    })
    .and_then(|mut watcher| {
        watcher.configure(config)?;
        Ok(watcher)
    }) {
        Ok(mut watcher) => {
            // Watch the directory recursively
            if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                state.status_message = format!("Failed to watch directory: {}", e);
                return;
            }

            state.file_watcher = Some(Box::new(watcher));
            state.file_events_receiver = Some(rx);
            state.status_message = format!("Watching: {}", path.display());
        }
        Err(e) => {
            state.status_message = format!("Failed to create file watcher: {}", e);
        }
    }
}

/// Processes file system events and updates the file list
pub fn process_file_events(ctx: &egui::Context, state: &mut AppState) {
    let mut needs_refresh = false;
    let mut error_message = None;
    let mut modified_paths: Vec<std::path::PathBuf> = Vec::new();

    // Collect events first to avoid borrowing issues
    if let Some(rx) = &state.file_events_receiver {
        while let Ok(event_result) = rx.try_recv() {
            match event_result {
                Ok(event) => {
                    // Check if this is a relevant event (create, remove, rename)
                    match event.kind {
                        notify::EventKind::Create(_)
                        | notify::EventKind::Remove(_)
                        | notify::EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
                            needs_refresh = true;
                        }
                        // Handle content modification events
                        notify::EventKind::Modify(notify::event::ModifyKind::Data(_))
                        | notify::EventKind::Modify(notify::event::ModifyKind::Any)
                        | notify::EventKind::Modify(notify::event::ModifyKind::Metadata(_)) => {
                            // Record modified file paths
                            for path in event.paths {
                                modified_paths.push(path);
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    error_message = Some(format!("File watcher error: {}", e));
                }
            }
        }
    }

    // Update error message if any
    if let Some(err) = error_message {
        state.status_message = err;
    }

    // Process modified files - check if timestamp changed from initial
    if !modified_paths.is_empty() {
        if let Some(root) = &state.current_root_dir {
            for abs_path in modified_paths {
                if let Ok(relative) = abs_path.strip_prefix(root) {
                    let relative_buf = relative.to_path_buf();
                    
                    // Check if this file exists in our initial timestamps
                    if let Some(initial_time) = state.initial_file_timestamps.get(&relative_buf) {
                        // Get current modification time
                        if let Ok(metadata) = std::fs::metadata(&abs_path) {
                            if let Ok(current_time) = metadata.modified() {
                                // Compare timestamps - if different, mark as modified
                                if current_time != *initial_time {
                                    state.modified_files.insert(relative_buf);
                                    ctx.request_repaint();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Refresh file list if needed
    if needs_refresh {
        if let Some(root) = state.current_root_dir.clone() {
            scan_directory(state, &root);
            ctx.request_repaint();
        }
    }
}

/// Completion status for operations
#[derive(Debug)]
pub enum CompletionStatus {
    PackingCompleted,
    ExtractionCompleted,
}

/// Renders the top navigation bar.
/// Returns Some(CompletionStatus) if an operation completed, None otherwise.
pub fn show(ctx: &egui::Context, state: &mut AppState) -> Option<CompletionStatus> {
    // Process file system events first
    process_file_events(ctx, state);
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
        state.modified_files.clear();
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

    let completion_status = if done_packing {
        Some(CompletionStatus::PackingCompleted)
    } else if done_extracting {
        Some(CompletionStatus::ExtractionCompleted)
    } else {
        None
    };

    TopBottomPanel::top("top_nav").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            render_left_menu(ui, state);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                render_right_toolbar(ui, state);
            });
        });

        // Show progress bar below menu bar if packing or extracting
        if state.is_packing || state.is_extracting {
            ui.separator();
            ui.horizontal(|ui| {
                if state.is_packing {
                    ui.add(egui::ProgressBar::new(state.pack_progress).show_percentage());
                } else if state.is_extracting {
                    ui.add(egui::ProgressBar::new(state.extract_progress).show_percentage());
                }
            });
        }
    });

    completion_status
}

fn render_left_menu(ui: &mut Ui, state: &mut AppState) {
    ui.menu_button("File", |ui| {
        if ui.button("Open Folder...").clicked() {
            if let Some(path) = FileDialog::new().pick_folder() {
                state.current_root_dir = Some(path.clone());
                state.status_message = format!("Opened folder: {}", path.display());

                // Scan for files immediately
                scan_directory(state, &path);

                // Start file system watching
                start_file_watcher(state, &path);
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
        if ui.button("Init Game Resources...").clicked() {
            state.show_init_game_dialog = true;
            ui.close();
        }
        if ui.button("Pack Folder...").clicked() {
            if let Some(root) = &state.current_root_dir {
                // Check for incremental packing
                let packing_mode = if state.packing_mode == crate::ui::app_state::PackingMode::Incremental {
                    if state.modified_files.is_empty() {
                        state.toasts.warning("No modified files detected for incremental packing.");
                        state.status_message = "Incremental packing skipped: no changes.".to_string();
                        ui.close();
                        return;
                    }
                    crate::psarc::PackingMode::Incremental
                } else {
                    crate::psarc::PackingMode::Full
                };

                if let Some(output) = FileDialog::new()
                    .add_filter("PSARC Archive", &["psarc"])
                    .save_file()
                {
                    // Start packing
                    let (tx, rx) = crossbeam_channel::unbounded();
                    state.pack_status_receiver = Some(rx);
                    state.is_packing = true;

                    let root_clone = root.clone();
                    let compression = state.compression_level.to_flate2();
                    let modified_files = state.modified_files.clone();
                    let existing_psarc = if packing_mode == crate::psarc::PackingMode::Incremental {
                        Some(output.clone())
                    } else {
                        None
                    };

                    // Call the PSARC module
                    let _ = crate::psarc::pack_directory(
                        &root_clone,
                        &output,
                        compression,
                        packing_mode,
                        modified_files,
                        existing_psarc,
                        move |status| {
                            let _ = tx.send(status);
                        },
                    );
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
            // Reset panel widths to default and force recreation
            state.left_panel_width = None;
            state.right_panel_width = None;
            state.layout_version += 1;
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
            state.show_settings = true;
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
    // Display current packing mode before the Pack PSARC button
    let mode_text = match state.packing_mode {
        crate::ui::app_state::PackingMode::Full => "Full",
        crate::ui::app_state::PackingMode::Incremental => "Incremental",
    };
    ui.label(format!("Mode: {}", mode_text));

    // Pack PSARC button - placed before Status label (right side in RTL layout)
    if ui.button("Pack PSARC").clicked() {
        if state.current_root_dir.is_none() {
            state.toasts.error("No folder opened!");
            state.status_message = "No folder opened!".to_string();
            return;
        }

        // Handle Incremental mode with confirmation popover
        if state.packing_mode == crate::ui::app_state::PackingMode::Incremental {
            if state.modified_files.is_empty() {
                state.toasts.warning("No modified files detected for incremental packing.");
                state.status_message = "Incremental packing skipped: no changes.".to_string();
                return;
            }

            // Get list of arc folders with modified files
            let arc_folders = get_modified_arc_folders(&state.modified_files);
            
            if arc_folders.is_empty() {
                state.toasts.warning("No arc folders found in modified files. Modified files must be inside arc_*_ep_*_* folders.");
                state.status_message = "No arc folders found in modified files.".to_string();
                return;
            }

            // Show confirmation popover
            state.pending_pack_folders = arc_folders;
            state.show_pack_confirm = true;
            state.status_message = "Confirm packing...".to_string();
        } else {
            // Full mode - use legacy file dialog approach
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
                    let compression = state.compression_level.to_flate2();
                    let modified_files = state.modified_files.clone();

                    // Call the PSARC module
                    let _ = crate::psarc::pack_directory(
                        &root_clone,
                        &output,
                        compression,
                        crate::psarc::PackingMode::Full,
                        modified_files,
                        None,
                        move |status| {
                            let _ = tx.send(status);
                        },
                    );
                } else {
                    state.status_message = "No output file selected!".to_string();
                }
            }
        }
    }

    ui.separator();
    if ui.button("Refresh").clicked() {
        state.status_message = "Refreshing file list...".to_owned();
        // Re-scan if folder is open
        if let Some(path) = state.current_root_dir.clone() {
            scan_directory(state, &path);
        }
    }
    if ui.button("Reset Camera").clicked() {
        state.status_message = "Camera reset.".to_owned();
    }
}
