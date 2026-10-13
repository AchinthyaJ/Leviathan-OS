use crate::color::Color;
use crate::graphics::Graphics;
use crate::icons::{draw_icon, IconId};
use crate::input::Action;
use crate::mouse::MouseState;
use crate::widget::{Button, Rect, Window};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct FileItem {
    pub name: String,
    pub is_dir: bool,
    pub is_exe: bool,
    pub size_bytes: u64,
    pub permissions: u32,
    pub modified_str: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogMode {
    None,
    NewFolder { input: String },
    Rename { input: String, original: String },
    DeleteConfirm { target: String, is_dir: bool },
}

pub struct FilesApp {
    pub window: Window,
    pub current_dir: PathBuf,
    pub items: Vec<FileItem>,
    pub selected_idx: Option<usize>,

    // Navigation History
    pub history_back: Vec<PathBuf>,
    pub history_forward: Vec<PathBuf>,

    // Navigation buttons
    btn_back: Button,
    btn_forward: Button,
    btn_up: Button,
    btn_home: Button,
    btn_root: Button,
    btn_etc: Button,
    btn_usr: Button,
    btn_tmp: Button,
    btn_media: Button,

    // File operation buttons
    btn_new_folder: Button,
    btn_rename: Button,
    btn_delete: Button,
    btn_copy: Button,
    btn_cut: Button,
    btn_paste: Button,
    btn_refresh: Button,

    // Dialog buttons
    btn_dialog_ok: Button,
    btn_dialog_cancel: Button,

    // Error banner
    pub error_banner: Option<String>,

    // Clipboard for copy/move
    pub clipboard_path: Option<PathBuf>,
    pub clipboard_is_cut: bool,

    // Active dialog
    pub dialog: DialogMode,

    // Scrolling & double-click tracking
    pub scroll_offset: usize,
    last_click_idx: Option<usize>,
    last_click_time: Instant,
}

impl FilesApp {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        // Navigation row
        let btn_back = Button::new_icon(x + 16, y + 34, 26, 22, IconId::ChevronLeft);
        let btn_forward = Button::new_icon(x + 46, y + 34, 26, 22, IconId::ChevronRight);
        let btn_up = Button::new_icon(x + 76, y + 34, 26, 22, IconId::ChevronUp);
        let btn_home = Button::new_with_icon(x + 108, y + 34, 68, 22, IconId::Home, "Home");
        let btn_root = Button::new_with_icon(x + 180, y + 34, 66, 22, IconId::HardDrive, "Root");
        let btn_etc = Button::new(x + 250, y + 34, 40, 22, "etc");
        let btn_usr = Button::new(x + 294, y + 34, 40, 22, "usr");
        let btn_tmp = Button::new(x + 338, y + 34, 40, 22, "tmp");
        let btn_media = Button::new(x + 382, y + 34, 52, 22, "media");

        // Action row
        let btn_new_folder = Button::new_with_icon(x + 16, y + 60, 78, 22, IconId::FolderPlus, "Folder");
        let btn_rename = Button::new_with_icon(x + 98, y + 60, 76, 22, IconId::Edit, "Rename");
        let btn_delete = Button::new_with_icon(x + 178, y + 60, 74, 22, IconId::Trash, "Delete");
        let btn_copy = Button::new_with_icon(x + 256, y + 60, 62, 22, IconId::Copy, "Copy");
        let btn_cut = Button::new_with_icon(x + 322, y + 60, 56, 22, IconId::Cut, "Cut");
        let btn_paste = Button::new_with_icon(x + 382, y + 60, 64, 22, IconId::Paste, "Paste");
        let btn_refresh = Button::new_with_icon(x + 450, y + 60, 76, 22, IconId::Refresh, "Refresh");

        // Dialog buttons
        let btn_dialog_ok = Button::new(x + 200, y + 200, 70, 24, "OK");
        let btn_dialog_cancel = Button::new(x + 280, y + 200, 70, 24, "Cancel");

        let mut app = Self {
            window: Window::new("Files - Leviathan File Browser", x, y, w, h),
            current_dir: PathBuf::from("/home/leviathan"),
            items: Vec::new(),
            selected_idx: None,
            history_back: Vec::new(),
            history_forward: Vec::new(),
            btn_back,
            btn_forward,
            btn_up,
            btn_home,
            btn_root,
            btn_etc,
            btn_usr,
            btn_tmp,
            btn_media,
            btn_new_folder,
            btn_rename,
            btn_delete,
            btn_copy,
            btn_cut,
            btn_paste,
            btn_refresh,
            btn_dialog_ok,
            btn_dialog_cancel,
            error_banner: None,
            clipboard_path: None,
            clipboard_is_cut: false,
            dialog: DialogMode::None,
            scroll_offset: 0,
            last_click_idx: None,
            last_click_time: Instant::now(),
        };

        app.refresh();
        app
    }

    pub fn navigate_to(&mut self, path: PathBuf) {
        if path.is_dir() {
            if path != self.current_dir {
                self.history_back.push(self.current_dir.clone());
                self.history_forward.clear();
            }
            self.current_dir = path;
            self.selected_idx = None;
            self.scroll_offset = 0;
            self.error_banner = None;
            self.refresh();
        } else {
            self.error_banner = Some(format!("Not a directory: {}", path.display()));
        }
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.history_back.pop() {
            self.history_forward.push(self.current_dir.clone());
            self.current_dir = prev;
            self.selected_idx = None;
            self.scroll_offset = 0;
            self.error_banner = None;
            self.refresh();
        }
    }

    pub fn go_forward(&mut self) {
        if let Some(next) = self.history_forward.pop() {
            self.history_back.push(self.current_dir.clone());
            self.current_dir = next;
            self.selected_idx = None;
            self.scroll_offset = 0;
            self.error_banner = None;
            self.refresh();
        }
    }

    pub fn refresh(&mut self) {
        self.items.clear();
        let p = if self.current_dir.exists() {
            &self.current_dir
        } else {
            &PathBuf::from("/")
        };

        match fs::read_dir(p) {
            Ok(entries) => {
                let mut dirs = Vec::new();
                let mut files = Vec::new();

                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        continue;
                    }
                    let meta = entry.metadata().ok();
                    let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);

                    #[cfg(unix)]
                    let (is_exe, permissions, modified_str) = {
                        use std::os::unix::fs::MetadataExt;
                        use std::os::unix::fs::PermissionsExt;
                        let perm = meta.as_ref().map(|m| m.permissions().mode()).unwrap_or(0);
                        let exe = perm & 0o111 != 0;
                        let mtime = meta.as_ref().map(|m| m.mtime()).unwrap_or(0);
                        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
                        unsafe { libc::localtime_r(&mtime, &mut tm) };
                        let mod_s = format!(
                            "{:04}-{:02}-{:02} {:02}:{:02}",
                            tm.tm_year + 1900,
                            tm.tm_mon + 1,
                            tm.tm_mday,
                            tm.tm_hour,
                            tm.tm_min
                        );
                        (exe, perm, mod_s)
                    };

                    #[cfg(not(unix))]
                    let (is_exe, permissions, modified_str) =
                        (false, 0o644, "2026-09-05 08:00".to_string());

                    let item = FileItem {
                        name,
                        is_dir,
                        is_exe,
                        size_bytes,
                        permissions,
                        modified_str,
                    };

                    if is_dir {
                        dirs.push(item);
                    } else {
                        files.push(item);
                    }
                }

                dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                self.items.extend(dirs);
                self.items.extend(files);
            }
            Err(e) => {
                self.error_banner = Some(format!("Cannot open directory: {}", e));
            }
        }
    }

    pub fn handle_key(&mut self, ch: char) {
        match &mut self.dialog {
            DialogMode::NewFolder { input } | DialogMode::Rename { input, .. } => {
                if ch == '\x08' {
                    input.pop();
                } else if ch == '\r' || ch == '\n' {
                    self.execute_dialog_ok();
                } else if ch == '\x1b' {
                    self.dialog = DialogMode::None;
                } else if !ch.is_control() && ch != '/' && ch != '\0' {
                    input.push(ch);
                }
            }
            DialogMode::DeleteConfirm { .. } => {
                if ch == '\r' || ch == '\n' || ch == 'y' || ch == 'Y' {
                    self.execute_dialog_ok();
                } else if ch == '\x1b' || ch == 'n' || ch == 'N' {
                    self.dialog = DialogMode::None;
                }
            }
            DialogMode::None => {
                if ch == '\r' || ch == '\n' {
                    if let Some(idx) = self.selected_idx {
                        if let Some(item) = self.items.get(idx) {
                            if item.is_dir {
                                let new_dir = self.current_dir.join(&item.name);
                                self.navigate_to(new_dir);
                            }
                        }
                    }
                } else if ch == '\x1b' {
                    self.selected_idx = None;
                    self.error_banner = None;
                }
            }
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        if self.dialog != DialogMode::None {
            return;
        }
        match action {
            Action::ArrowUp => {
                if !self.items.is_empty() {
                    let cur = self.selected_idx.unwrap_or(0);
                    if cur > 0 {
                        self.selected_idx = Some(cur - 1);
                        if cur - 1 < self.scroll_offset {
                            self.scroll_offset = cur - 1;
                        }
                    }
                }
            }
            Action::ArrowDown => {
                if !self.items.is_empty() {
                    let cur = self.selected_idx.unwrap_or(0);
                    if cur + 1 < self.items.len() {
                        self.selected_idx = Some(cur + 1);
                        let visible = ((self.window.rect.h - 150) / 26).max(1) as usize;
                        if cur + 1 >= self.scroll_offset + visible {
                            self.scroll_offset += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn execute_dialog_ok(&mut self) {
        let cur_dialog = std::mem::replace(&mut self.dialog, DialogMode::None);
        match cur_dialog {
            DialogMode::NewFolder { input } => {
                let name = input.trim();
                if name.is_empty() {
                    self.error_banner = Some("Folder name cannot be empty".to_string());
                    return;
                }
                let target = self.current_dir.join(name);
                if target.exists() {
                    self.error_banner = Some(format!("'{}' already exists", name));
                    return;
                }
                match fs::create_dir(&target) {
                    Ok(()) => {
                        self.refresh();
                        if let Some(pos) = self.items.iter().position(|i| i.name == name) {
                            self.selected_idx = Some(pos);
                        }
                    }
                    Err(e) => {
                        self.error_banner = Some(format!("Create folder failed: {}", e));
                    }
                }
            }
            DialogMode::Rename { input, original } => {
                let new_name = input.trim();
                if new_name.is_empty() {
                    self.error_banner = Some("Name cannot be empty".to_string());
                    return;
                }
                if new_name == original {
                    return;
                }
                let src = self.current_dir.join(&original);
                let dst = self.current_dir.join(new_name);
                if dst.exists() {
                    self.error_banner = Some(format!("'{}' already exists", new_name));
                    return;
                }
                match fs::rename(&src, &dst) {
                    Ok(()) => {
                        self.refresh();
                        if let Some(pos) = self.items.iter().position(|i| i.name == new_name) {
                            self.selected_idx = Some(pos);
                        }
                    }
                    Err(e) => {
                        self.error_banner = Some(format!("Rename failed: {}", e));
                    }
                }
            }
            DialogMode::DeleteConfirm { target, is_dir } => {
                let path = self.current_dir.join(&target);
                let res = if is_dir {
                    fs::remove_dir_all(&path)
                } else {
                    fs::remove_file(&path)
                };
                match res {
                    Ok(()) => {
                        self.selected_idx = None;
                        self.refresh();
                    }
                    Err(e) => {
                        self.error_banner = Some(format!("Delete failed: {}", e));
                    }
                }
            }
            DialogMode::None => {}
        }
    }

    pub fn update(&mut self, mouse: &MouseState, screen_w: i32, screen_h: i32) -> Option<String> {
        self.window.update_drag(mouse, screen_w, screen_h);
        if !self.window.is_open {
            return None;
        }

        let wx = self.window.rect.x;
        let wy = self.window.rect.y;
        let ww = self.window.rect.w;
        let wh = self.window.rect.h;

        // Position navigation buttons
        self.btn_back.rect.x = wx + 16;
        self.btn_back.rect.y = wy + 34;
        self.btn_forward.rect.x = wx + 46;
        self.btn_forward.rect.y = wy + 34;
        self.btn_up.rect.x = wx + 76;
        self.btn_up.rect.y = wy + 34;
        self.btn_home.rect.x = wx + 108;
        self.btn_home.rect.y = wy + 34;
        self.btn_root.rect.x = wx + 180;
        self.btn_root.rect.y = wy + 34;
        self.btn_etc.rect.x = wx + 250;
        self.btn_etc.rect.y = wy + 34;
        self.btn_usr.rect.x = wx + 294;
        self.btn_usr.rect.y = wy + 34;
        self.btn_tmp.rect.x = wx + 338;
        self.btn_tmp.rect.y = wy + 34;
        self.btn_media.rect.x = wx + 382;
        self.btn_media.rect.y = wy + 34;

        // Position operation buttons
        self.btn_new_folder.rect.x = wx + 16;
        self.btn_new_folder.rect.y = wy + 60;
        self.btn_rename.rect.x = wx + 98;
        self.btn_rename.rect.y = wy + 60;
        self.btn_delete.rect.x = wx + 178;
        self.btn_delete.rect.y = wy + 60;
        self.btn_copy.rect.x = wx + 256;
        self.btn_copy.rect.y = wy + 60;
        self.btn_cut.rect.x = wx + 322;
        self.btn_cut.rect.y = wy + 60;
        self.btn_paste.rect.x = wx + 382;
        self.btn_paste.rect.y = wy + 60;
        self.btn_refresh.rect.x = wx + 450;
        self.btn_refresh.rect.y = wy + 60;

        // Position dialog buttons
        let dw = 340;
        let dh = 140;
        let dx = wx + (ww - dw) / 2;
        let dy = wy + (wh - dh) / 2;
        self.btn_dialog_ok.rect.x = dx + 80;
        self.btn_dialog_ok.rect.y = dy + dh - 36;
        self.btn_dialog_cancel.rect.x = dx + 190;
        self.btn_dialog_cancel.rect.y = dy + dh - 36;

        // If modal dialog is active, intercept clicks for dialog only
        if self.dialog != DialogMode::None {
            if self.btn_dialog_ok.is_clicked(mouse) {
                self.execute_dialog_ok();
            } else if self.btn_dialog_cancel.is_clicked(mouse) {
                self.dialog = DialogMode::None;
            }
            return None;
        }

        // Close error banner button
        if self.error_banner.is_some() && mouse.left_clicked {
            let banner_rect = Rect::new(wx + 16, wy + 114, ww - 32, 22);
            if banner_rect.contains(mouse.x, mouse.y) {
                self.error_banner = None;
            }
        }

        // Navigation clicks
        if self.btn_back.is_clicked(mouse) {
            self.go_back();
        } else if self.btn_forward.is_clicked(mouse) {
            self.go_forward();
        } else if self.btn_up.is_clicked(mouse) {
            if let Some(parent) = self.current_dir.parent() {
                self.navigate_to(parent.to_path_buf());
            }
        } else if self.btn_home.is_clicked(mouse) {
            self.navigate_to(PathBuf::from("/home/leviathan"));
        } else if self.btn_root.is_clicked(mouse) {
            self.navigate_to(PathBuf::from("/"));
        } else if self.btn_etc.is_clicked(mouse) {
            self.navigate_to(PathBuf::from("/etc"));
        } else if self.btn_usr.is_clicked(mouse) {
            self.navigate_to(PathBuf::from("/usr"));
        } else if self.btn_tmp.is_clicked(mouse) {
            self.navigate_to(PathBuf::from("/tmp"));
        } else if self.btn_media.is_clicked(mouse) {
            let media_path = if Path::new("/media").exists() {
                PathBuf::from("/media")
            } else {
                PathBuf::from("/mnt")
            };
            self.navigate_to(media_path);
        }

        // Operation clicks
        if self.btn_refresh.is_clicked(mouse) {
            self.error_banner = None;
            self.refresh();
        } else if self.btn_new_folder.is_clicked(mouse) {
            self.dialog = DialogMode::NewFolder {
                input: "New_Folder".to_string(),
            };
        } else if self.btn_rename.is_clicked(mouse) {
            if let Some(idx) = self.selected_idx {
                if let Some(item) = self.items.get(idx) {
                    self.dialog = DialogMode::Rename {
                        input: item.name.clone(),
                        original: item.name.clone(),
                    };
                }
            } else {
                self.error_banner = Some("Select an item to rename".to_string());
            }
        } else if self.btn_delete.is_clicked(mouse) {
            if let Some(idx) = self.selected_idx {
                if let Some(item) = self.items.get(idx) {
                    self.dialog = DialogMode::DeleteConfirm {
                        target: item.name.clone(),
                        is_dir: item.is_dir,
                    };
                }
            } else {
                self.error_banner = Some("Select an item to delete".to_string());
            }
        } else if self.btn_copy.is_clicked(mouse) {
            if let Some(idx) = self.selected_idx {
                if let Some(item) = self.items.get(idx) {
                    self.clipboard_path = Some(self.current_dir.join(&item.name));
                    self.clipboard_is_cut = false;
                    self.error_banner = None;
                }
            }
        } else if self.btn_cut.is_clicked(mouse) {
            if let Some(idx) = self.selected_idx {
                if let Some(item) = self.items.get(idx) {
                    self.clipboard_path = Some(self.current_dir.join(&item.name));
                    self.clipboard_is_cut = true;
                    self.error_banner = None;
                }
            }
        } else if self.btn_paste.is_clicked(mouse) {
            if let Some(ref src) = self.clipboard_path.clone() {
                if let Some(file_name) = src.file_name() {
                    let dst = self.current_dir.join(file_name);
                    if src == &dst {
                        self.error_banner =
                            Some("Source and destination are identical".to_string());
                    } else {
                        let is_cut = self.clipboard_is_cut;
                        let res = if is_cut {
                            fs::rename(src, &dst)
                        } else if src.is_dir() {
                            copy_dir_recursive(src, &dst)
                        } else {
                            fs::copy(src, &dst).map(|_| ())
                        };

                        match res {
                            Ok(()) => {
                                if is_cut {
                                    self.clipboard_path = None;
                                }
                                self.refresh();
                            }
                            Err(e) => {
                                self.error_banner = Some(format!("Paste failed: {}", e));
                            }
                        }
                    }
                }
            } else {
                self.error_banner = Some("Clipboard is empty".to_string());
            }
        }

        // Mouse wheel scrolling
        if mouse.scroll_delta != 0 {
            let visible_count = ((wh - 150) / 26).max(1) as usize;
            if mouse.scroll_delta > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(2);
            } else if self.scroll_offset + visible_count < self.items.len() {
                self.scroll_offset = (self.scroll_offset + 2).min(self.items.len().saturating_sub(1));
            }
        }

        // Item selection & double-click open
        let banner_offset = if self.error_banner.is_some() { 24 } else { 0 };
        let list_y = wy + 114 + banner_offset;
        let item_h = 24;
        let item_w = ww - 32;
        let visible_count = ((wh - 146 - banner_offset) / item_h).max(1) as usize;

        if mouse.left_clicked {
            for (i, item) in self
                .items
                .iter()
                .enumerate()
                .skip(self.scroll_offset)
                .take(visible_count)
            {
                let iy = list_y + ((i - self.scroll_offset) as i32 * item_h);
                let rect = Rect::new(wx + 16, iy, item_w, item_h);

                if rect.contains(mouse.x, mouse.y) {
                    let now = Instant::now();
                    let is_double_click = self.last_click_idx == Some(i)
                        && now.duration_since(self.last_click_time).as_millis() < 450;

                    self.selected_idx = Some(i);
                    self.last_click_idx = Some(i);
                    self.last_click_time = now;

                    if is_double_click {
                        if item.is_dir {
                            let new_dir = self.current_dir.join(&item.name);
                            self.navigate_to(new_dir);
                            return None;
                        } else {
                            let full_path = self.current_dir.join(&item.name);
                            return Some(full_path.to_string_lossy().to_string());
                        }
                    }
                    break;
                }
            }
        }

        None
    }

    pub fn draw(&mut self, gfx: &mut Graphics, mouse: &MouseState) {
        if !self.window.is_open {
            return;
        }

        self.window.draw_frame(gfx, mouse);

        let wx = self.window.rect.x;
        let wy = self.window.rect.y;
        let ww = self.window.rect.w;
        let wh = self.window.rect.h;

        // Navigation toolbar
        self.btn_back.draw(gfx, mouse);
        self.btn_forward.draw(gfx, mouse);
        self.btn_up.draw(gfx, mouse);
        self.btn_home.draw(gfx, mouse);
        self.btn_root.draw(gfx, mouse);
        self.btn_etc.draw(gfx, mouse);
        self.btn_usr.draw(gfx, mouse);
        self.btn_tmp.draw(gfx, mouse);
        self.btn_media.draw(gfx, mouse);

        // Action toolbar
        self.btn_new_folder.draw(gfx, mouse);
        self.btn_rename.draw(gfx, mouse);
        self.btn_delete.draw(gfx, mouse);
        self.btn_copy.draw(gfx, mouse);
        self.btn_cut.draw(gfx, mouse);

        // Paste button styling (active or dim)
        if self.clipboard_path.is_some() {
            self.btn_paste.bg_color = Color::new(30, 60, 90);
        } else {
            self.btn_paste.bg_color = Color::MACOS_TITLEBAR;
        }
        self.btn_paste.draw(gfx, mouse);
        self.btn_refresh.draw(gfx, mouse);

        // Path address bar
        let path_bar = Rect::new(wx + 16, wy + 86, ww - 32, 22);
        path_bar.draw_fill(gfx, Color::MACOS_TITLEBAR);
        path_bar.draw_outline(gfx, Color::MACOS_BORDER);
        let path_str = self.current_dir.to_string_lossy();
        gfx.draw_string(
            (wx + 22) as u32,
            (wy + 91) as u32,
            &path_str,
            Color::new(0, 220, 240),
        );

        // Error banner if any
        let mut banner_offset = 0;
        if let Some(ref err) = self.error_banner {
            banner_offset = 24;
            let err_rect = Rect::new(wx + 16, wy + 112, ww - 32, 20);
            err_rect.draw_fill(gfx, Color::new(90, 20, 25));
            err_rect.draw_outline(gfx, Color::new(220, 50, 60));
            gfx.draw_string(
                (wx + 24) as u32,
                (wy + 116) as u32,
                err,
                Color::new(255, 200, 200),
            );
            gfx.draw_string(
                (wx + ww - 36) as u32,
                (wy + 116) as u32,
                "[x]",
                Color::new(255, 100, 100),
            );
        }

        // File list table headers
        let list_y = wy + 112 + banner_offset;
        let header_rect = Rect::new(wx + 16, list_y, ww - 32, 20);
        header_rect.draw_fill(gfx, Color::new(24, 26, 32));
        header_rect.draw_outline(gfx, Color::MACOS_BORDER);

        gfx.draw_string(
            (wx + 24) as u32,
            (list_y + 4) as u32,
            "Name",
            Color::MACOS_TEXT_SECONDARY,
        );
        gfx.draw_string(
            (wx + ww - 260) as u32,
            (list_y + 4) as u32,
            "Size",
            Color::MACOS_TEXT_SECONDARY,
        );
        gfx.draw_string(
            (wx + ww - 180) as u32,
            (list_y + 4) as u32,
            "Mode",
            Color::MACOS_TEXT_SECONDARY,
        );
        gfx.draw_string(
            (wx + ww - 95) as u32,
            (list_y + 4) as u32,
            "Date",
            Color::MACOS_TEXT_SECONDARY,
        );

        // File list rows
        let row_start_y = list_y + 22;
        let item_h = 24;
        let item_w = ww - 32;
        let visible_count = ((wh - 160 - banner_offset) / item_h).max(1) as usize;

        if self.items.is_empty() {
            gfx.draw_string(
                (wx + 28) as u32,
                (row_start_y + 15) as u32,
                "(Empty Directory)",
                Color::MACOS_TEXT_SECONDARY,
            );
        } else {
            for (i, item) in self
                .items
                .iter()
                .enumerate()
                .skip(self.scroll_offset)
                .take(visible_count)
            {
                let iy = row_start_y + ((i - self.scroll_offset) as i32 * item_h);
                let rect = Rect::new(wx + 16, iy, item_w, item_h - 2);

                let is_hover = rect.contains(mouse.x, mouse.y);
                let is_sel = self.selected_idx == Some(i);

                let bg = if is_sel {
                    Color::new(25, 45, 65)
                } else if is_hover {
                    Color::MACOS_HOVER
                } else {
                    Color::MACOS_WINDOW_BG
                };

                let border = if is_sel {
                    Color::new(0, 190, 210)
                } else if is_hover {
                    Color::MACOS_BORDER_ACTIVE
                } else {
                    Color::new(20, 22, 28)
                };

                rect.draw_fill(gfx, bg);
                rect.draw_outline(gfx, border);

                // Type Icon & Name
                let (icon_id, icon_col) = if item.is_dir {
                    (IconId::Folder, Color::new(0, 190, 210))
                } else if item.is_exe {
                    (IconId::Terminal, Color::new(50, 215, 75))
                } else if item.name.ends_with(".deb") {
                    (IconId::Packages, Color::new(255, 179, 64))
                } else {
                    (IconId::File, Color::MACOS_TEXT_SECONDARY)
                };

                draw_icon(gfx, icon_id, rect.x + 8, rect.y + 4, 16, icon_col);

                let max_name_len = ((item_w - 270) / 8).max(10) as usize;
                let display_name = if item.name.len() > max_name_len {
                    format!("{}...", &item.name[..max_name_len.saturating_sub(3)])
                } else {
                    item.name.clone()
                };

                gfx.draw_string(
                    (rect.x + 30) as u32,
                    (rect.y + 5) as u32,
                    &display_name,
                    if is_sel {
                        Color::new(255, 255, 255)
                    } else {
                        Color::MACOS_TEXT
                    },
                );

                // Size
                let size_str = if item.is_dir {
                    "<DIR>".to_string()
                } else if item.size_bytes >= 1024 * 1024 {
                    format!("{:.1} MB", item.size_bytes as f64 / (1024.0 * 1024.0))
                } else if item.size_bytes >= 1024 {
                    format!("{} KB", item.size_bytes / 1024)
                } else {
                    format!("{} B", item.size_bytes)
                };
                gfx.draw_string(
                    (rect.x + item_w - 260) as u32,
                    (rect.y + 4) as u32,
                    &size_str,
                    Color::MACOS_TEXT_SECONDARY,
                );

                // Unix Permissions
                let perm_str = format_mode(item.permissions, item.is_dir);
                gfx.draw_string(
                    (rect.x + item_w - 180) as u32,
                    (rect.y + 4) as u32,
                    &perm_str,
                    Color::MACOS_TEXT_SECONDARY,
                );

                // Modified Date
                let date_short = if item.modified_str.len() >= 10 {
                    &item.modified_str[5..10]
                } else {
                    &item.modified_str
                };
                gfx.draw_string(
                    (rect.x + item_w - 75) as u32,
                    (rect.y + 4) as u32,
                    date_short,
                    Color::MACOS_TEXT_SECONDARY,
                );
            }
        }

        // Status bar at bottom
        let status_bar_y = wy + wh - 22;
        let status_rect = Rect::new(wx, status_bar_y, ww, 22);
        status_rect.draw_fill(gfx, Color::new(20, 22, 28));

        let status_text = if let Some(idx) = self.selected_idx {
            if let Some(item) = self.items.get(idx) {
                format!(
                    "\"{}\" | {} | {} | {}",
                    item.name,
                    if item.is_dir {
                        "<DIR>".into()
                    } else {
                        format!("{} bytes", item.size_bytes)
                    },
                    format_mode(item.permissions, item.is_dir),
                    item.modified_str
                )
            } else {
                format!("{} items", self.items.len())
            }
        } else {
            let clip_str = if let Some(ref c) = self.clipboard_path {
                format!(
                    " | Clip: {} ({})",
                    c.file_name().unwrap_or_default().to_string_lossy(),
                    if self.clipboard_is_cut {
                        "Cut"
                    } else {
                        "Copy"
                    }
                )
            } else {
                String::new()
            };
            format!("{} items in directory{}", self.items.len(), clip_str)
        };
        gfx.draw_string(
            (wx + 16) as u32,
            (status_bar_y + 4) as u32,
            &status_text,
            Color::MACOS_TEXT_SECONDARY,
        );

        // Modal Dialog Overlay
        if self.dialog != DialogMode::None {
            let dw = 340;
            let dh = 140;
            let dx = wx + (ww - dw) / 2;
            let dy = wy + (wh - dh) / 2;

            let overlay_rect = Rect::new(dx, dy, dw, dh);
            overlay_rect.draw_fill(gfx, Color::new(28, 30, 38));
            overlay_rect.draw_outline(gfx, Color::new(0, 190, 210));

            match &self.dialog {
                DialogMode::NewFolder { input } => {
                    gfx.draw_string(
                        (dx + 20) as u32,
                        (dy + 16) as u32,
                        "Create New Folder",
                        Color::MACOS_TEXT,
                    );
                    gfx.draw_string(
                        (dx + 20) as u32,
                        (dy + 38) as u32,
                        "Enter folder name:",
                        Color::MACOS_TEXT_SECONDARY,
                    );

                    let input_rect = Rect::new(dx + 20, dy + 58, dw - 40, 26);
                    input_rect.draw_fill(gfx, Color::new(18, 20, 26));
                    input_rect.draw_outline(gfx, Color::new(0, 190, 210));
                    let display_input = format!("{}_", input);
                    gfx.draw_string(
                        (dx + 28) as u32,
                        (dy + 65) as u32,
                        &display_input,
                        Color::MACOS_TEXT,
                    );
                }
                DialogMode::Rename { input, original } => {
                    gfx.draw_string(
                        (dx + 20) as u32,
                        (dy + 16) as u32,
                        "Rename Item",
                        Color::MACOS_TEXT,
                    );
                    let orig_label = format!("Original: {}", original);
                    gfx.draw_string(
                        (dx + 20) as u32,
                        (dy + 38) as u32,
                        &orig_label,
                        Color::MACOS_TEXT_SECONDARY,
                    );

                    let input_rect = Rect::new(dx + 20, dy + 58, dw - 40, 26);
                    input_rect.draw_fill(gfx, Color::new(18, 20, 26));
                    input_rect.draw_outline(gfx, Color::new(0, 190, 210));
                    let display_input = format!("{}_", input);
                    gfx.draw_string(
                        (dx + 28) as u32,
                        (dy + 65) as u32,
                        &display_input,
                        Color::MACOS_TEXT,
                    );
                }
                DialogMode::DeleteConfirm { target, is_dir } => {
                    gfx.draw_string(
                        (dx + 20) as u32,
                        (dy + 16) as u32,
                        "Confirm Permanent Delete",
                        Color::new(255, 80, 80),
                    );
                    let msg = format!(
                        "Delete {} \"{}\"?",
                        if *is_dir { "folder" } else { "file" },
                        target
                    );
                    gfx.draw_string((dx + 20) as u32, (dy + 42) as u32, &msg, Color::MACOS_TEXT);
                    gfx.draw_string(
                        (dx + 20) as u32,
                        (dy + 64) as u32,
                        "This action cannot be undone.",
                        Color::MACOS_TEXT_SECONDARY,
                    );
                }
                DialogMode::None => {}
            }

            self.btn_dialog_ok.draw(gfx, mouse);
            self.btn_dialog_cancel.draw(gfx, mouse);
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let dst_child = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_child)?;
        } else {
            fs::copy(entry.path(), dst_child)?;
        }
    }
    Ok(())
}

fn format_mode(mode: u32, is_dir: bool) -> String {
    let mut s = String::with_capacity(10);
    s.push(if is_dir { 'd' } else { '-' });
    s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o100 != 0 { 'x' } else { '-' });
    s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o010 != 0 { 'x' } else { '-' });
    s.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o001 != 0 { 'x' } else { '-' });
    s
}
