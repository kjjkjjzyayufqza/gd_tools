use super::app_state::{AppState, CachedTreeNode, FlatTreeItem};
use egui::{ScrollArea, SidePanel, TextEdit, Color32, RichText};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// Modified indicator - yellow/orange dot like VS Code
const MODIFIED_INDICATOR: &str = " ●";
const MODIFIED_COLOR: Color32 = Color32::from_rgb(255, 193, 7); // Amber/Orange yellow
const ROW_HEIGHT: f32 = 20.0; // Approximate height per row for virtual scrolling

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.left_panel_visible {
        return;
    }

    let mut panel = SidePanel::left(format!("left_panel_{}", state.layout_version))
        .resizable(true)
        .min_width(150.0)
        .max_width(1000.0);

    // Use custom width if set, otherwise use default
    if let Some(width) = state.left_panel_width {
        panel = panel.default_width(width);
    } else {
        panel = panel.default_width(250.0);
    }

    panel
        .show(ctx, |ui| {
            render_filters(ui, state);
            ui.separator();
            render_file_list_optimized(ui, state);
        });
}

fn render_filters(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(&mut state.search_query).hint_text("Search..."));
        // Placeholder for extension filter
        egui::ComboBox::from_id_salt("ext_filter")
            .selected_text("All")
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut 0, 0, "All");
            });
    });
}

/// Optimized file list rendering with caching and virtual scrolling
fn render_file_list_optimized(ui: &mut egui::Ui, state: &mut AppState) {
    if state.loaded_files.is_empty() {
        ui.label("No files loaded. Open a folder to start.");
        return;
    }

    // Show root folder name
    if let Some(root) = &state.current_root_dir {
        ui.label(format!(
            "📂 {} ({} files)",
            root.file_name().unwrap_or_default().to_string_lossy(),
            state.loaded_files.len()
        ));
    }

    // Ensure tree is built and cached
    ensure_tree_cached(state);

    // Build flat list of visible items for virtual scrolling
    let flat_items = build_visible_flat_list(state);
    let total_items = flat_items.len();

    if total_items == 0 {
        ui.label("No matching files found.");
        return;
    }

    // Clone necessary state to avoid borrow issues
    let modified_files = state.modified_files.clone();
    let folders_with_modified = state.folders_with_modified.clone();
    let expanded_folders = state.expanded_folders.clone();
    let selected_file = state.selected_file.clone();
    let search_query = state.search_query.clone();

    // Collect UI actions to apply after rendering
    let mut new_selected_file: Option<String> = None;
    let mut folders_to_toggle: Vec<String> = Vec::new();

    // Virtual scrolling with fixed row height
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, total_items, |ui, row_range| {
            for row_idx in row_range {
                if let Some(item) = flat_items.get(row_idx) {
                    let indent = "  ".repeat(item.depth);
                    
                    if item.is_file {
                        // Render file
                        let is_modified = is_file_modified(&item.full_path, &modified_files);
                        let is_selected = selected_file
                            .as_deref()
                            .map(|s| s == item.full_path)
                            .unwrap_or(false);

                        ui.horizontal(|ui| {
                            let display_name = format!("{}📄 {}", indent, item.name);
                            let response = ui
                                .selectable_label(is_selected, display_name)
                                .on_hover_text(if is_modified {
                                    format!("{} (Modified)", &item.full_path)
                                } else {
                                    item.full_path.clone()
                                });

                            if response.clicked() {
                                new_selected_file = Some(item.full_path.clone());
                            }

                            if is_modified {
                                ui.label(RichText::new(MODIFIED_INDICATOR).color(MODIFIED_COLOR).strong());
                            }
                        });
                    } else {
                        // Render folder
                        let is_expanded = expanded_folders.contains(&item.full_path);
                        let has_modified = folders_with_modified.contains(&item.full_path);

                        let arrow = if is_expanded { "▼" } else { "▶" };
                        let folder_icon = "📂";
                        
                        let display_text = if has_modified {
                            format!("{}{} {} {}{}", indent, arrow, folder_icon, item.name, MODIFIED_INDICATOR)
                        } else {
                            format!("{}{} {} {}", indent, arrow, folder_icon, item.name)
                        };

                        // Add child count for collapsed folders with many items
                        let display_with_count = if !is_expanded && item.child_count > 0 {
                            format!("{} ({})", display_text, item.child_count)
                        } else {
                            display_text
                        };

                        let response = ui.selectable_label(false, display_with_count);
                        
                        if response.clicked() {
                            folders_to_toggle.push(item.full_path.clone());
                        }
                    }
                }
            }
        });

    // Apply collected UI actions
    if let Some(selected) = new_selected_file {
        state.selected_file = Some(selected);
    }

    for folder in folders_to_toggle {
        if state.expanded_folders.contains(&folder) {
            state.expanded_folders.remove(&folder);
        } else {
            state.expanded_folders.insert(folder);

            // Auto-expand when searching
            if !search_query.is_empty() {
                // Already expanded
            }
        }
    }
}

/// Ensure the tree is built and cached
fn ensure_tree_cached(state: &mut AppState) {
    let current_hash = state.compute_files_hash();
    
    if state.cached_tree.is_none() || state.loaded_files_hash != current_hash {
        // Rebuild tree
        state.cached_tree = Some(build_cached_tree(&state.loaded_files));
        state.loaded_files_hash = current_hash;
        
        // Force rebuild of folders_with_modified
        state.folders_with_modified_version = 0;
    }
    
    // Update modified folders cache when modified_files changes
    if state.folders_with_modified_version != state.modified_files_version {
        if let Some(tree) = &state.cached_tree {
            state.folders_with_modified = compute_folders_with_modified(tree, &state.modified_files);
            state.folders_with_modified_version = state.modified_files_version;
        }
    }
}

