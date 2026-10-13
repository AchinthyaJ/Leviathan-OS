use crate::clipboard::Clipboard;
use crate::color::Color;
use crate::graphics::Graphics;
use crate::icons::{draw_icon, IconId};
use crate::image_loader::ImageCache;
use crate::input::Action;
use crate::layer::{Layer, LayerElement};
use crate::layer_manager::LayerManager;
use crate::mouse::MouseState;
use crate::widget::Rect;
use std::path::Path;

pub struct LayerEditorApp {
    pub window: WindowState,
    pub layer_manager: LayerManager,
    pub selected_layer_name: Option<String>,
    pub selected_element_id: Option<String>,
    pub drag_offset: Option<(i32, i32)>, // (x, y) offset from mouse to element top-left when dragging
    pub layer_name_input: String,         // for new layer name
    pub image_path_input: String,         // for importing image
    pub image_cache: ImageCache,
    pub is_adding_icon: bool,    // typing in image path field
    pub is_importing_image: bool,
    pub status_message: String,   // brief feedback message shown in the editor
}

#[derive(Debug, Clone)]
pub struct WindowState {
    pub is_open: bool,
    pub is_focused: bool,
    pub rect: Rect,
    pub is_minimized: bool,
    pub is_maximized: bool,
}

impl WindowState {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            is_open: false,
            is_focused: false,
            rect: Rect::new(x, y, w, h),
            is_minimized: false,
            is_maximized: false,
        }
    }

    pub fn toggle_maximize(&mut self, sw: i32, sh: i32) {
        if self.is_maximized {
            self.is_maximized = false;
            // restore saved rect — for simplicity keep current since we didn't save it separately
        } else {
            self.is_maximized = true;
            self.rect = Rect::new(0, 0, sw, sh);
        }
    }
}

impl LayerEditorApp {
    pub fn new(_width: u32, _height: u32) -> Self {
        Self {
            window: WindowState::new(100, 100, 800, 600),
            layer_manager: LayerManager::new(),
            selected_layer_name: None,
            selected_element_id: None,
            drag_offset: None,
            layer_name_input: String::new(),
            image_path_input: String::new(),
            image_cache: ImageCache::new(),
            is_adding_icon: false,
            is_importing_image: false,
            status_message: String::new(),
        }
    }

    pub fn open(&mut self) {
        self.window.is_open = true;
        self.window.is_focused = true;
        self.window.is_minimized = false;
        self.window.is_maximized = false;
        // Refresh layer list from disk each time we open
        self.layer_manager.load_layers();
    }

    pub fn close(&mut self) {
        self.window.is_open = false;
        self.window.is_focused = false;
    }

    /// Returns the canvas rect (the area where elements are drawn)
    fn canvas_rect(&self) -> Rect {
        let wr = &self.window.rect;
        Rect::new(wr.x + 170, wr.y + 60, wr.w - 180, wr.h - 80)
    }

