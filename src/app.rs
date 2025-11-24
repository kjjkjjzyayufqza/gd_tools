use crate::ui::{
    app_state::AppState, center_panel, floating_window, left_panel, right_panel, top_panel,
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
        // Configure fonts to support Chinese characters
        setup_custom_fonts(&cc.egui_ctx);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

/// Setup custom fonts with Chinese character support
fn setup_custom_fonts(ctx: &egui::Context) {
    use egui::FontFamily;

    let mut fonts = egui::FontDefinitions::default();

    // Try to load system fonts that support Chinese characters
    #[cfg(target_os = "windows")]
    {
        // Try common Windows Chinese fonts
        let chinese_fonts = [
            "Microsoft YaHei",
            "SimSun",
            "SimHei",
            "KaiTi",
            "FangSong",
        ];

        for font_name in &chinese_fonts {
            if let Some(font_data) = load_system_font(font_name) {
                fonts.font_data.insert(
                    "chinese_font".to_owned(),
                    std::sync::Arc::new(font_data),
                );
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "chinese_font".to_owned());
                fonts
                    .families
                    .get_mut(&FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "chinese_font".to_owned());
                break;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let chinese_fonts = ["PingFang SC", "STHeiti", "STSong", "Arial Unicode MS"];

        for font_name in &chinese_fonts {
            if let Some(font_data) = load_system_font(font_name) {
                fonts.font_data.insert(
                    "chinese_font".to_owned(),
                    std::sync::Arc::new(font_data),
                );
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "chinese_font".to_owned());
                fonts
                    .families
                    .get_mut(&FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "chinese_font".to_owned());
                break;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let chinese_fonts = [
            "Noto Sans CJK SC",
            "WenQuanYi Micro Hei",
            "WenQuanYi Zen Hei",
            "AR PL UMing CN",
        ];

        for font_name in &chinese_fonts {
            if let Some(font_data) = load_system_font(font_name) {
                fonts.font_data.insert(
                    "chinese_font".to_owned(),
                    std::sync::Arc::new(font_data),
                );
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "chinese_font".to_owned());
                fonts
                    .families
                    .get_mut(&FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "chinese_font".to_owned());
                break;
            }
        }
    }

    ctx.set_fonts(fonts);
}

/// Load a system font by name
fn load_system_font(font_name: &str) -> Option<egui::FontData> {
    #[cfg(target_os = "windows")]
    {
        use std::path::PathBuf;
        let system_fonts_dir = PathBuf::from("C:\\Windows\\Fonts");
        
        // Windows font file names mapping
        let font_file_mapping: std::collections::HashMap<&str, &[&str]> = [
            ("Microsoft YaHei", &["msyh.ttc", "msyhbd.ttc", "msyhl.ttc"] as &[&str]),
            ("SimSun", &["simsun.ttc", "simsun.ttf"] as &[&str]),
            ("SimHei", &["simhei.ttf"] as &[&str]),
            ("KaiTi", &["simkai.ttf"] as &[&str]),
            ("FangSong", &["simfang.ttf"] as &[&str]),
        ]
        .iter()
        .cloned()
        .collect();

        // Try mapped file names first
        if let Some(file_names) = font_file_mapping.get(font_name) {
            for file_name in *file_names {
                let path = system_fonts_dir.join(file_name);
                if path.exists() {
                    if let Ok(font_bytes) = std::fs::read(&path) {
                        return Some(egui::FontData::from_owned(font_bytes));
                    }
                }
            }
        }

        // Fallback: try direct font name
        let font_paths = [
            system_fonts_dir.join(format!("{}.ttf", font_name)),
            system_fonts_dir.join(format!("{}.ttc", font_name)),
            system_fonts_dir.join(format!("{}.otf", font_name)),
        ];

        for path in &font_paths {
            if path.exists() {
                if let Ok(font_bytes) = std::fs::read(path) {
                    return Some(egui::FontData::from_owned(font_bytes));
                }
            }
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // For macOS and Linux, try to use fontconfig or system font loading
        // This is a simplified version - in production you might want to use
        // a font loading library like fontdb or font-kit
        if let Ok(font_bytes) = load_font_via_system(font_name) {
            return Some(egui::FontData::from_owned(font_bytes));
        }
    }

    None
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn load_font_via_system(_font_name: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Placeholder - would need font loading library for proper implementation
    Err("Font loading not implemented for this platform".into())
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