/// Build the cached tree structure from file paths
fn build_cached_tree(files: &[PathBuf]) -> CachedTreeNode {
    // Use a temporary HashMap for building, then convert to sorted Vec
    let mut temp_root = TempTreeNode {
        name: String::new(),
        full_path: String::new(),
        children: HashMap::new(),
        is_file: false,
    };

    for file_path in files {
        let components: Vec<String> = file_path
            .iter()
            .map(|c| c.to_string_lossy().to_string())
            .collect();

        if !components.is_empty() {
            temp_root.insert_path(&components, 0);
        }
    }

    // Convert to cached tree with sorted children
    convert_to_cached_tree(&temp_root)
}

/// Temporary tree node for building (uses HashMap for O(1) insertion)
struct TempTreeNode {
    name: String,
    full_path: String,
    children: HashMap<String, TempTreeNode>,
    is_file: bool,
}

impl TempTreeNode {
    fn insert_path(&mut self, components: &[String], index: usize) {
        if index >= components.len() {
            return;
        }

        let component = components[index].clone();
        let is_file = index == components.len() - 1;
        let path_parts: Vec<&str> = components[..=index].iter().map(|s| s.as_str()).collect();
        let full_path = path_parts.join("/");

        if !self.children.contains_key(&component) {
            self.children.insert(
                component.clone(),
                TempTreeNode {
                    name: component.clone(),
                    full_path: full_path.clone(),
                    children: HashMap::new(),
                    is_file,
                },
            );
        }

        if !is_file {
            if let Some(child) = self.children.get_mut(&component) {
                child.insert_path(components, index + 1);
            }
        }
    }
}

/// Convert temporary tree to cached tree with sorted children and file counts
fn convert_to_cached_tree(temp: &TempTreeNode) -> CachedTreeNode {
    let mut children: Vec<CachedTreeNode> = temp.children
        .values()
        .map(convert_to_cached_tree)
        .collect();

    // Sort: folders first, then files, alphabetically within each group
    children.sort_by(|a, b| {
        match (a.is_file, b.is_file) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.name.cmp(&b.name),
        }
    });

    // Calculate file count
    let file_count = if temp.is_file {
        1
    } else {
        children.iter().map(|c| c.file_count).sum()
    };

    CachedTreeNode {
        name: temp.name.clone(),
        full_path: temp.full_path.clone(),
        children,
        is_file: temp.is_file,
        file_count,
    }
}

/// Compute set of folder paths that contain modified files
fn compute_folders_with_modified(tree: &CachedTreeNode, modified_files: &HashSet<PathBuf>) -> HashSet<String> {
    let mut result = HashSet::new();
    compute_folders_with_modified_recursive(tree, modified_files, &mut result);
    result
}

fn compute_folders_with_modified_recursive(
    node: &CachedTreeNode,
    modified_files: &HashSet<PathBuf>,
    result: &mut HashSet<String>,
) -> bool {
    if node.is_file {
        let path = PathBuf::from(&node.full_path);
        return modified_files.contains(&path);
    }

    let mut has_modified = false;
    for child in &node.children {
        if compute_folders_with_modified_recursive(child, modified_files, result) {
            has_modified = true;
        }
    }

    if has_modified && !node.full_path.is_empty() {
        result.insert(node.full_path.clone());
    }

    has_modified
}

/// Build a flat list of currently visible items based on expanded folders
fn build_visible_flat_list(state: &AppState) -> Vec<FlatTreeItem> {
    let mut items = Vec::new();
    
    if let Some(tree) = &state.cached_tree {
        let search_query = state.search_query.trim();
        
        // For root node, add all children
        for child in &tree.children {
            build_flat_list_recursive(
                child,
                0,
                &state.expanded_folders,
                search_query,
                &mut items,
            );
        }
    }

    items
}

fn build_flat_list_recursive(
    node: &CachedTreeNode,
    depth: usize,
    expanded_folders: &HashSet<String>,
    search_query: &str,
    items: &mut Vec<FlatTreeItem>,
) {
    // Check if this node or any children match the search
    let matches_search = matches_search_cached(node, search_query);
    let has_matching_children = has_matching_children_cached(node, search_query);
    
    if !matches_search && !has_matching_children {
        return;
    }

    // Add this node to the flat list
    items.push(FlatTreeItem {
        name: node.name.clone(),
        full_path: node.full_path.clone(),
        is_file: node.is_file,
        depth,
        child_count: node.file_count,
        has_children: !node.children.is_empty(),
    });

    // For folders, only recurse if expanded (or if searching and has matches)
    if !node.is_file {
        let is_expanded = expanded_folders.contains(&node.full_path);
        let force_expand = !search_query.is_empty() && has_matching_children;
        
        if is_expanded || force_expand {
            for child in &node.children {
                build_flat_list_recursive(
                    child,
                    depth + 1,
                    expanded_folders,
                    search_query,
                    items,
                );
            }
        }
    }
}

fn matches_search_cached(node: &CachedTreeNode, search_query: &str) -> bool {
    if search_query.is_empty() {
        return true;
    }

    let query_lower = search_query.to_lowercase();
    node.name.to_lowercase().contains(&query_lower)
        || node.full_path.to_lowercase().contains(&query_lower)
}

fn has_matching_children_cached(node: &CachedTreeNode, search_query: &str) -> bool {
    if search_query.is_empty() {
        return true;
    }

    for child in &node.children {
        if matches_search_cached(child, search_query) {
            return true;
        }
        if !child.is_file && has_matching_children_cached(child, search_query) {
            return true;
        }
    }

    false
}

fn is_file_modified(full_path: &str, modified_files: &HashSet<PathBuf>) -> bool {
    let path = PathBuf::from(full_path);
    modified_files.contains(&path)
}


