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
    /// Custom width for left panel (None means use default)
    pub left_panel_width: Option<f32>,
    /// Custom width for right panel (None means use default)
    pub right_panel_width: Option<f32>,
    /// Layout version for forcing panel recreation on reset
    pub layout_version: u32,

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

    // PSARC Extraction State
    #[serde(skip)]
    pub is_extracting: bool,
    #[serde(skip)]
    pub extract_progress: f32,
    #[serde(skip)]
    pub extract_status_receiver: Option<crossbeam_channel::Receiver<crate::psarc::ExtractionStatus>>,


    // Tree view expanded state
    #[serde(skip)]
    pub expanded_folders: std::collections::HashSet<String>,

    // Search filter
    #[serde(skip)]
    pub search_query: String,

    // File system watcher
    #[serde(skip)]
    pub file_watcher: Option<Box<dyn notify::Watcher>>,
    #[serde(skip)]
    pub file_events_receiver: Option<crossbeam_channel::Receiver<notify::Result<notify::Event>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            show_popup: false,
            left_panel_visible: true,
            right_panel_visible: true,
            left_panel_width: None,
            right_panel_width: None,
            layout_version: 0,
            selected_file: None,
            status_message: "Ready".to_owned(),
            current_root_dir: None,
            loaded_files: Vec::new(),
            is_packing: false,
            pack_progress: 0.0,
            pack_status_receiver: None,
            is_extracting: false,
            extract_progress: 0.0,
            extract_status_receiver: None,
            expanded_folders: std::collections::HashSet::new(),
            search_query: String::new(),
            file_watcher: None,
            file_events_receiver: None,
        }
    }
}
