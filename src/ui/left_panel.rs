use egui::{SidePanel, TextEdit, ScrollArea, CollapsingHeader, Label, Sense};
use super::app_state::AppState;

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.left_panel_visible {
        return;
    }

    SidePanel::left("left_panel")
        .resizable(true)
        .default_width(250.0)
        .min_width(150.0)
        .max_width(1000.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                render_filters(ui);
                ui.separator();
                render_file_tree(ui, state);
            });
        });
}

fn render_filters(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(&mut String::new()).hint_text("Search..."));
        // Placeholder for extension filter
        egui::ComboBox::from_id_salt("ext_filter")
            .selected_text("All")
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut 0, 0, "All");
                ui.selectable_value(&mut 1, 1, "Models");
                ui.selectable_value(&mut 2, 2, "Textures");
            });
    });

    ui.horizontal(|ui| {
        ui.label("Scope:");
        ui.radio_value(&mut 0, 0, "Game");
        ui.radio_value(&mut 1, 1, "Mod");
        ui.radio_value(&mut 2, 2, "Combined");
    });
}

fn render_file_tree(ui: &mut egui::Ui, state: &mut AppState) {
    ScrollArea::vertical().show(ui, |ui| {
        CollapsingHeader::new("assets (Game)").default_open(true).show(ui, |ui| {
            CollapsingHeader::new("models").show(ui, |ui| {
                 file_item(ui, state, "player.obj", "Game");
                 file_item(ui, state, "enemy.fbx", "Game");
            });
            CollapsingHeader::new("textures").show(ui, |ui| {
                file_item(ui, state, "wood.png", "Game");
                file_item(ui, state, "metal.png", "Game");
            });
        });

        CollapsingHeader::new("mod_assets (My Mod)").default_open(true).show(ui, |ui| {
             CollapsingHeader::new("models").show(ui, |ui| {
                 file_item(ui, state, "custom_sword.obj", "Mod");
            });
        });
    });
}

fn file_item(ui: &mut egui::Ui, state: &mut AppState, name: &str, source: &str) {
    let _is_selected = state.selected_file.as_deref() == Some(name);
    let label_text = format!("📄 {} ({})", name, source);
    
    if ui.add(Label::new(label_text).sense(Sense::click()).selectable(false)).clicked() {
        state.selected_file = Some(name.to_owned());
    }
    
    // Use selectable_label for better selection visualization if preferred
    // if ui.selectable_label(is_selected, label_text).clicked() {
    //      state.selected_file = Some(name.to_owned());
    // }
}

