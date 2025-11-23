# Egui UI Design Specification

## 1. Overall Layout Overview

The application will use a classic "Docking" or "Three-Column" layout, typical for content creation and management tools. This structure maximizes the central workspace while keeping management and inspection tools readily available.  
For a **modding tool**, this layout emphasizes:

*   Quick navigation across game and mod assets.
*   Immediate visual feedback for models, textures, and other resources.
*   Clear visibility of metadata, conflicts, and overrides.

### Egui Panel Structure
We will utilize the following `egui` containers to achieve this:

1.  **Top Navigation Bar**: `egui::TopBottomPanel::top("top_nav")`
    *   **Role**: Hosts global commands, menus, and state toggles for the whole modding workspace.
    *   **Height**: Fixed, minimal height (approx. 24-30pt).

2.  **Left File Browser**: `egui::SidePanel::left("left_panel")`
    *   **Role**: File navigation and selection (both game assets and mod files).
    *   **Width**: Resizable, default approx. 250-300pt.
    *   **Behavior**: Collapsible if needed, contains the filter and tree view.
    *   **Modding Focus**: Clearly distinguishes vanilla files vs. overridden/added files by the mod.

3.  **Right Info Panel**: `egui::SidePanel::right("right_panel")`
    *   **Role**: Context-sensitive information based on the left panel's selection.
    *   **Width**: Resizable, default approx. 300pt.
    *   **Behavior**: Collapsible.
    *   **Modding Focus**: Shows detailed properties, references in game data, conflicts, and mod-specific overrides for the selected item.

4.  **Center Preview Area**: `egui::CentralPanel::default()`
    *   **Role**: The main workspace for 3D visualization and other previews.
    *   **Size**: Automatically fills the remaining space between the left and right panels.
    *   **Modding Focus**: Displays previews of the currently selected asset (models, animations, textures, UI layouts, etc.).

5.  **Floating Window**: `egui::Window::new("Floating Window")`
    *   **Role**: A modal or modeless popup for auxiliary tasks (logs, custom tools, batch operations).
    *   **Behavior**: Draggable, resizable, togglable via the Top Nav.

---

## 2. Top Navigation Bar Design

The top bar serves as the global command center for the modding workflow.

### Layout Strategy
*   **Container**: `egui::menu::bar` inside the top panel.
*   **Alignment**: Left-aligned for menus, Right-aligned for global status/tools.

### Content
1.  **Menu Area (Left)**:
    *   **File**:
        *   `Open Game Install...` – Select the base game directory.
        *   `Open Mod Project...` – Open an existing mod project.
        *   `New Mod Project...` – Create a new mod in a chosen directory.
        *   `Save Project` / `Save All` – Persist project configuration or metadata.
        *   `Recent Projects` – Quick list for recently opened mod projects.
        *   `Exit` – Close the tool.
    *   **View**:
        *   `Toggle Left Panel` – Show/hide the file browser.
        *   `Toggle Right Panel` – Show/hide the info panel.
        *   `Toggle Floating Window` – Show/hide floating utility window.
        *   `Reset Layout` – Restore default panel sizes and docking layout.
        *   (Optional) `Theme` – Switch between light/dark or game-themed color schemes.
    *   **Mods** (modding-specific menu):
        *   `Build Mod` – Pack assets/scripts into the game's expected mod format.
        *   `Run In Game` – Launch the game with current mod enabled (if supported).
        *   `Validate` – Run static checks (missing references, invalid paths, version mismatches).
        *   `Manage Load Order` – Open a dedicated dialog or use the floating window to reorder mods.
    *   **Tools**:
        *   `Open Floating Window` – Checkbox/toggle controlling `show_popup`.
        *   `Batch Operations` – e.g., batch import, batch rename, batch re-path textures.
        *   `Settings` – Paths (game install, temp folder), language, backup options.
    *   **Help**:
        *   `View Docs` – Open documentation for the tool.
        *   `Check Game Version Compatibility` – Show supported game versions.
        *   `About` – Version information, license.

2.  **Toolbar Area (Middle/Right)**:
    *   **Quick Actions**:
        *   `Refresh File List` – Rescan game and mod directories.
        *   `Reset Camera` – Reset the 3D view to default position.
    *   **Status Indicator** (optional):
        *   Can show the currently active mod project name and short path (e.g., `my_mod (E:/Games/MyMod)`).
        *   Can show build and validation summary (e.g., last build result, number of warnings).
        *   Can show current game version and mod target version (e.g., `Game v1.2.3 / Target v1.2`).

