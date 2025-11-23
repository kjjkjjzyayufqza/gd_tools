use egui::{TopBottomPanel, Ui};
use super::app_state::AppState;

/// Renders the top navigation bar.
pub fn show(ctx: &egui::Context, state: &mut AppState) {
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
        if ui.button("Save Project").clicked() {
             state.status_message = "Project Saved.".to_owned();
            ui.close();
        }
        if ui.button("Exit").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });

    ui.menu_button("View", |ui| {
        if ui.checkbox(&mut state.left_panel_visible, "Toggle Left Panel").clicked() {
            ui.close();
        }
        if ui.checkbox(&mut state.right_panel_visible, "Toggle Right Panel").clicked() {
            ui.close();
        }
        if ui.checkbox(&mut state.show_popup, "Toggle Floating Window").clicked() {
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
        if ui.checkbox(&mut state.show_popup, "Open Floating Window").clicked() {
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
    ui.label(format!("Status: {}", state.status_message));
    ui.separator();
    if ui.button("Refresh").clicked() {
        state.status_message = "Refreshing file list...".to_owned();
    }
    if ui.button("Reset Camera").clicked() {
        state.status_message = "Camera reset.".to_owned();
    }
}
