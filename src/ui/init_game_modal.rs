use super::app_state::AppState;
use egui::{Window, Button, TextEdit};
use rfd::FileDialog;
use std::path::PathBuf;
use std::thread;
use crossbeam_channel;

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    // Process extraction progress updates
    if let Some(rx) = &state.init_game_extraction_progress_receiver {
        while let Ok((progress, current_file)) = rx.try_recv() {
            state.init_game_extraction_progress = progress;
            state.init_game_current_file = current_file;
            ctx.request_repaint();
        }
    }
    
    // Process extraction completion
    let mut completion_result: Option<(bool, String, Vec<String>)> = None;
    {
        if let Some(rx) = &state.init_game_extraction_receiver {
            while let Ok(result) = rx.try_recv() {
                completion_result = Some(result);
            }
        }
    }
    
    if let Some((success, message, errors)) = completion_result {
        state.init_game_is_extracting = false;
        state.init_game_extraction_progress = 0.0;
        state.init_game_current_file.clear();
        state.init_game_extraction_progress_receiver = None;
        state.init_game_extraction_receiver = None;
        
        if success {
            state.toasts.success(&message)
                .duration(std::time::Duration::from_secs(5));
        } else {
            state.toasts.error(&message)
                .duration(std::time::Duration::from_secs(10));
            if !errors.is_empty() {
                for error in errors {
                    state.toasts.error(&error)
                        .duration(std::time::Duration::from_secs(5));
                }
            }
        }
    }
    
    // Prevent closing window during extraction
    let mut open = state.show_init_game_dialog;
    if state.init_game_is_extracting {
        // Force window to stay open during extraction
        open = true;
    }
    
    Window::new("Init Game Resources")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size([600.0, 500.0])
        .show(ctx, |ui| {
            ui.heading("Gravity Daze/Gravity Rush");
            ui.separator();
            ui.add_space(10.0);

            // PSARC file selection fields
            let psarc_names = [
                "arc_0_ep_0_0.psarc",
                "arc_0_ep_1_7.psarc",
                "arc_1_ep_8_11.psarc",
                "arc_2_ep_12_30.psarc",
                "arc_3_ep_31_31.psarc",
            ];

            for (idx, name) in psarc_names.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", name));
                    let display_text = state.init_game_psarc_files[idx]
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Not selected".to_string());
                    
                    let mut text_buffer = display_text;
                    let text_edit = TextEdit::singleline(&mut text_buffer)
                        .desired_width(300.0)
                        .interactive(false);
                    ui.add(text_edit);
                    
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("PSARC Archive", &["psarc"])
                            .pick_file()
                        {
                            state.init_game_psarc_files[idx] = Some(path);
                        }
                    }
                    
                    if state.init_game_psarc_files[idx].is_some() {
                        if ui.button("Remove").clicked() {
                            state.init_game_psarc_files[idx] = None;
                        }
                    }
                });
                ui.add_space(5.0);
            }

            ui.separator();
            ui.add_space(10.0);

            // Output directory selection
            ui.label("Output Directory:");
            ui.horizontal(|ui| {
                let display_text = state.init_game_output_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Not selected".to_string());
                
                let mut text_buffer = display_text;
                let text_edit = TextEdit::singleline(&mut text_buffer)
                    .desired_width(300.0)
                    .interactive(false);
                ui.add(text_edit);
                
                if ui.button("Browse...").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        state.init_game_output_dir = Some(path);
                    }
                }
                
                if state.init_game_output_dir.is_some() {
                    if ui.button("Remove").clicked() {
                        state.init_game_output_dir = None;
                    }
                }
            });

            ui.add_space(10.0);
            ui.label("Note: The following folders will be automatically created in the output directory:");
            ui.indent("output_indent", |ui| {
                ui.label("• arc_0_ep_0_0");
                ui.label("• arc_0_ep_1_7");
                ui.label("• arc_1_ep_8_11");
                ui.label("• arc_2_ep_12_30");
                ui.label("• arc_3_ep_31_31");
            });

            ui.add_space(20.0);
            ui.separator();

            // Show extraction progress if extracting
            if state.init_game_is_extracting {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!("Extracting: {}", state.init_game_current_file));
                });
                ui.add(egui::ProgressBar::new(state.init_game_extraction_progress).show_percentage());
                ui.add_space(10.0);
            }

            // Extract button - centered at bottom
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                let button_text = if state.init_game_is_extracting {
                    "Extracting..."
                } else {
                    "Extract"
                };
                
                if ui.add_enabled(
                    !state.init_game_is_extracting &&
                    state.init_game_output_dir.is_some() && 
                    state.init_game_psarc_files.iter().any(|f| f.is_some()),
                    Button::new(button_text)
                ).clicked() {
                    extract_game_resources(state, ctx);
                }
            });
        });
    
    // Only allow closing if not extracting
    if !state.init_game_is_extracting {
        state.show_init_game_dialog = open;
    }
}

