use super::app_state::AppState;
use egui::{CollapsingHeader, ScrollArea, SidePanel, TextEdit};
use std::collections::HashMap;
use std::path::PathBuf;

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
            render_filters(ui, state);
            ui.separator();
            render_file_list(ui, state);
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

fn render_file_list(ui: &mut egui::Ui, state: &mut AppState) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if state.loaded_files.is_empty() {
                ui.label("No files loaded. Open a folder to start.");
            } else {
                if let Some(root) = &state.current_root_dir {
                    ui.label(format!(
                        "📂 {}",
                        root.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }

                let tree = build_file_tree(&state.loaded_files);
                render_tree_node(ui, state, &tree, "");
            }
        });
}

#[derive(Debug, Clone)]
struct TreeNode {
    name: String,
    full_path: String,
    children: HashMap<String, TreeNode>,
    is_file: bool,
}

fn build_file_tree(files: &[PathBuf]) -> TreeNode {
    let mut root = TreeNode {
        name: String::new(),
        full_path: String::new(),
        children: HashMap::new(),
        is_file: false,
    };

    for file_path in files {
        let components: Vec<&str> = file_path
            .iter()
            .filter_map(|c| c.to_str())
            .collect();

        if !components.is_empty() {
            root.insert_path(&components, 0);
        }
    }

    root
}

impl TreeNode {
    fn insert_path(&mut self, components: &[&str], index: usize) {
        if index >= components.len() {
            return;
        }

        let component = components[index].to_string();
        let is_file = index == components.len() - 1;
        let path_parts: Vec<&str> = components[..=index].iter().copied().collect();
        let full_path = path_parts.join("/");

        if !self.children.contains_key(&component) {
            self.children.insert(
                component.clone(),
                TreeNode {
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

fn matches_search(node: &TreeNode, search_query: &str) -> bool {
    if search_query.is_empty() {
        return true;
    }

    let query_lower = search_query.to_lowercase();
    node.name.to_lowercase().contains(&query_lower)
        || node.full_path.to_lowercase().contains(&query_lower)
}

fn has_matching_children(node: &TreeNode, search_query: &str) -> bool {
    if search_query.is_empty() {
        return true;
    }

    for child in node.children.values() {
        if matches_search(child, search_query) {
            return true;
        }
        if !child.is_file && has_matching_children(child, search_query) {
            return true;
        }
    }

    false
}

fn render_tree_node(
    ui: &mut egui::Ui,
    state: &mut AppState,
    node: &TreeNode,
    indent: &str,
) {
    let search_query = state.search_query.trim();
    let should_show = matches_search(node, search_query) || has_matching_children(node, search_query);

    if node.name.is_empty() {
        // Root node - render all children
        let mut sorted_children: Vec<_> = node.children.values().collect();
        sorted_children.sort_by(|a, b| {
            match (a.is_file, b.is_file) {
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                _ => a.name.cmp(&b.name),
            }
        });

        for child in sorted_children {
            render_tree_node(ui, state, child, indent);
        }
        return;
    }

    if !should_show {
        return;
    }

    if node.is_file {
        // Render file
        let display_name = format!("{}📄 {}", indent, node.name);
        let is_selected = state
            .selected_file
            .as_deref()
            .map(|s| s == node.full_path)
            .unwrap_or(false);

        let response = ui
            .selectable_label(is_selected, display_name)
            .on_hover_text(&node.full_path);

        if response.clicked() {
            state.selected_file = Some(node.full_path.clone());
        }
    } else {
        // Render folder
        let folder_key = node.full_path.clone();
        let has_matches = has_matching_children(node, search_query);
        let should_expand = has_matches && !search_query.is_empty();
        let was_expanded = state.expanded_folders.contains(&folder_key) || should_expand;

        // Auto-expand folders containing matches when searching
        if should_expand && !state.expanded_folders.contains(&folder_key) {
            state.expanded_folders.insert(folder_key.clone());
        }

        let header_response = CollapsingHeader::new(format!("{}📂 {}", indent, node.name))
            .id_salt(format!("folder_{}", folder_key))
            .default_open(was_expanded)
            .show(ui, |ui| {
                let mut sorted_children: Vec<_> = node.children.values().collect();
                sorted_children.sort_by(|a, b| {
                    match (a.is_file, b.is_file) {
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        _ => a.name.cmp(&b.name),
                    }
                });

                let new_indent = format!("{}  ", indent);
                for child in sorted_children {
                    render_tree_node(ui, state, child, &new_indent);
                }
            });

        // Update state when header is clicked
        if header_response.header_response.clicked() {
            if was_expanded {
                state.expanded_folders.remove(&folder_key);
            } else {
                state.expanded_folders.insert(folder_key);
            }
        }
    }
}