### Interaction
*   Clicking "Open Floating Window" toggles a boolean state `show_popup` which controls the visibility of the core floating component.
*   When a long-running action is triggered (build, validation), the status indicator changes to a "working" state and may show a small spinner icon.
*   Disabled menu items indicate unavailable actions (e.g., `Build Mod` is disabled until a mod project is open).

---

## 3. Left File Browser Panel Design

This panel mimics a standard OS file explorer but with specialized filtering for game and modding workflows.

### Layout Strategy
*   **Container**: Vertical layout (`ui.vertical`).
*   **Top Area**: Filters and scope selection (game vs. mod).
*   **Body Area**: Tree view for directories and files.

### Section A: Filters (Top of Panel)
*   **Unified Filter Row**:
    *   Layout: Horizontal (`ui.horizontal`).
    *   **Name Filter**: `egui::TextEdit::singleline` (Takes up most width).
        *   Filters by partial name match (case-insensitive).
        *   Supports simple patterns such as `player_*` or `*_diffuse`.
    *   **Extension Filter**: A small `egui::ComboBox` or icon button to filter by type.
        *   Example options:
            *   `All`
            *   `Models (*.obj, *.fbx, *.gltf, ...)`
            *   `Textures (*.png, *.jpg, *.dds, ...)`
            *   `Scripts (*.lua, *.rs, *.cs, ...)`
            *   `Data (*.json, *.yml, *.xml, ...)`
        *   The filter is applied on top of the name filter.

*   **Source Scope Toggle** (optional but recommended for modding):
    *   Radio buttons or small segmented control:
        *   `Game` – Show only vanilla game files (read-only).
        *   `Mod` – Show only files inside the current mod project.
        *   `Combined` – Show a unified view where overridden files are clearly marked.
    *   In `Combined` mode:
        *   Overridden files can use a different color or icon (e.g., a small "override" badge).
        *   New files added by the mod may have a "plus" badge icon.

### Section B: Tree View (Body of Panel)
*   **Container**: `egui::ScrollArea::vertical`.
*   **Structure**:
    *   Use `egui::CollapsingHeader` for directories.
    *   Use `egui::Label` or `egui::SelectableLabel` for files.
    *   **Indentation**: Automatic via `CollapsingHeader`.
*   **Visuals**:
    *   Directory Icon: 📁 (Unicode or texture).
    *   File Icon: 📄 (Unicode or texture, or per-type icons).
    *   **Selection**: Clicking a file highlights it (changes background color) and updates the **Right Info Panel** and **Center Preview Area**.
    *   **Override State**:
        *   Vanilla file: normal color.
        *   Overridden by mod: highlight with a subtle color and/or overlay icon.
        *   Mod-only file: separate color (e.g., green-tinted text).

*   **Context Menu (Optional)**:
    *   On right-click of a file or directory:
        *   `Open in External Editor` – For scripts or text-based assets.
        *   `Reveal in Explorer` – Open OS file explorer at the path.
        *   `Duplicate to Mod` – Copy a vanilla file into the mod project for overriding.
        *   `Rename`, `Delete` (only for mod files).
        *   `Compare With Vanilla` – Open a diff view (e.g., in the floating window).

---

## 4. Center 3D Preview Area Design

This is the primary visualization space.

### Layout Strategy
*   **Container**: `egui::CentralPanel`.
*   **Background**: Dark grey or a custom gradient to contrast with the UI.

### Content
1.  **3D Viewport**:
    *   **Implementation**: Since this is an `egui` design doc, we assume an integration with a renderer (like `glow` or `wgpu`) inside an `egui::PaintCallback` or simply using `egui`'s custom painting for a wireframe box.
    *   **Placeholder Behavior**: A wireframe cube rotating automatically around the Y-axis when no previewable asset is selected.
    *   **Asset-Specific Behavior**:
        *   When a model file is selected, load and display it with simple lighting.
        *   When a texture is selected, display it as a 2D preview with zoom/pan.
        *   When non-visual assets (e.g., scripts or data) are selected, show a simple placeholder or text overlay.
    *   **Interaction**: Mouse drag to rotate camera, scroll to zoom (standard orbit controls), optional middle mouse or modifier + drag to pan.

