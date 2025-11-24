use serde::{Deserialize, Serialize};
use std::path::PathBuf;
// use std::sync::{Arc, Mutex}; // Unused for now

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

    // PSARC Packing State
    #[serde(skip)]
    pub current_root_dir: Option<PathBuf>,
    #[serde(skip)]
    pub loaded_files: Vec<PathBuf>,
    #[serde(skip)]
    pub is_packing: bool,
    #[serde(skip)]
    pub pack_progress: f32,

    // Thread-safe communication for packing status updates
    #[serde(skip)]
    pub pack_status_receiver: Option<crossbeam_channel::Receiver<crate::psarc::PackingStatus>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            show_popup: false,
            left_panel_visible: true,
            right_panel_visible: true,
            selected_file: None,
            status_message: "Ready".to_owned(),
            current_root_dir: None,
            loaded_files: Vec::new(),
            is_packing: false,
            pack_progress: 0.0,
            pack_status_receiver: None,
        }
    }
}