    pub fn update(&mut self, mouse_state: &MouseState, _width: i32, _height: i32) {
        if !self.window.is_open {
            return;
        }

        let wr = self.window.rect;

        // Click on title bar focuses the window
        let title_bar = Rect::new(wr.x, wr.y, wr.w, 20);
        if mouse_state.left_clicked && title_bar.contains(mouse_state.x, mouse_state.y) {
            self.window.is_focused = true;
        }

        if !self.window.is_focused {
            return;
        }

        // ── Layer list panel (left side) ──────────────────────────────────────
        let layer_list_rect = Rect::new(wr.x + 10, wr.y + 60, 150, wr.h - 120);
        if layer_list_rect.contains(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            let relative_y = mouse_state.y - (wr.y + 60);
            let layer_index = (relative_y / 20) as usize;
            if let Some(layer) = self.layer_manager.layers.get(layer_index) {
                let name = layer.name.clone();
                self.selected_layer_name = Some(name);
                self.selected_element_id = None;
                self.drag_offset = None;
            }
        }

        // ── Canvas ────────────────────────────────────────────────────────────
        let canvas = self.canvas_rect();
        if canvas.contains(mouse_state.x, mouse_state.y) {
            if mouse_state.left_clicked {
                let mut found = false;
                if let Some(layer_name) = self.selected_layer_name.clone() {
                    if let Some(layer) = self.layer_manager.get_layer(&layer_name) {
                        for element in &layer.elements {
                            let er = Rect::new(element.x, element.y,
                                               element.width as i32, element.height as i32);
                            if er.contains(mouse_state.x, mouse_state.y) {
                                self.selected_element_id = Some(element.id.clone());
                                self.drag_offset = Some((
                                    mouse_state.x - element.x,
                                    mouse_state.y - element.y,
                                ));
                                found = true;
                                break;
                            }
                        }
                    }
                }
                if !found {
                    self.selected_element_id = None;
                    self.drag_offset = None;
                }
            }

            // Drag selected element
            if mouse_state.left_pressed {
                if let (Some(sel_id), Some(offset)) = (self.selected_element_id.clone(), self.drag_offset) {
                    if let Some(layer_name) = self.selected_layer_name.clone() {
                        if let Some(layer) = self.layer_manager.get_layer_mut(&layer_name) {
                            if let Some(el) = layer.elements.iter_mut().find(|e| e.id == sel_id) {
                                el.x = mouse_state.x - offset.0;
                                el.y = mouse_state.y - offset.1;
                            }
                        }
                    }
                }
            }

            if !mouse_state.left_pressed {
                self.drag_offset = None;
            }
        }

        let wr = self.window.rect; // reborrow after possible mutable borrow above

        // ── Buttons ───────────────────────────────────────────────────────────
        let button_y = wr.y + wr.h - 100;

        // New Layer
        let new_layer_btn = Rect::new(wr.x + 10, button_y, 80, 20);
        if new_layer_btn.contains(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            self.layer_name_input.clear();
            self.is_adding_icon = false;
            self.is_importing_image = false;
            self.status_message = "Type name then press Enter".to_string();
        }

        // Add Icon (type path then Enter)
        let add_icon_btn = Rect::new(wr.x + 100, button_y, 80, 20);
        if add_icon_btn.contains(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            self.is_adding_icon = true;
            self.is_importing_image = false;
            self.image_path_input.clear();
            self.status_message = "Type image path then Enter".to_string();
        }

        // Import Image (alias for Add Icon)
        let import_image_btn = Rect::new(wr.x + 190, button_y, 80, 20);
        if import_image_btn.contains(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            self.is_importing_image = true;
            self.is_adding_icon = false;
            self.image_path_input.clear();
            self.status_message = "Type image path then Enter".to_string();
        }

        // Paste path from clipboard
        let paste_btn = Rect::new(wr.x + 280, button_y, 80, 20);
        if paste_btn.contains(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            let path = Clipboard::get_text();
            if !path.is_empty() {
                self.add_image_element(path.trim().to_string());
            }
        }

        // Save
        let save_btn = Rect::new(wr.x + wr.w - 100, wr.y + wr.h - 40, 80, 20);
        if save_btn.contains(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            self.save_selected_layer();
        }

        // Delete selected element
        let delete_btn = Rect::new(wr.x + wr.w - 190, wr.y + wr.h - 40, 80, 20);
        if delete_btn.contains(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            self.delete_selected_element();
        }

        // Activate / Deactivate layer
        let activate_btn = Rect::new(wr.x + wr.w - 280, wr.y + wr.h - 40, 80, 20);
        if activate_btn.contains(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            if let Some(layer_name) = &self.selected_layer_name {
                let name = layer_name.clone();
                if self.layer_manager.active_layer_name.as_deref() == Some(&name) {
                    self.layer_manager.active_layer_name = None;
                } else {
                    self.layer_manager.set_active_layer(&name);
                }
            }
        }
    }