fn extract_game_resources(state: &mut AppState, ctx: &egui::Context) {
    let output_dir = match &state.init_game_output_dir {
        Some(dir) => dir.clone(),
        None => {
            state.toasts.error("Output directory not selected!");
            return;
        }
    };

    let folder_names = [
        "arc_0_ep_0_0",
        "arc_0_ep_1_7",
        "arc_1_ep_8_11",
        "arc_2_ep_12_30",
        "arc_3_ep_31_31",
    ];

    let mut files_to_extract: Vec<(PathBuf, String)> = Vec::new();
    for (idx, psarc_file) in state.init_game_psarc_files.iter().enumerate() {
        if let Some(file_path) = psarc_file {
            files_to_extract.push((file_path.clone(), folder_names[idx].to_string()));
        }
    }

    if files_to_extract.is_empty() {
        state.toasts.error("No PSARC files selected!");
        return;
    }

    // Set extracting state
    state.init_game_is_extracting = true;
    state.init_game_extraction_progress = 0.0;
    state.init_game_current_file = "Starting...".to_string();

    // Start extraction in background thread
    let output_dir_clone = output_dir.clone();
    let files_clone = files_to_extract.clone();
    let total_files = files_clone.len();
    let (tx, rx) = crossbeam_channel::unbounded();
    let (progress_tx, progress_rx) = crossbeam_channel::unbounded();
    
    // Store progress receiver
    state.init_game_extraction_progress_receiver = Some(progress_rx);
    
    thread::spawn(move || {
        let mut success_count = 0;
        let mut fail_count = 0;
        let mut errors = Vec::new();

        for (file_idx, (psarc_path, folder_name)) in files_clone.iter().enumerate() {
            // Update progress
            let _ = progress_tx.send((
                (file_idx as f32) / (total_files as f32),
                format!("Extracting {}...", psarc_path.file_name().unwrap_or_default().to_string_lossy())
            ));
            
            let target_dir = output_dir_clone.join(folder_name);
            
            // Create target directory
            if let Err(e) = std::fs::create_dir_all(&target_dir) {
                let error_msg = format!("Failed to create directory {}: {}", folder_name, e);
                errors.push(error_msg.clone());
                eprintln!("{}", error_msg);
                fail_count += 1;
                continue;
            }

            // Extract PSARC file using blocking extraction
            match extract_single_psarc_blocking(psarc_path, &target_dir, &progress_tx, file_idx, total_files) {
                Ok(_) => {
                    success_count += 1;
                    eprintln!("Successfully extracted {} to {}", psarc_path.display(), folder_name);
                }
                Err(e) => {
                    let error_msg = format!("Failed to extract {}: {}", psarc_path.display(), e);
                    errors.push(error_msg.clone());
                    eprintln!("{}", error_msg);
                    fail_count += 1;
                }
            }
        }

        // Send completion notification
        let message = if fail_count == 0 {
            format!("Extraction completed successfully! {} file(s) extracted.", success_count)
        } else {
            format!("Extraction completed with errors: {} succeeded, {} failed", success_count, fail_count)
        };
        
        let _ = progress_tx.send((1.0, "Completed".to_string()));
        let _ = tx.send((fail_count == 0, message, errors));
    });

    // Store receiver for checking completion
    state.init_game_extraction_receiver = Some(rx);
    state.toasts.info("Extraction started...");
    ctx.request_repaint();
}

fn extract_single_psarc_blocking(
    psarc_path: &PathBuf, 
    output_dir: &PathBuf,
    progress_tx: &crossbeam_channel::Sender<(f32, String)>,
    file_idx: usize,
    total_files: usize,
) -> Result<(), String> {
    use crate::psarc::extract_psarc;
    use std::sync::mpsc;
    
    let (tx, rx) = mpsc::channel();
    let psarc_clone = psarc_path.clone();
    let output_clone = output_dir.clone();
    let progress_tx_clone = progress_tx.clone();
    let file_name = psarc_path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    // Start extraction (runs in its own thread)
    // Note: extract_psarc spawns a thread and returns Ok(()) immediately
    // The actual extraction status is communicated via the callback
    #[allow(unused_must_use)]
    let _ = extract_psarc(&psarc_clone, &output_clone, move |status| {
        // Update progress based on extraction status
        let base_progress = (file_idx as f32) / (total_files as f32);
        let file_progress = status.progress;
        let total_progress = base_progress + (file_progress / (total_files as f32));
        
        let current_file = if !status.current_file.is_empty() {
            status.current_file.clone()
        } else {
            file_name.clone()
        };
        
        let _ = progress_tx_clone.send((total_progress, current_file));
        let _ = tx.send(status);
    });

    // Wait for completion
    loop {
        match rx.recv() {
            Ok(status) => {
                if !status.is_extracting {
                    if let Some(error) = status.error {
                        return Err(error);
                    }
                    return Ok(());
                }
            }
            Err(_) => {
                return Err("Channel closed unexpectedly".to_string());
            }
        }
    }
}

