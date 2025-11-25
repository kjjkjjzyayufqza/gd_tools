use super::app_state::AppState;
use egui::Window;
use std::collections::HashSet;
use std::path::PathBuf;

/// Extracts files that belong to a specific arc folder and returns paths relative to that folder.
/// Input paths are relative to root (e.g., "arc_1_ep_8_11/subfolder/file.txt")
/// Output paths are relative to the arc folder (e.g., "subfolder/file.txt")
fn get_modified_files_for_arc(modified_files: &HashSet<PathBuf>, arc_folder_name: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    let arc_prefix = format!("{}/", arc_folder_name);
    let arc_prefix_backslash = format!("{}\\", arc_folder_name);
    
    for path in modified_files {
        let path_str = path.to_string_lossy();
        // Check if path starts with the arc folder name
        if path_str.starts_with(&arc_prefix) || path_str.starts_with(&arc_prefix_backslash) {
            // Extract the part after the arc folder
            let relative = if path_str.starts_with(&arc_prefix) {
                &path_str[arc_prefix.len()..]
            } else {
                &path_str[arc_prefix_backslash.len()..]
            };
            // Normalize to forward slashes for PSARC
            result.insert(relative.replace('\\', "/"));
        } else if let Some(first_component) = path.components().next() {
            // Handle case where path might have different format
            if first_component.as_os_str().to_string_lossy() == arc_folder_name {
                let relative_path: PathBuf = path.components().skip(1).collect();
                result.insert(relative_path.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    
    result
}

/// Renders the pack confirmation modal for Incremental mode.
/// Shows a list of arc folders that will be packed and allows the user to confirm or cancel.
pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_pack_confirm {
        return;
    }

    let mut should_close = false;
    let mut should_start_packing = false;

    Window::new("Confirm Pack")
        .collapsible(false)
        .resizable(false)
        .min_width(350.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading("Pack PSARC (Incremental)");
            ui.separator();
            ui.add_space(5.0);

            // Check if game folder is set
            if state.game_folder.is_none() {
                ui.colored_label(egui::Color32::RED, "Error: Game Folder is not set!");
                ui.label("Please set the Game Folder in Settings before packing.");
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Open Settings").clicked() {
                        state.show_settings = true;
                        should_close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
                return;
            }

            // Check if there are folders to pack
            if state.pending_pack_folders.is_empty() {
                ui.label("No modified arc folders detected.");
                ui.add_space(10.0);
                if ui.button("Close").clicked() {
                    should_close = true;
                }
                return;
            }

            // Display the list of folders to pack
            ui.label("The following arc folders will be packed:");
            ui.add_space(5.0);

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for folder in &state.pending_pack_folders {
                        ui.horizontal(|ui| {
                            ui.label("📦");
                            ui.label(folder);
                        });
                    }
                });

            ui.add_space(10.0);

            // Show output directory
            if let Some(game_folder) = &state.game_folder {
                ui.horizontal(|ui| {
                    ui.label("Output to:");
                    ui.label(
                        egui::RichText::new(game_folder.to_string_lossy().to_string())
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                });
            }

            ui.add_space(10.0);

            // Validate FileList.xml exists in each folder
            let mut missing_filelist_folders: Vec<String> = Vec::new();
            if let Some(root) = &state.current_root_dir {
                for folder in &state.pending_pack_folders {
                    let filelist_path = root.join(folder).join("FileList.xml");
                    if !filelist_path.exists() {
                        missing_filelist_folders.push(folder.clone());
                    }
                }
            }

            if !missing_filelist_folders.is_empty() {
                ui.colored_label(egui::Color32::RED, "Error: Missing FileList.xml in:");
                for folder in &missing_filelist_folders {
                    ui.label(format!("  • {}", folder));
                }
                ui.add_space(10.0);
                if ui.button("Cancel").clicked() {
                    should_close = true;
                }
                return;
            }

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Confirm").clicked() {
                    should_start_packing = true;
                    should_close = true;
                }
                if ui.button("Cancel").clicked() {
                    should_close = true;
                }
            });
        });

    if should_close {
        state.show_pack_confirm = false;
    }

    if should_start_packing {
        start_packing(state);
    }
}

/// Starts the packing process for all pending arc folders
fn start_packing(state: &mut AppState) {
    let root_dir = match &state.current_root_dir {
        Some(dir) => dir.clone(),
        None => {
            state.toasts.error("No root directory set.");
            return;
        }
    };

    let game_folder = match &state.game_folder {
        Some(dir) => dir.clone(),
        None => {
            state.toasts.error("Game folder not set. Please set it in Settings.");
            return;
        }
    };

    let folders = state.pending_pack_folders.clone();
    let compression = state.compression_level.to_flate2();
    
    // Clone modified_files for use in the background thread
    let all_modified_files = state.modified_files.clone();

    // Create channel for status updates
    let (tx, rx) = crossbeam_channel::unbounded();
    state.pack_status_receiver = Some(rx);
    state.is_packing = true;
    state.pack_progress = 0.0;

    // Start packing in background thread
    std::thread::spawn(move || {
        let total_folders = folders.len();
        let mut total_recompressed = 0usize;
        let mut total_reused = 0usize;
        
        for (idx, folder_name) in folders.iter().enumerate() {
            let arc_path = root_dir.join(folder_name);
            let output_path = game_folder.join(format!("{}.psarc", folder_name));

            // Get modified files for this specific arc folder
            let modified_for_arc = get_modified_files_for_arc(&all_modified_files, folder_name);
            let modified_count = modified_for_arc.len();

            // Update progress
            let base_progress = idx as f32 / total_folders as f32;
            let _ = tx.send(crate::psarc::PackingStatus {
                current_file: format!("Packing {} ({} modified files)...", folder_name, modified_count),
                progress: base_progress,
                is_packing: true,
                error: None,
            });

            // Pack the arc folder with incremental support
            match crate::psarc::pack_arc_folder_sync(
                &arc_path,
                &output_path,
                compression,
                &modified_for_arc,
                |file_progress, current_file| {
                    let overall_progress = base_progress + (file_progress / total_folders as f32);
                    let _ = tx.send(crate::psarc::PackingStatus {
                        current_file: format!("[{}] {}", folder_name, current_file),
                        progress: overall_progress,
                        is_packing: true,
                        error: None,
                    });
                },
            ) {
                Ok((recompressed, reused)) => {
                    total_recompressed += recompressed;
                    total_reused += reused;
                    let _ = tx.send(crate::psarc::PackingStatus {
                        current_file: format!("Completed: {}.psarc ({} recompressed, {} cached)", folder_name, recompressed, reused),
                        progress: (idx + 1) as f32 / total_folders as f32,
                        is_packing: true,
                        error: None,
                    });
                }
                Err(e) => {
                    let _ = tx.send(crate::psarc::PackingStatus {
                        current_file: format!("Error packing {}", folder_name),
                        progress: (idx + 1) as f32 / total_folders as f32,
                        is_packing: false,
                        error: Some(format!("Failed to pack {}: {}", folder_name, e)),
                    });
                    return;
                }
            }
        }

        // Final completion status
        let _ = tx.send(crate::psarc::PackingStatus {
            current_file: format!("Done ({} recompressed, {} cached)", total_recompressed, total_reused),
            progress: 1.0,
            is_packing: false,
            error: None,
        });
    });

    state.pending_pack_folders.clear();
}