    fn add_image_element(&mut self, path: String) {
        if path.is_empty() {
            return;
        }
        if let Some(layer_name) = self.selected_layer_name.clone() {
            if let Some(layer) = self.layer_manager.get_layer_mut(&layer_name) {
                let id = format!("element_{}", layer.elements.len());
                let element = LayerElement {
                    id,
                    element_type: crate::layer::ElementType::Icon,
                    x: 100,
                    y: 100,
                    width: 64,
                    height: 64,
                    image_path: Some(path.clone()),
                    visibility: true,
                };
                layer.elements.push(element);
                // Pre-load the image into the cache (ignore errors)
                let _ = self.image_cache.load(&path);
                self.status_message = format!("Added: {}", Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path));
            } else {
                self.status_message = "Select a layer first".to_string();
            }
        } else {
            self.status_message = "Select a layer first".to_string();
        }
    }

    fn save_selected_layer(&mut self) {
        // Rename if name_input is not empty
        if !self.layer_name_input.is_empty() {
            if let Some(layer_name) = self.selected_layer_name.clone() {
                if let Some(layer) = self.layer_manager.get_layer_mut(&layer_name) {
                    layer.name = self.layer_name_input.clone();
                }
                self.selected_layer_name = Some(self.layer_name_input.clone());
            }
            self.layer_name_input.clear();
        }

        if let Some(layer_name) = self.selected_layer_name.clone() {
            // Clone the layer to avoid borrow conflicts
            if let Some(layer_clone) = self.layer_manager.get_layer(&layer_name).cloned() {
                match self.layer_manager.save_layer(&layer_clone) {
                    Ok(()) => {
                        self.status_message = format!("Saved '{}'", layer_name);
                    }
                    Err(e) => {
                        eprintln!("leviathan-ui: Failed to save layer '{}': {}", layer_name, e);
                        self.status_message = format!("Save failed: {}", e);
                    }
                }
            }
        } else {
            self.status_message = "No layer selected".to_string();
        }
    }

    fn delete_selected_element(&mut self) {
        if let Some(layer_name) = self.selected_layer_name.clone() {
            if let Some(element_id) = self.selected_element_id.clone() {
                if let Some(layer) = self.layer_manager.get_layer_mut(&layer_name) {
                    layer.elements.retain(|e| e.id != element_id);
                    self.selected_element_id = None;
                    self.drag_offset = None;
                    self.status_message = "Element deleted".to_string();
                }
            } else {
                self.status_message = "No element selected".to_string();
            }
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        if !self.window.is_open || !self.window.is_focused {
            return;
        }

        match action {
            Action::Char(b) => {
                if self.is_adding_icon || self.is_importing_image {
                    if b >= 32 && b < 127 {
                        self.image_path_input.push(b as char);
                    }
                } else {
                    if b >= 32 && b < 127 {
                        self.layer_name_input.push(b as char);
                    }
                }
            }
            Action::Backspace => {
                if self.is_adding_icon || self.is_importing_image {
                    self.image_path_input.pop();
                } else {
                    self.layer_name_input.pop();
                }
            }
            Action::Enter => {
                if self.is_adding_icon || self.is_importing_image {
                    let path = self.image_path_input.trim().to_string();
                    self.image_path_input.clear();
                    self.is_adding_icon = false;
                    self.is_importing_image = false;
                    self.add_image_element(path);
                } else if !self.layer_name_input.is_empty() {
                    // Create a new layer with this name
                    let name = self.layer_name_input.clone();
                    self.layer_name_input.clear();
                    // Avoid duplicate names
                    if self.layer_manager.layers.iter().any(|l| l.name == name) {
                        self.status_message = format!("Layer '{}' already exists", name);
                    } else {
                        let new_layer = Layer {
                            name: name.clone(),
                            elements: Vec::new(),
                        };
                        match self.layer_manager.save_layer(&new_layer) {
                            Ok(()) => {
                                self.selected_layer_name = Some(name.clone());
                                self.status_message = format!("Created layer '{}'", name);
                            }
                            Err(e) => {
                                self.status_message = format!("Create failed: {}", e);
                            }
                        }
                    }
                }
            }
            Action::Escape => {
                self.is_adding_icon = false;
                self.is_importing_image = false;
                self.image_path_input.clear();
                self.layer_name_input.clear();
                self.status_message.clear();
            }
            Action::Delete => {
                self.delete_selected_element();
            }
            _ => {}
        }
    }

    pub fn draw(&mut self, gfx: &mut Graphics, mouse_state: &MouseState) {
        if !self.window.is_open {
            return;
        }

        let wr = self.window.rect;

        // Window background
        gfx.fill_rect(wr.x, wr.y, wr.w, wr.h, Color::new(40, 40, 40));
        gfx.draw_rect(wr.x, wr.y, wr.w, wr.h, Color::new(80, 80, 80));

        // Title bar
        gfx.fill_rect(wr.x, wr.y, wr.w, 20, Color::new(60, 60, 60));
        gfx.draw_string_i(wr.x + 10, wr.y + 5, "Layer Editor", Color::new(255, 255, 255));

        // Close button
        let close_btn = Rect::new(wr.x + wr.w - 22, wr.y + 2, 20, 16);
        gfx.fill_rect(close_btn.x, close_btn.y, close_btn.w, close_btn.h, Color::new(180, 40, 40));
        draw_icon(gfx, IconId::Close, close_btn.x + (close_btn.w - 10) / 2, close_btn.y + (close_btn.h - 10) / 2, 10, Color::WHITE);

        // Layer Name input label + box
        gfx.draw_string_i(wr.x + 10, wr.y + 30, "Name:", Color::new(200, 200, 200));
        let name_input_rect = Rect::new(wr.x + 60, wr.y + 28, wr.w - 70, 18);
        gfx.fill_rect(name_input_rect.x, name_input_rect.y, name_input_rect.w, name_input_rect.h,
                      Color::new(50, 50, 50));
        gfx.draw_rect(name_input_rect.x, name_input_rect.y, name_input_rect.w, name_input_rect.h,
                      Color::new(100, 100, 100));
        let name_display = if self.layer_name_input.is_empty() && !self.is_adding_icon && !self.is_importing_image {
            "type name then Enter..."
        } else {
            &self.layer_name_input
        };
        let name_col = if self.layer_name_input.is_empty() { Color::new(90, 90, 90) } else { Color::WHITE };
        gfx.draw_string_i(name_input_rect.x + 4, name_input_rect.y + 4, name_display, name_col);

        // ── Layer list ────────────────────────────────────────────────────────
        let layer_list_rect = Rect::new(wr.x + 10, wr.y + 60, 150, wr.h - 120);
        gfx.fill_rect(layer_list_rect.x, layer_list_rect.y, layer_list_rect.w, layer_list_rect.h,
                      Color::new(50, 50, 50));
        gfx.draw_rect(layer_list_rect.x, layer_list_rect.y, layer_list_rect.w, layer_list_rect.h,
                      Color::new(100, 100, 100));
        gfx.draw_string_i(layer_list_rect.x + 5, layer_list_rect.y + 5, "Layers", Color::new(200, 200, 200));

        // Active layer indicator in list
        for (i, layer) in self.layer_manager.layers.iter().enumerate() {
            let y = layer_list_rect.y + 22 + (i as i32 * 20);
            if y + 18 > layer_list_rect.y + layer_list_rect.h {
                break; // don't draw outside the list box
            }
            let is_selected = self.selected_layer_name.as_deref() == Some(&layer.name);
            let is_active = self.layer_manager.active_layer_name.as_deref() == Some(&layer.name);
            let bg = if is_selected {
                Color::new(70, 80, 110)
            } else {
                Color::new(50, 50, 50)
            };
            gfx.fill_rect(layer_list_rect.x, y, layer_list_rect.w, 18, bg);
            let prefix = if is_active { "● " } else { "  " };
            let display = format!("{}{}", prefix, layer.name);
            let text_col = if is_selected { Color::WHITE } else { Color::new(200, 200, 200) };
            gfx.draw_string_i(layer_list_rect.x + 5, y + 4, &display, text_col);
        }

        // ── Canvas ────────────────────────────────────────────────────────────
        let canvas = self.canvas_rect();
        gfx.fill_rect(canvas.x, canvas.y, canvas.w, canvas.h, Color::new(25, 25, 25));
        gfx.draw_rect(canvas.x, canvas.y, canvas.w, canvas.h, Color::new(60, 60, 60));

        // Draw elements on the canvas
        if let Some(layer_name) = self.selected_layer_name.clone() {
            if let Some(layer) = self.layer_manager.get_layer(&layer_name) {
                for element in &layer.elements {
                    if !element.visibility {
                        continue;
                    }

                    let er = Rect::new(element.x, element.y,
                                       element.width as i32, element.height as i32);

                    // Try to draw image, fall back to colored rect
                    let mut drew_image = false;
                    if let Some(ref path) = element.image_path {
                        // Attempt to load into cache if not already
                        if self.image_cache.get(path).is_none() {
                            let _ = self.image_cache.load(path);
                        }
                        if let Some(img) = self.image_cache.get(path) {
                            let pitch = gfx.pitch();
                            let gw = gfx.width();
                            let gh = gfx.height();
                            let buf = gfx.backbuffer_mut();
                            img.draw_to_buffer_scaled(buf, gw, gh, pitch,
                                                      er.x, er.y,
                                                      er.w.max(1) as u32, er.h.max(1) as u32,
                                                      1.0);
                            drew_image = true;
                        }
                    }

                    if !drew_image {
                        gfx.fill_rect(er.x, er.y, er.w, er.h, Color::new(80, 80, 100));
                        let label = element.image_path
                            .as_deref()
                            .and_then(|p| Path::new(p).file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("Icon");
                        gfx.draw_string_i(er.x + 4, er.y + 4, label, Color::WHITE);
                    }

                    // Element border
                    gfx.draw_rect(er.x, er.y, er.w, er.h, Color::new(140, 140, 140));

                    // Selection highlight (thick cyan border)
                    if self.selected_element_id.as_deref() == Some(&element.id) {
                        gfx.draw_rect(er.x - 1, er.y - 1, er.w + 2, er.h + 2, Color::new(0, 200, 255));
                        gfx.draw_rect(er.x - 2, er.y - 2, er.w + 4, er.h + 4, Color::new(0, 200, 255));
                    }
                }
            }
        } else {
            gfx.draw_string_i(canvas.x + canvas.w / 2 - 80, canvas.y + canvas.h / 2 - 4,
                              "Select or create a layer", Color::new(80, 80, 80));
        }

        // ── Bottom button row ─────────────────────────────────────────────────
        let button_y = wr.y + wr.h - 100;

        Self::draw_button(gfx, mouse_state, wr.x + 10,   button_y, 80, 20, "New Layer", Color::new(60, 60, 80));
        Self::draw_button(gfx, mouse_state, wr.x + 100,  button_y, 80, 20, "Add Icon",
                          if self.is_adding_icon { Color::new(0, 120, 0) } else { Color::new(60, 60, 60) });
        Self::draw_button(gfx, mouse_state, wr.x + 190,  button_y, 80, 20, "Import Img",
                          if self.is_importing_image { Color::new(0, 120, 0) } else { Color::new(60, 60, 60) });
        Self::draw_button(gfx, mouse_state, wr.x + 280,  button_y, 80, 20, "Paste Path", Color::new(60, 60, 60));

        // Image path input (shown when in add/import mode)
        let input_y = wr.y + wr.h - 70;
        if self.is_adding_icon || self.is_importing_image {
            gfx.draw_string_i(wr.x + 10, input_y, "Path:", Color::new(200, 200, 200));
            let img_input = Rect::new(wr.x + 55, input_y - 2, wr.w - 65, 18);
            gfx.fill_rect(img_input.x, img_input.y, img_input.w, img_input.h, Color::new(50, 50, 50));
            gfx.draw_rect(img_input.x, img_input.y, img_input.w, img_input.h, Color::new(0, 180, 100));
            gfx.draw_string_i(img_input.x + 4, img_input.y + 3, &self.image_path_input, Color::WHITE);
            gfx.draw_string_i(wr.x + 10, input_y + 20, "Press Enter to add, Escape to cancel", Color::new(120, 120, 120));
        }

        // Bottom action buttons
        let act_y = wr.y + wr.h - 40;
        let is_active = self.layer_manager.active_layer_name.as_deref()
            == self.selected_layer_name.as_deref();
        let act_label = if is_active { "Deactivate" } else { "Activate" };
        let act_col   = if is_active { Color::new(0, 120, 200) } else { Color::new(60, 60, 80) };

        Self::draw_button(gfx, mouse_state, wr.x + wr.w - 280, act_y, 80, 20, act_label, act_col);
        Self::draw_button(gfx, mouse_state, wr.x + wr.w - 190, act_y, 80, 20, "Delete",  Color::new(140, 30, 30));
        Self::draw_button(gfx, mouse_state, wr.x + wr.w - 100, act_y, 80, 20, "Save",    Color::new(0, 130, 0));

        // Status message
        if !self.status_message.is_empty() {
            gfx.draw_string_i(wr.x + 10, wr.y + wr.h - 18, &self.status_message, Color::new(180, 220, 100));
        }
    }

    fn draw_button(gfx: &mut Graphics, mouse: &MouseState,
                   x: i32, y: i32, w: i32, h: i32, label: &str, bg: Color) {
        let is_hovered = mouse.x >= x && mouse.x < x + w && mouse.y >= y && mouse.y < y + h;
        let bg = if is_hovered { Color::new(bg.r.saturating_add(20), bg.g.saturating_add(20), bg.b.saturating_add(20)) } else { bg };
        gfx.fill_rect(x, y, w, h, bg);
        gfx.draw_rect(x, y, w, h, Color::new(100, 100, 100));
        let tx = x + (w - label.len() as i32 * 8) / 2;
        let ty = y + (h - 8) / 2;
        if tx >= 0 && ty >= 0 {
            gfx.draw_string_i(tx, ty, label, Color::WHITE);
        }
    }
}