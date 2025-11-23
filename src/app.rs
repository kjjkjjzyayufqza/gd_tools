use crate::ui::{
    app_state::AppState,
    center_panel,
    floating_window,
    left_panel,
    right_panel,
    top_panel,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    state: AppState,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Top Navigation Bar
        top_panel::show(ctx, &mut self.state);

        // 2. Left File Browser
        left_panel::show(ctx, &mut self.state);

        // 3. Right Info Panel
        right_panel::show(ctx, &mut self.state);

        // 4. Center Preview Area (Must be called last for CentralPanel)
        center_panel::show(ctx, &mut self.state);

        // 5. Floating Window
        floating_window::show(ctx, &mut self.state);
    }
}