2.  **Overlay Controls (HUD)**:
    *   Position: Top-right or bottom-left corner of the central panel (using `ui.put` or nested absolute positioning).
    *   **Controls**:
        *   `Play/Pause` button for rotation or animations.
        *   `Reset Camera` button.
        *   `Background Color` picker (small circle).
        *   `Grid Toggle` checkbox.
        *   Optional: `Wireframe` toggle, `Lighting Preset` selector.

3.  **Comparison Mode (Optional for Modding)**:
    *   When an overridden asset is selected, allow switching between:
        *   `Vanilla` view.
        *   `Modded` view.
        *   `Side-by-Side` or `Split` view (if feasible).
    *   The control can be a small segmented toggle in the HUD or in the right panel.

---

## 5. Right Info Panel Design

Displays details about the selection from the Left Panel, tailored for modding workflows.

### Layout Strategy
*   **Container**: `egui::ScrollArea::vertical`.
*   **Sections**: Stack of collapsible sections for metadata, references, and mod-specific information.

### Content Logic
The content changes dynamically based on a conceptual `SelectionType`, which can include:

*   `SelectionType::Directory`
*   `SelectionType::Model`
*   `SelectionType::Texture`
*   `SelectionType::Script`
*   `SelectionType::Data` (JSON, YAML, etc.)
*   `SelectionType::Other`

### Example Sections

1.  **Common Section (All Types)**:
    *   **Path**: Full path (game vs. mod) with a clear label.
    *   **Source**: `Game` / `Mod` / `Override (Game + Mod)`.
    *   **File Size**: Human-readable.
    *   **Last Modified**: Timestamp with relative time (e.g., `2 hours ago`).
    *   **Encoding / Format**: Basic format string (e.g., `FBX`, `PNG`, `Lua`).

2.  **Model-Specific Section**:
    *   Polygon count (if available).
    *   Number of meshes, materials.
    *   Referenced textures (clickable entries that select those textures in the left panel).
    *   Bounding box size (for quick sanity checks).

3.  **Texture-Specific Section**:
    *   Resolution (width × height).
    *   Format (e.g., `RGBA8`, `BC7`).
    *   Color space (sRGB / Linear).
    *   Mip levels (if available).
    *   Linked materials or models (clickable).

4.  **Script / Data Section**:
    *   Entry point or script type (e.g., `Init Script`, `AI Script`).
    *   High-level validation status (e.g., `Syntax OK`, `Parse Error`).
    *   Linked entities or systems (e.g., NPCs using this script).

5.  **Modding-Specific Status Section**:
    *   Override state:
        *   `Vanilla only`
        *   `Overridden by current mod`
        *   `New asset in current mod`
    *   Conflict summary:
        *   List of other mods that also touch this asset (if known).
        *   Simple conflict level: `None`, `Potential`, `Severe`.
    *   Quick actions:
        *   `Duplicate to Mod` (for vanilla assets).
        *   `Revert to Vanilla` (for overridden assets).
        *   `Open Diff View` (e.g., show script diff in floating window).

---

## 6. Floating Window Design (The Core Component)

A persistent, pop-up utility window.

### Layout Strategy
*   **Component**: `egui::Window`.
*   **Properties**:
    *   `resizable(true)`
    *   `collapsible(true)`
    *   `title_bar(true)`
    *   `scroll2([true, true])`
    *   `open(&mut show_popup)`: Controlled by the Top Nav boolean.

### Content Layout
The floating window is designed as a flexible container for specialized modding tools. Some possible modes:

1.  **Build & Log View**:
    *   Title: `Build Output`.
    *   Scrollable text area showing:
        *   Build steps.
        *   Warnings and errors with clickable paths leading to relevant assets.
    *   Filter bar for `Info / Warning / Error`.

2.  **Diff / Comparison View**:
    *   Title: `Compare Asset`.
    *   Side-by-side or unified diff for scripts and text assets.
    *   For binary assets, show metadata comparison (file size, hash, timestamps).

3.  **Batch Tools View**:
    *   Title: `Batch Tools`.
    *   Options like:
        *   Batch rename textures.
        *   Relink texture paths in materials.
        *   Normalize file naming patterns for a selected directory.

4.  **Mod Load Order / Profile Manager**:
    *   Title: `Mod Manager`.
    *   List of detected mods with:
        *   Enabled/disabled toggles.
        *   Drag handles to reorder.
        *   Quick conflict warning icons.

