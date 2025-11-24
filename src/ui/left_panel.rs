use super::app_state::AppState;
use egui::{Label, ScrollArea, Sense, SidePanel, TextEdit};

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
                render_file_list(ui, state);
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
            });
    });
}

fn render_file_list(ui: &mut egui::Ui, state: &mut AppState) {
    ScrollArea::vertical().show(ui, |ui| {
        if state.loaded_files.is_empty() {
            ui.label("No files loaded. Open a folder to start.");
        } else {
            if let Some(root) = &state.current_root_dir {
                ui.label(format!(
                    "📂 {}",
                    root.file_name().unwrap_or_default().to_string_lossy()
                ));
            }

            // Use virtual scrolling for large lists if possible, but egui's simple list is okay for now
            // For thousands of files, we might want to optimize this further.
            for path in &state.loaded_files {
                let name = path.to_string_lossy();
                let _is_selected = state.selected_file.as_deref() == Some(name.as_ref());

                if ui
                    .add(
                        Label::new(format!("📄 {}", name))
                            .sense(Sense::click())
                            .selectable(false),
                    ) // We handle selection manually
                    .clicked()
                {
                    state.selected_file = Some(name.into_owned());
                }
            }
        }
    });
}
