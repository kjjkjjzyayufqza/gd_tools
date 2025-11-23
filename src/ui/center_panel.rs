use egui::{CentralPanel, Ui};
use super::app_state::AppState;

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    CentralPanel::default().show(ctx, |ui| {
        // The central panel fills the remaining space
        let rect = ui.available_rect_before_wrap();
        render_3d_viewport(ui, rect, state);
        render_hud(ui, rect, state);
    });
}

fn render_3d_viewport(ui: &mut Ui, rect: egui::Rect, _state: &mut AppState) {
    // This is where the 3D renderer would hook in.
    // For now, we just draw a placeholder.
    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(30));
    
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.heading("3D Viewport Placeholder");
                ui.label("Use your mouse to rotate/pan (mock)");
            });
        });
    });
}

fn render_hud(ui: &mut Ui, rect: egui::Rect, _state: &mut AppState) {
    // Overlay controls
    // We use a fixed position relative to the central panel
    let hud_pos = rect.min + egui::vec2(10.0, 10.0);
    let hud_rect = egui::Rect::from_min_size(hud_pos, egui::vec2(rect.width() - 20.0, 40.0));
    
    ui.scope_builder(egui::UiBuilder::new().max_rect(hud_rect), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Play").clicked() {}
            if ui.button("Pause").clicked() {}
            if ui.button("Reset Camera").clicked() {}
            ui.checkbox(&mut false, "Grid");
        });
    });
}

