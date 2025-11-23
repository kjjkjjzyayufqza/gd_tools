use egui::{Window, ScrollArea};
use super::app_state::AppState;

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    Window::new("Floating Window")
        .open(&mut state.show_popup)
        .resizable(true)
        .collapsible(true)
        .default_width(400.0)
        .default_height(300.0)
        .show(ctx, |ui| {
            ui.label("This is a floating utility window.");
            
            ui.separator();
            
            // Example content: Build Output
            ui.collapsing("Build Output", |ui| {
                ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                     ui.label("[INFO] Build started...");
                     ui.label("[INFO] Parsing assets...");
                     ui.colored_label(egui::Color32::YELLOW, "[WARN] Texture 'wood.png' missing mipmaps.");
                     ui.colored_label(egui::Color32::GREEN, "[SUCCESS] Build completed.");
                });
            });

             ui.separator();

             // Example content: Batch Tools
             ui.collapsing("Batch Tools", |ui| {
                 if ui.button("Batch Rename").clicked() {
                     // ...
                 }
             });
        });
}

