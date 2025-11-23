use serde::{Deserialize, Serialize};

/// Application state that can be persisted and shared across components.
#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct AppState {
    /// Whether the floating window is visible.
    pub show_popup: bool,
    /// Whether the left file browser panel is visible.
    pub left_panel_visible: bool,
    /// Whether the right info panel is visible.
    pub right_panel_visible: bool,
    
    // Runtime-only state (skipped during serialization)
    #[serde(skip)]
    pub selected_file: Option<String>,
    #[serde(skip)]
    pub status_message: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            show_popup: false,
            left_panel_visible: true,
            right_panel_visible: true,
            selected_file: None,
            status_message: "Ready".to_owned(),
        }
    }
}

