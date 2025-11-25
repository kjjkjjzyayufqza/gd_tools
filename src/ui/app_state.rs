use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use std::collections::{HashMap, HashSet};
use egui_notify::{Toasts, Anchor};
use egui;
use flate2;

/// Optimized tree node for file tree rendering
#[derive(Debug, Clone)]
pub struct CachedTreeNode {
    pub name: String,
    pub full_path: String,
    pub children: Vec<CachedTreeNode>,
    pub is_file: bool,
    /// Cached count of total files in this subtree (for display)
    pub file_count: usize,
}

/// Flattened tree item for virtual scrolling
#[derive(Debug, Clone)]
pub struct FlatTreeItem {
    pub name: String,
    pub full_path: String,
    pub is_file: bool,
    pub depth: usize,
    /// For folders: number of children that would be visible if expanded
    pub child_count: usize,
    /// Whether this item has children (for folder expand arrow)
    pub has_children: bool,
}
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
    pub initial_file_timestamps: HashMap<PathBuf, SystemTime>,
    // Files that have been modified since folder was opened (relative paths)
    #[serde(skip)]
    pub modified_files: HashSet<PathBuf>,

    // Notification system
    #[serde(skip)]
    pub toasts: Toasts,

    // Cached file tree for performance optimization
    #[serde(skip)]
    pub cached_tree: Option<CachedTreeNode>,
    /// Hash of loaded_files to detect changes
    #[serde(skip)]
    pub loaded_files_hash: u64,
    /// Cached flat list for virtual scrolling (only visible items)
    #[serde(skip)]
    pub flat_tree_cache: Vec<FlatTreeItem>,
    /// Hash to detect if flat tree needs rebuild
    #[serde(skip)]
    pub flat_tree_hash: u64,
    /// Cached set of folders with modified children
    #[serde(skip)]
    pub folders_with_modified: HashSet<String>,
    /// Version counter for modified_files to detect changes
    #[serde(skip)]
    pub modified_files_version: u64,
    /// Last version of modified_files used to compute folders_with_modified
    #[serde(skip)]
    pub folders_with_modified_version: u64,
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
            initial_file_timestamps: HashMap::new(),
            modified_files: HashSet::new(),
            toasts: Toasts::default()
                .with_anchor(Anchor::TopRight)
                .with_margin(egui::vec2(10.0, 40.0)),
            cached_tree: None,
            loaded_files_hash: 0,
            flat_tree_cache: Vec::new(),
            flat_tree_hash: 0,
            folders_with_modified: HashSet::new(),
            modified_files_version: 0,
            folders_with_modified_version: 0,
        }
    }
}

impl AppState {
    /// Mark that modified_files has changed
    pub fn bump_modified_files_version(&mut self) {
        self.modified_files_version = self.modified_files_version.wrapping_add(1);
    }
}

impl AppState {
    /// Invalidate the tree cache (call when files are loaded/changed)
    pub fn invalidate_tree_cache(&mut self) {
        self.cached_tree = None;
        self.flat_tree_cache.clear();
        self.flat_tree_hash = 0;
        self.folders_with_modified.clear();
    }

    /// Compute a simple hash of loaded files for change detection
    pub fn compute_files_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.loaded_files.len().hash(&mut hasher);
        // Only hash first and last few paths for performance
        for path in self.loaded_files.iter().take(10) {
            path.hash(&mut hasher);
        }
        if self.loaded_files.len() > 20 {
            for path in self.loaded_files.iter().skip(self.loaded_files.len() - 10) {
                path.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Compute hash for flat tree cache invalidation
    pub fn compute_flat_tree_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.loaded_files_hash.hash(&mut hasher);
        self.expanded_folders.len().hash(&mut hasher);
        for folder in &self.expanded_folders {
            folder.hash(&mut hasher);
        }
        self.search_query.hash(&mut hasher);
        hasher.finish()
    }
}