The internal layout can switch between these modes using tabs or a small mode selector, depending on the current task.

---

## 7. Visual Layout Sketches (ASCII)

### 7.1 Global Layout

```text
+-----------------------------------------------------------------------+
|  [File] [View] [Mods] [Tools] [Help]     [Status: Ready] [Build&Run]  | <--- Top Panel
+---------------------+------------------------------+------------------+
| Search... [Ext] [Scope]                            |                  |
| > assets (Game)    |                              |  INFO PANEL      | <--- Right Panel
|   v models         |       +----------+           |  (Details,       |
|     > char.obj (*) |       |   /  /   |           |   Conflicts,     |
|     > env.fbx      |       |  +---+   |           |   References)    |
|   > textures       |       |  |   |   |           |                  |
|     - wood.png (+) |       |  +---+   |           |                  |
|                     |       +----------+           |                  |
|                     |       (Rotating Box / Model) |                  |
|                     |                              |                  |
|                     |  [HUD: Play/Reset/Grid]      |                  |
|                     |                              |                  |
+---------------------+------------------------------+------------------+

Legend:
(*) overridden asset
(+) new mod asset
```

### 7.2 Left Panel Detail (File Browser)

```text
+-----------------------------------------+
| Search...        [Ext v]    [Scope v]   | <--- Unified Horizontal Row
+-----------------------------------------+
| > assets (Game)                         | <--- CollapsingHeader
|   > textures                            |
|     [ICON] wood_diffuse.png             | <--- SelectableLabel
|     [ICON] wood_spec.png                |
|   > scripts                             |
|     [ICON] init.lua                     |
| > mod_assets (Current Mod)              |
|   > characters                          |
|     [ICON*] char.obj (override)         |
+-----------------------------------------+
```

### 7.3 Floating Window (Popup)

```text
+------------------------------------------+
|  Build Output                        [X] | <--- Window Title Bar
+------------------------------------------+
|  [Info] [Warning] [Error]                | <--- Filter Bar
|                                          |
|  [OK] Building mod "my_mod"...           |
|  [OK] Packing textures...                |
|  [WARN] Missing mipmaps for wood.png     |
|  [ERR] Failed to compile script init.lua |
|                                          |
+------------------------------------------+
```

---

## 8. Modding Workflows and UX Considerations

This section describes how the above UI supports common modding workflows.

### 8.1 Typical Workflow: Create and Test a New Mod

1. **Open Game Install** using `File > Open Game Install...`.
2. **Create New Mod Project** via `File > New Mod Project...`.
3. **Browse Assets** in the left panel (`Scope = Game`), locate a model or texture.
4. **Duplicate Asset to Mod** using context menu (`Duplicate to Mod`), which:
    *   Copies the asset into the mod project directory.
    *   Switches `Scope` to `Combined` or `Mod` to highlight the new asset.
5. **Edit Asset Externally** (if needed) via `Open in External Editor`.
6. **Preview Changes** in the center panel and check metadata in the right panel.
7. **Build Mod** via `Mods > Build Mod`, read logs in the floating window.
8. **Run Game with Mod** via `Mods > Run In Game` or manually from outside the tool.

### 8.2 Workflow: Investigate and Resolve Conflicts

1. Set `Scope` to `Combined` to see both game and mod assets.
2. Use the left panel to navigate to assets with conflict warnings (flagged visually).
3. Select an asset to view:
    *   Override state and conflict details in the right panel.
    *   Optional comparison controls in the center panel (`Vanilla` vs. `Modded`).
4. Open the **Diff / Comparison View** in the floating window for scripts or data.
5. Use the **Mod Manager** mode (floating window) to adjust load order if conflicts are load-order-dependent.

### 8.3 UX Notes and Guidelines

*   Prefer **read-only** views for vanilla game files and make it visually obvious when editing mod copies instead of original files.
*   Clearly separate **destructive actions** (delete, overwrite) with confirmation dialogs.
*   Keep frequently used actions (build, refresh, reset camera) in the top toolbar or HUD.
*   Maintain consistent visual language for:
    *   Game vs. mod vs. override assets.
    *   Error vs. warning vs. success states.
*   Provide keyboard shortcuts for expert users (e.g., `Ctrl+Shift+B` for build, `F5` for refresh).


