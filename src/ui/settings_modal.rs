use super::app_state::{AppState, CompressionLevel, PackingMode};
use egui::{ComboBox, Window};

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    let mut open = state.show_settings;
    Window::new("Settings")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("PSARC Settings");
            ui.separator();

            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([40.0, 10.0])
                .striped(true)
                .show(ui, |ui| {
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
        });
    state.show_settings = open;
}

