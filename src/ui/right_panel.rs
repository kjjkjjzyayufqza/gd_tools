use super::app_state::AppState;
use egui::{ScrollArea, SidePanel};

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.right_panel_visible {
        return;
    }

    SidePanel::right("right_panel")
        .resizable(true)
        .default_width(500.0)
        .min_width(300.0)
        .max_width(1200.0) // Allow the panel to be resized up to 1200 pixels
        .show(ctx, |ui| {
            // Ensure the scroll area fills the available space and doesn't shrink to content
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(selected) = &state.selected_file {
                        ui.heading("Asset Details");
                        ui.separator();

                        // Common Section
                        ui.collapsing("Common", |ui| {
                            ui.label(format!("Name: {}", selected));
                            ui.label("Path: /assets/models/"); // Mock path
                            ui.label("Source: Game"); // Mock source
                            ui.label("Size: 1.2 MB");
                        });

                        // Mock type-specific sections based on filename extension
                        if selected.ends_with(".obj") || selected.ends_with(".fbx") {
                            ui.collapsing("Model Info", |ui| {
                                ui.label("Vertices: 12,500");
                                ui.label("Triangles: 24,000");
                                ui.label("Materials: 2");
                            });
                        } else if selected.ends_with(".png") || selected.ends_with(".jpg") {
                            ui.collapsing("Texture Info", |ui| {
                                ui.label("Resolution: 1024x1024");
                                ui.label("Format: RGBA8");
                            });
                        }

                        // Modding Status
                        ui.collapsing("Modding Status", |ui| {
                            ui.label("Override State: Vanilla only");
                            if ui.button("Duplicate to Mod").clicked() {
                                state.status_message =
                                    format!("Duplicating {} to mod...", selected);
                            }
                        });
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.label("No asset selected.");
                            ui.label("Select a file from the left panel to view details.");
                        });
                    }
                });
        });
}
