use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use egui_notify::{Toasts, Anchor};
use egui;
use flate2;
// use std::sync::{Arc, Mutex}; // Unused for now

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum CompressionLevel {
    None,
    Fast,
    #[default]
    Best,
    Default,
}

impl CompressionLevel {
    pub fn to_flate2(&self) -> flate2::Compression {
        match self {
            CompressionLevel::None => flate2::Compression::none(),
            CompressionLevel::Fast => flate2::Compression::fast(),
            CompressionLevel::Default => flate2::Compression::default(),
            CompressionLevel::Best => flate2::Compression::best(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum PackingMode {
    #[default]
    Full,        // Repack everything
    Incremental, // Only modified files
}

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

    // Settings
    pub compression_level: CompressionLevel,
    pub packing_mode: PackingMode,
    /// Game folder path for PSARC output
    pub game_folder: Option<PathBuf>,
    #[serde(skip)]
    pub show_settings: bool,
    /// Whether the pack confirmation modal is visible
    #[serde(skip)]
    pub show_pack_confirm: bool,
    /// List of arc folders pending to be packed (e.g., ["arc_1_ep_8_11", "arc_2_ep_12_30"])
    #[serde(skip)]
    pub pending_pack_folders: Vec<String>,
    #[serde(skip)]
    pub show_init_game_dialog: bool,
    #[serde(skip)]
    pub init_game_psarc_files: [Option<PathBuf>; 5],
    #[serde(skip)]
    pub init_game_output_dir: Option<PathBuf>,
    #[serde(skip)]
    pub init_game_extraction_receiver: Option<crossbeam_channel::Receiver<(bool, String, Vec<String>)>>,
    #[serde(skip)]
    pub init_game_extraction_progress_receiver: Option<crossbeam_channel::Receiver<(f32, String)>>,
    #[serde(skip)]
    pub init_game_is_extracting: bool,
    #[serde(skip)]
    pub init_game_extraction_progress: f32,
    #[serde(skip)]
    pub init_game_current_file: String,

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

    // File modification tracking - stores initial timestamps when folder is opened
    #[serde(skip)]
    pub initial_file_timestamps: std::collections::HashMap<PathBuf, SystemTime>,
    // Files that have been modified since folder was opened (relative paths)
    #[serde(skip)]
    pub modified_files: std::collections::HashSet<PathBuf>,

    // Notification system
    #[serde(skip)]
    pub toasts: Toasts,
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
            compression_level: CompressionLevel::Best,
            packing_mode: PackingMode::Full,
            game_folder: None,
            show_settings: false,
            show_pack_confirm: false,
            pending_pack_folders: Vec::new(),
            show_init_game_dialog: false,
            init_game_psarc_files: [None, None, None, None, None],
            init_game_output_dir: None,
            init_game_extraction_receiver: None,
            init_game_extraction_progress_receiver: None,
            init_game_is_extracting: false,
            init_game_extraction_progress: 0.0,
            init_game_current_file: String::new(),
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
            initial_file_timestamps: std::collections::HashMap::new(),
            modified_files: std::collections::HashSet::new(),
            toasts: Toasts::default()
                .with_anchor(Anchor::TopRight)
                .with_margin(egui::vec2(10.0, 40.0)),
        }
    }
}
