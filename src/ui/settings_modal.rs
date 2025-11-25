use super::app_state::{AppState, CompressionLevel, PackingMode};
use egui::{ComboBox, Window};
use rfd::FileDialog;

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    let mut open = state.show_settings;
    Window::new("Settings")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .min_width(400.0)
        .show(ctx, |ui| {
            ui.heading("PSARC Settings");
            ui.separator();

            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([40.0, 10.0])
                .striped(true)
                .show(ui, |ui| {
                    // Game Folder (Output Directory)
                    ui.label("Game Folder:");
                    ui.horizontal(|ui| {
                        let folder_text = state.game_folder
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Not Set".to_string());
                        
                        // Truncate long paths for display
                        let display_text = if folder_text.len() > 40 {
                            format!("...{}", &folder_text[folder_text.len() - 37..])
                        } else {
                            folder_text.clone()
                        };
                        
                        ui.label(display_text).on_hover_text(&folder_text);
                        
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                state.game_folder = Some(path);
                            }
                        }
                        
                        if state.game_folder.is_some() && ui.button("Clear").clicked() {
                            state.game_folder = None;
                        }
                    });
                    ui.end_row();

                    // Compression Level
                    ui.label("Compression Level:");
                    ComboBox::from_id_salt("compression_level")
                        .selected_text(format!("{:?}", state.compression_level))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.compression_level, CompressionLevel::None, "None");
                            ui.selectable_value(&mut state.compression_level, CompressionLevel::Fast, "Fast");
                            ui.selectable_value(&mut state.compression_level, CompressionLevel::Default, "Default");
                            ui.selectable_value(&mut state.compression_level, CompressionLevel::Best, "Best");
                        });
                    ui.end_row();

                    // Packing Mode
                    ui.label("Packing Mode:");
                    ComboBox::from_id_salt("packing_mode")
                        .selected_text(format!("{:?}", state.packing_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.packing_mode, PackingMode::Full, "Full (Repack All)");
                            ui.selectable_value(&mut state.packing_mode, PackingMode::Incremental, "Incremental (Modified Only)");
                        });
                    ui.end_row();
                });
            
            ui.separator();
            ui.add_space(5.0);
            ui.label(egui::RichText::new("Note: Game Folder is used as the output directory for PSARC packing in Incremental mode.").small().weak());
        });
    state.show_settings = open;
}

