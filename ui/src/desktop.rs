use crate::app_files::FilesApp;
use crate::app_packages::PackagesApp;
use crate::app_settings::SettingsApp;
use crate::app_terminal::TerminalApp;
use crate::color::Color;
use crate::desktop_entry::{AppEntry, AppScanner};
use crate::dock::{DockAppId, FloatingDock};
use crate::graphics::Graphics;
use crate::icons::{draw_icon, IconId};
use crate::input::Action;
use crate::layer_editor::LayerEditorApp;
use crate::mouse::{Mouse, MouseState};
use crate::widget::Rect;
use crate::window_manager::WindowManager;
use std::fs;
use std::time::Instant;

pub struct Desktop {
    pub terminal: TerminalApp,
    pub settings: SettingsApp,
    pub packages: PackagesApp,
    pub files: FilesApp,
    pub layer_editor: LayerEditorApp,
    pub wm: WindowManager,
    pub dock: FloatingDock,
    pub mouse: Mouse,
    pub active_app: Option<DockAppId>,
    pub show_3d_viewport: bool,
    pub launcher_search: String,
    pub launcher_category: usize,
    start_time: Instant,
    last_frame_time: Instant,
    cached_apps: Vec<AppEntry>,
    mem_info_cache: String,
    last_mem_check: Instant,
    pub is_low_mem: bool,
    pub is_net_online: bool,
    last_net_check: Instant,
    wallpaper_cache: Vec<u8>,
    cached_wallpaper_idx: usize,
    cached_ui_scale: f32,
}

impl Desktop {
    pub fn new(width: u32, height: u32) -> Self {
        let mouse = Mouse::new(width, height);
        let dock = FloatingDock::new(width, height);

        // Position Terminal window on left
        let term_w = 600;
        let term_h = 420;
        let term_x = 40;
        let term_y = 50;
        let mut terminal = TerminalApp::new(term_x, term_y, term_w, term_h);
        terminal.window.is_open = false;
        terminal.window.is_focused = false;

        // Position Settings window
        let set_w = 540;
        let set_h = 400;
        let set_x = width as i32 - set_w - 40;
        let set_y = 70;
        let mut settings = SettingsApp::new(set_x, set_y, set_w, set_h);
        settings.window.is_open = false;
        settings.window.is_focused = false;

        // Position Packages window in center
        let pkg_w = 660;
        let pkg_h = 440;
        let pkg_x = (width as i32 - pkg_w) / 2;
        let pkg_y = 55;
        let mut packages = PackagesApp::new(pkg_x, pkg_y, pkg_w as u32, pkg_h as u32);
        packages.window.is_open = false;
        packages.window.is_focused = false;

        // Position Files window
        let files_w = 560;
        let files_h = 380;
        let files_x = 80;
        let files_y = 90;
        let mut files = FilesApp::new(files_x, files_y, files_w, files_h);
        files.window.is_open = false;
        files.window.is_focused = false;

        // Layer Editor
        let mut layer_editor = LayerEditorApp::new(width, height);
        layer_editor.window.is_open = false;

        let cached_apps = AppScanner::scan_applications();

        Self {
            terminal,
            settings,
            packages,
            files,
            layer_editor,
            wm: WindowManager::new(),
            dock,
            mouse,
            active_app: None,
            show_3d_viewport: false,
            launcher_search: String::new(),
            launcher_category: 0,
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            cached_apps,
            mem_info_cache: "RAM: 128M / 2048M".to_string(),
            last_mem_check: Instant::now(),
            is_low_mem: false,
            is_net_online: true,
            last_net_check: Instant::now(),
            wallpaper_cache: Vec::new(),
            cached_wallpaper_idx: usize::MAX,
            cached_ui_scale: 1.0,
        }
    }

    fn read_memory_usage(&mut self) {
        if self.last_mem_check.elapsed().as_secs_f32() < 3.0 {
            return;
        }
        self.last_mem_check = Instant::now();

        if let Ok(content) = fs::read_to_string("/proc/meminfo") {
            let mut total_kb: u64 = 0;
            let mut free_kb: u64 = 0;
            let mut available_kb: u64 = 0;

            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    available_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                } else if line.starts_with("MemFree:") && available_kb == 0 {
                    free_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                }
            }

            let avail = if available_kb > 0 { available_kb } else { free_kb };
            let used_mb = total_kb.saturating_sub(avail) / 1024;
            let total_mb = total_kb / 1024;
            self.is_low_mem = avail < 64 * 1024;
            if total_mb > 0 {
                self.mem_info_cache = format!("RAM: {}M / {}M", used_mb, total_mb);
            }
        }
    }

    fn check_network_status(&mut self) {
        if self.last_net_check.elapsed().as_secs_f32() < 4.0 {
            return;
        }
        self.last_net_check = Instant::now();
        let mut is_online = false;
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != "lo" {
                    if let Ok(state) = fs::read_to_string(entry.path().join("operstate")) {
                        if state.trim().eq_ignore_ascii_case("up") {
                            is_online = true;
                            break;
                        }
                    }
                }
            }
        }
        self.is_net_online = is_online;
    }

    fn render_wallpaper_cache(&mut self, gfx: &Graphics) {
        let width = gfx.width();
        let height = gfx.height();
        let (col_top, col_bot) = self.settings.get_wallpaper_colors();
        let mut buffer = vec![0u8; gfx.buffer_size()];
        let bpp = gfx.bytes_per_pixel() as usize;
        let pitch = gfx.pitch() as usize;

        for y in 0..height {
            let ratio = y as f32 / height as f32;
            let r = ((1.0 - ratio) * col_top.r as f32 + ratio * col_bot.r as f32) as u8;
            let g = ((1.0 - ratio) * col_top.g as f32 + ratio * col_bot.g as f32) as u8;
            let b = ((1.0 - ratio) * col_top.b as f32 + ratio * col_bot.b as f32) as u8;
            let row_color = Color::new(r, g, b);
            let pval = gfx.pack_color(row_color);
            let bytes = pval.to_ne_bytes();
            let row_offset = (y as usize) * pitch;

            if bpp == 4 {
                for x in 0..width {
                    let off = row_offset + (x as usize) * 4;
                    if off + 4 <= buffer.len() {
                        buffer[off..off + 4].copy_from_slice(&bytes);
                    }
                }
            } else {
                for x in 0..width {
                    let off = row_offset + (x as usize) * bpp;
                    if off + bpp <= buffer.len() {
                        buffer[off..off + bpp].copy_from_slice(&bytes[..bpp]);
                    }
                }
            }
        }
        self.wallpaper_cache = buffer;
        self.cached_wallpaper_idx = self.settings.wallpaper_idx;
    }

    pub fn update(&mut self, width: u32, height: u32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        let time_sec = self.start_time.elapsed().as_secs_f32();

        self.read_memory_usage();
        self.check_network_status();

        let mouse_state = self.mouse.update();
        let sw = width as i32;
        let sh = height as i32;

        self.mouse.sensitivity = self.settings.mouse_sensitivity;
        self.mouse.set_resolution(width, height);

        if (self.settings.ui_scale - self.cached_ui_scale).abs() > 0.01 {
            self.cached_ui_scale = self.settings.ui_scale;
            self.dock.set_layout(width, height, self.settings.ui_scale);
        }

        // 1. Handle Floating Dock clicks
        if let Some(app_id) = self.dock.handle_click(&mouse_state) {
            match app_id {
                DockAppId::AllApps => {
                    if self.dock.is_all_apps_open {
                        self.cached_apps = AppScanner::scan_applications();
                        self.launcher_search.clear();
                    }
                }
                DockAppId::Terminal => {
                    self.terminal.window.is_open = true;
                    self.terminal.window.is_focused = true;
                    self.packages.window.is_focused = false;
                    self.settings.window.is_focused = false;
                    self.files.window.is_focused = false;
                    self.active_app = Some(DockAppId::Terminal);
                }
                DockAppId::Packages => {
                    self.packages.window.is_open = true;
                    self.packages.window.is_focused = true;
                    self.terminal.window.is_focused = false;
                    self.settings.window.is_focused = false;
                    self.files.window.is_focused = false;
                    self.active_app = Some(DockAppId::Packages);
                }
                DockAppId::Settings => {
                    self.settings.window.is_open = true;
                    self.settings.window.is_focused = true;
                    self.terminal.window.is_focused = false;
                    self.packages.window.is_focused = false;
                    self.files.window.is_focused = false;
                    self.active_app = Some(DockAppId::Settings);
                }
                DockAppId::Files => {
                    self.files.window.is_open = true;
                    self.files.window.is_focused = true;
                    self.terminal.window.is_focused = false;
                    self.packages.window.is_focused = false;
                    self.settings.window.is_focused = false;
                    self.active_app = Some(DockAppId::Files);
                }
                DockAppId::Abyss3D => {
                    self.show_3d_viewport = !self.show_3d_viewport;
                }
                DockAppId::LayerEditor => {
                    if self.layer_editor.window.is_open {
                        self.layer_editor.close();
                    } else {
                        self.layer_editor.open();
                        self.terminal.window.is_focused = false;
                        self.packages.window.is_focused = false;
                        self.settings.window.is_focused = false;
                        self.files.window.is_focused = false;
                    }
                }
            }
        }

        // 2. Handle All Apps Fugo Launcher clicks
        if self.dock.is_all_apps_open && mouse_state.left_clicked {
            let dw = 580;
            let dh = 380;
            let dx = (sw - dw) / 2;
            let dy = (sh - dh) / 2;
            let drawer_rect = Rect::new(dx, dy, dw, dh);

            if !drawer_rect.contains(mouse_state.x, mouse_state.y)
                && !self.dock.rect.contains(mouse_state.x, mouse_state.y)
            {
                self.dock.is_all_apps_open = false;
            } else {
                // Category tabs
                let cats = ["All", "System", "Internet", "Utilities"];
                for (i, _) in cats.iter().enumerate() {
                    let tab_rect = Rect::new(dx + 24 + (i as i32 * 85), dy + 70, 78, 22);
                    if tab_rect.contains(mouse_state.x, mouse_state.y) {
                        self.launcher_category = i;
                    }
                }

                // Check app card clicks
                let filtered: Vec<&AppEntry> = self
                    .cached_apps
                    .iter()
                    .filter(|app| {
                        let matches_cat = match self.launcher_category {
                            1 => app.categories.contains("System") || app.categories.contains("Settings") || app.name == "Settings" || app.name == "Files" || app.name == "Terminal",
                            2 => app.categories.contains("Network") || app.categories.contains("WebBrowser") || app.name.to_lowercase().contains("browser") || app.name.to_lowercase().contains("firefox") || app.name.to_lowercase().contains("net"),
                            3 => app.categories.contains("Utility") || app.categories.contains("Development") || app.categories.contains("Graphics") || app.name == "Packages" || app.name == "3D Abyss" || app.name == "htop",
                            _ => true,
                        };
                        let matches_search = self.launcher_search.is_empty()
                            || app.name.to_lowercase().contains(&self.launcher_search.to_lowercase())
                            || app.comment.to_lowercase().contains(&self.launcher_search.to_lowercase());
                        matches_cat && matches_search
                    })
                    .collect();

                let tile_w = 124;
                let tile_h = 80;
                for (i, app) in filtered.iter().enumerate().take(8) {
                    let col = (i % 4) as i32;
                    let row = (i / 4) as i32;
                    let tx = dx + 24 + col * (tile_w + 14);
                    let ty = dy + 104 + row * (tile_h + 14);
                    let tile_rect = Rect::new(tx, ty, tile_w, tile_h);

                    if tile_rect.contains(mouse_state.x, mouse_state.y) {
                        self.dock.is_all_apps_open = false;
                        match app.name.as_str() {
                            "Terminal" => {
                                self.terminal.window.is_open = true;
                                self.terminal.window.is_focused = true;
                                self.packages.window.is_focused = false;
                                self.settings.window.is_focused = false;
                                self.files.window.is_focused = false;
                            }
                            "Packages" => {
                                self.packages.window.is_open = true;
                                self.packages.window.is_focused = true;
                                self.terminal.window.is_focused = false;
                                self.settings.window.is_focused = false;
                                self.files.window.is_focused = false;
                            }
                            "Settings" => {
                                self.settings.window.is_open = true;
                                self.settings.window.is_focused = true;
                                self.terminal.window.is_focused = false;
                                self.packages.window.is_focused = false;
                                self.files.window.is_focused = false;
                            }
                            "Files" => {
                                self.files.window.is_open = true;
                                self.files.window.is_focused = true;
                                self.terminal.window.is_focused = false;
                                self.packages.window.is_focused = false;
                                self.settings.window.is_focused = false;
                            }
                            "3D Abyss" => {
                                self.show_3d_viewport = !self.show_3d_viewport;
                            }
                            _ => {
                                let (bin, args) = AppScanner::parse_exec_line(&app.exec, &app.name, &app.icon);
                                if app.terminal {
                                    // For Terminal=true apps, find a real terminal emulator and
                                    // invoke the command inside it rather than the built-in widget.
                                    let term = Self::find_terminal_emulator();
                                    if let Some(ref term_bin) = term {
                                        let mut term_args: Vec<String> = vec!["-e".to_string(), bin.clone()];
                                        term_args.extend_from_slice(&args);
                                        let arg_refs: Vec<&str> = term_args.iter().map(|s| s.as_str()).collect();
                                        self.wm.spawn_app(term_bin, &arg_refs, &app.name, 720, 420, sw, sh);
                                    } else {
                                        // Fall back to Leviathan's built-in terminal
                                        self.terminal.window.is_open = true;
                                        self.terminal.window.is_focused = true;
                                        self.terminal.input_line = bin.clone();
                                        self.terminal.execute_command();
                                    }
                                } else {
                                    let cfg = crate::window_manager::AppLaunchConfig {
                                        cmd: bin,
                                        args,
                                        title: app.name.clone(),
                                        width: 720,
                                        height: 420,
                                        use_sandbox: false,
                                        run_as_user: true,
                                        working_dir: app.path.clone(),
                                    };
                                    self.wm.spawn_app_with_config(&cfg, sw, sh);
                                }
                            }

                        }
                        break;
                    }
                }
            }
        }

        // 3. Focus switching between open windows (including external WM windows)
        if mouse_state.left_clicked {
            let external_focused_clicked = self.wm.has_focused_window_at(mouse_state.x, mouse_state.y);
            let builtin_focused_clicked = (self.packages.window.is_open && self.packages.window.is_focused && self.packages.window.rect.contains(mouse_state.x, mouse_state.y))
                || (self.terminal.window.is_open && self.terminal.window.is_focused && self.terminal.window.rect.contains(mouse_state.x, mouse_state.y))
                || (self.files.window.is_open && self.files.window.is_focused && self.files.window.rect.contains(mouse_state.x, mouse_state.y))
                || (self.settings.window.is_open && self.settings.window.is_focused && self.settings.window.rect.contains(mouse_state.x, mouse_state.y));

            if !external_focused_clicked && !builtin_focused_clicked {
                if self.wm.try_focus_at(mouse_state.x, mouse_state.y) {
                    // An external app window got focus, defocus all builtins
                    self.packages.window.is_focused = false;
                    self.terminal.window.is_focused = false;
                    self.settings.window.is_focused = false;
                    self.files.window.is_focused = false;
                } else if self.packages.window.is_open && self.packages.window.rect.contains(mouse_state.x, mouse_state.y) {
                    self.packages.window.is_focused = true;
                    self.terminal.window.is_focused = false;
                    self.settings.window.is_focused = false;
                    self.files.window.is_focused = false;
                    self.wm.try_focus_at(-1, -1);
                } else if self.terminal.window.is_open && self.terminal.window.rect.contains(mouse_state.x, mouse_state.y) {
                    self.terminal.window.is_focused = true;
                    self.packages.window.is_focused = false;
                    self.settings.window.is_focused = false;
                    self.files.window.is_focused = false;
                    self.wm.try_focus_at(-1, -1);
                } else if self.files.window.is_open && self.files.window.rect.contains(mouse_state.x, mouse_state.y) {
                    self.files.window.is_focused = true;
                    self.terminal.window.is_focused = false;
                    self.packages.window.is_focused = false;
                    self.settings.window.is_focused = false;
                    self.wm.try_focus_at(-1, -1);
                } else if self.settings.window.is_open && self.settings.window.rect.contains(mouse_state.x, mouse_state.y) {
                    self.settings.window.is_focused = true;
                    self.terminal.window.is_focused = false;
                    self.packages.window.is_focused = false;
                    self.files.window.is_focused = false;
                    self.wm.try_focus_at(-1, -1);
                }
            }
        }

        // Poll pending GUI launch from terminal (e.g. user typed 'firefox' in terminal)
        if let Some((cmd, args, title)) = self.terminal.pending_gui_launch.take() {
            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            self.wm.spawn_app(&cmd, &arg_refs, &title, 760, 440, sw, sh);
        }

        // 4. Update running apps and window manager
        self.wm.update(&mouse_state, sw, sh);

        if self.terminal.window.is_open {
            self.terminal.update(&mouse_state, sw, sh, dt);
        }
        if self.packages.window.is_open {
            self.packages.update(&mouse_state, sw, sh);
            if self.packages.packages_changed {
                self.packages.packages_changed = false;
                self.cached_apps = AppScanner::scan_applications();
            }
        }
        if self.files.window.is_open {
            if let Some(exe) = self.files.update(&mouse_state, sw, sh) {
                if exe.ends_with(".deb") {
                    self.terminal.window.is_open = true;
                    self.terminal.window.is_focused = true;
                    self.terminal.input_line = format!("sudo apt-get install -y '{}'", exe);
                    self.terminal.execute_command();
                } else {
                    self.terminal.window.is_open = true;
                    self.terminal.window.is_focused = true;
                    self.terminal.input_line = exe;
                    self.terminal.execute_command();
                }
            }
        }
        if self.settings.window.is_open {
            self.settings.update(&mouse_state, sw, sh, time_sec);
        }
        if self.layer_editor.window.is_open {
            self.layer_editor.update(&mouse_state, sw, sh);
        }

        // Update running dots on dock
        self.dock.items[1].is_running = self.terminal.window.is_open;
        self.dock.items[2].is_running = self.packages.window.is_open;
        self.dock.items[3].is_running = self.settings.window.is_open;
        self.dock.items[4].is_running = self.files.window.is_open;
        self.dock.items[5].is_running = self.show_3d_viewport;
        if self.dock.items.len() > 6 {
            self.dock.items[6].is_running = self.layer_editor.window.is_open;
        }
    }

    /// Return the path of an available terminal emulator, or None if none is found.
    fn find_terminal_emulator() -> Option<String> {
        let candidates = [
            "/usr/bin/kitty",
            "/usr/bin/alacritty",
            "/usr/bin/foot",
            "/usr/bin/xterm",
            "/usr/bin/konsole",
            "/usr/bin/gnome-terminal",
        ];
        candidates.iter()
            .find(|p| std::path::Path::new(p).exists())
            .map(|p| p.to_string())
    }

    pub fn defocus_all(&mut self) {
        self.terminal.window.is_focused = false;
        self.files.window.is_focused = false;
        self.settings.window.is_focused = false;
        self.packages.window.is_focused = false;
        self.wm.try_focus_at(-1, -1);
    }

    pub fn cycle_window_focus(&mut self) {
        let mut targets: Vec<usize> = Vec::new();
        if self.terminal.window.is_open && !self.terminal.window.is_minimized {
            targets.push(0);
        }
        if self.files.window.is_open && !self.files.window.is_minimized {
            targets.push(1);
        }
        if self.settings.window.is_open && !self.settings.window.is_minimized {
            targets.push(2);
        }
        if self.packages.window.is_open && !self.packages.window.is_minimized {
            targets.push(3);
        }
        for (i, w) in self.wm.windows.iter().enumerate() {
            if w.mapped && !w.is_closed {
                targets.push(4 + i);
            }
        }

        if targets.len() <= 1 {
            return;
        }

        let cur_target = if self.terminal.window.is_focused {
            Some(0)
        } else if self.files.window.is_focused {
            Some(1)
        } else if self.settings.window.is_focused {
            Some(2)
        } else if self.packages.window.is_focused {
            Some(3)
        } else if let Some(id) = self.wm.focused_id {
            self.wm.windows.iter().position(|w| w.id == id).map(|p| 4 + p)
        } else {
            None
        };

        let cur_idx = cur_target.and_then(|t| targets.iter().position(|&x| x == t)).unwrap_or(0);
        let next_idx = (cur_idx + 1) % targets.len();
        let next_target = targets[next_idx];

        self.defocus_all();
        match next_target {
            0 => self.terminal.window.is_focused = true,
            1 => self.files.window.is_focused = true,
            2 => self.settings.window.is_focused = true,
            3 => self.packages.window.is_focused = true,
            n => {
                let wm_idx = n - 4;
                if let Some(w) = self.wm.windows.get(wm_idx) {
                    let id = w.id;
                    self.wm.focus_window(id);
                    self.wm.raise_to_front(id);
                }
            }
        }
    }

    pub fn close_focused_window(&mut self) {
        if self.wm.has_any_focused() {
            self.wm.close_focused();
        } else if self.terminal.window.is_focused && self.terminal.window.is_open {
            self.terminal.window.is_open = false;
        } else if self.files.window.is_focused && self.files.window.is_open {
            self.files.window.is_open = false;
        } else if self.settings.window.is_focused && self.settings.window.is_open {
            self.settings.window.is_open = false;
        } else if self.packages.window.is_focused && self.packages.window.is_open {
            self.packages.window.is_open = false;
        }
    }

    pub fn minimize_focused_window(&mut self) {
        if self.wm.has_any_focused() {
            self.wm.minimize_focused();
        } else if self.terminal.window.is_focused && self.terminal.window.is_open {
            self.terminal.window.is_minimized = true;
            self.terminal.window.is_focused = false;
        } else if self.files.window.is_focused && self.files.window.is_open {
            self.files.window.is_minimized = true;
            self.files.window.is_focused = false;
        } else if self.settings.window.is_focused && self.settings.window.is_open {
            self.settings.window.is_minimized = true;
            self.settings.window.is_focused = false;
        } else if self.packages.window.is_focused && self.packages.window.is_open {
            self.packages.window.is_minimized = true;
            self.packages.window.is_focused = false;
        }
    }

    pub fn maximize_focused_window(&mut self, sw: i32, sh: i32) {
        if self.wm.has_any_focused() {
            self.wm.maximize_focused(sw, sh);
        } else if self.terminal.window.is_focused && self.terminal.window.is_open {
            if !self.terminal.window.is_maximized {
                self.terminal.window.toggle_maximize(sw, sh);
            }
        } else if self.files.window.is_focused && self.files.window.is_open {
            if !self.files.window.is_maximized {
                self.files.window.toggle_maximize(sw, sh);
            }
        } else if self.settings.window.is_focused && self.settings.window.is_open {
            if !self.settings.window.is_maximized {
                self.settings.window.toggle_maximize(sw, sh);
            }
        } else if self.packages.window.is_focused && self.packages.window.is_open {
            if !self.packages.window.is_maximized {
                self.packages.window.toggle_maximize(sw, sh);
            }
        }
    }

    pub fn restore_or_minimize_focused_window(&mut self, sw: i32, sh: i32) {
        if self.wm.has_any_focused() {
            self.wm.restore_or_minimize_focused(sw, sh);
        } else if self.terminal.window.is_focused && self.terminal.window.is_open {
            if self.terminal.window.is_maximized {
                self.terminal.window.toggle_maximize(sw, sh);
            } else {
                self.terminal.window.is_minimized = true;
                self.terminal.window.is_focused = false;
            }
        } else if self.files.window.is_focused && self.files.window.is_open {
            if self.files.window.is_maximized {
                self.files.window.toggle_maximize(sw, sh);
            } else {
                self.files.window.is_minimized = true;
                self.files.window.is_focused = false;
            }
        } else if self.settings.window.is_focused && self.settings.window.is_open {
            if self.settings.window.is_maximized {
                self.settings.window.toggle_maximize(sw, sh);
            } else {
                self.settings.window.is_minimized = true;
                self.settings.window.is_focused = false;
            }
        } else if self.packages.window.is_focused && self.packages.window.is_open {
            if self.packages.window.is_maximized {
                self.packages.window.toggle_maximize(sw, sh);
            } else {
                self.packages.window.is_minimized = true;
                self.packages.window.is_focused = false;
            }
        }
    }

    pub fn handle_action(&mut self, action: Action, sw: i32, sh: i32) {
        match action {
            Action::Super => {
                self.dock.is_all_apps_open = !self.dock.is_all_apps_open;
                if self.dock.is_all_apps_open {
                    self.cached_apps = AppScanner::scan_applications();
                    self.launcher_search.clear();
                }
            }
            Action::SuperT => {
                self.terminal.window.is_open = true;
                self.terminal.window.is_minimized = false;
                self.defocus_all();
                self.terminal.window.is_focused = true;
            }
            Action::SuperE => {
                self.files.window.is_open = true;
                self.files.window.is_minimized = false;
                self.defocus_all();
                self.files.window.is_focused = true;
            }
            Action::AltTab => {
                self.cycle_window_focus();
            }
            Action::AltF4 => {
                self.close_focused_window();
            }
            Action::SuperM => {
                self.minimize_focused_window();
            }
            Action::SuperUp => {
                self.maximize_focused_window(sw, sh);
            }
            Action::SuperDown => {
                self.restore_or_minimize_focused_window(sw, sh);
            }
            Action::Char(b) => {
                self.handle_key(b);
            }
            Action::ArrowUp | Action::ArrowDown | Action::ArrowLeft | Action::ArrowRight |
            Action::Home | Action::End | Action::PageUp | Action::PageDown => {
                if self.terminal.window.is_open && self.terminal.window.is_focused {
                    self.terminal.handle_action(action);
                } else if self.files.window.is_open && self.files.window.is_focused {
                    self.files.handle_action(action);
                } else if self.wm.has_any_focused() {
                    let k = match action {
                        Action::ArrowUp => "Up",
                        Action::ArrowDown => "Down",
                        Action::ArrowLeft => "Left",
                        Action::ArrowRight => "Right",
                        Action::Home => "Home",
                        Action::End => "End",
                        Action::PageUp => "Prior",
                        Action::PageDown => "Next",
                        _ => "",
                    };
                    if !k.is_empty() {
                        if let Some(id) = self.wm.focused_id {
                            if let Some(win) = self.wm.get_window(id) {
                                if let Some(ref app) = win.external_app {
                                    let _ = self.wm.xdo_tx.send(crate::window_manager::XdoCmd::Key {
                                        display: app.display_num,
                                        key: k.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Action::Backspace | Action::Enter | Action::Delete | Action::Escape => {
                if self.terminal.window.is_open && self.terminal.window.is_focused {
                    self.terminal.handle_action(action);
                } else if self.layer_editor.window.is_open && self.layer_editor.window.is_focused {
                    self.layer_editor.handle_action(action);
                }
            }
            _ => {
                if self.layer_editor.window.is_open && self.layer_editor.window.is_focused {
                    self.layer_editor.handle_action(action);
                }
            }
        }
    }

    pub fn handle_key(&mut self, key_byte: u8) {
        if self.dock.is_all_apps_open {
            if key_byte == 8 {
                self.launcher_search.pop();
            } else if key_byte == 27 {
                self.dock.is_all_apps_open = false;
            } else if key_byte >= 32 && key_byte < 127 {
                self.launcher_search.push(key_byte as char);
            }
        } else if self.wm.has_any_focused() {
            // External window has focus - route all keys to it
            self.wm.handle_key(key_byte as char);
        } else if self.packages.window.is_open && self.packages.window.is_focused {
            self.packages.handle_key(key_byte as char);
        } else if self.files.window.is_open && self.files.window.is_focused {
            self.files.handle_key(key_byte as char);
        } else if self.terminal.window.is_open && self.terminal.window.is_focused {
            self.terminal.handle_char(key_byte as char);
        } else {
            self.wm.handle_key(key_byte as char);
        }
    }

    pub fn draw(&mut self, gfx: &mut Graphics, current_fps: f32) {
        let width = gfx.width();
        let height = gfx.height();
        let mouse_state = MouseState {
            x: self.mouse.x,
            y: self.mouse.y,
            left_pressed: self.mouse.left_pressed,
            right_pressed: self.mouse.right_pressed,
            middle_pressed: self.mouse.middle_pressed,
            left_clicked: false,
            right_clicked: false,
            middle_clicked: false,
            left_released: false,
            right_released: false,
            middle_released: false,
            scroll_delta: 0,
        };

        // 1. Desktop Background (Instant single-pass copy from pre-rendered cache)
        if self.wallpaper_cache.len() != gfx.buffer_size() || self.settings.wallpaper_idx != self.cached_wallpaper_idx {
            self.render_wallpaper_cache(gfx);
        }
        gfx.backbuffer_mut().copy_from_slice(&self.wallpaper_cache);
        gfx.mark_all_dirty();

        // Watermark
        let mark = "L E V I A T H A N";
        gfx.draw_string((width / 2) - 72, (height / 2) - 6, mark, Color::new(40, 38, 50));

        // 2. Top Menu / Status Bar
        let top_rect = Rect::new(0, 0, width as i32, 28);
        top_rect.draw_fill(gfx, Color::new(38, 38, 40));

        // Left logo & system name
        draw_icon(gfx, IconId::AllApps, 12, 7, 14, Color::new(255, 255, 255));
        gfx.draw_string(32, 7, "Leviathan", Color::new(255, 255, 255));

        // Active app indicator
        let active_title = if self.wm.has_any_focused() {
            // Show the title of the focused external window
            self.wm.windows.iter()
                .find(|w| w.focused)
                .map(|w| w.title.as_str())
                .unwrap_or("Leviathan Window")
        } else if self.packages.window.is_focused && self.packages.window.is_open {
            "Packages"
        } else if self.terminal.window.is_focused && self.terminal.window.is_open {
            "Terminal"
        } else if self.files.window.is_focused && self.files.window.is_open {
            "Files"
        } else if self.settings.window.is_focused && self.settings.window.is_open {
            "Settings"
        } else {
            ""
        };
        gfx.draw_string(140, 7, active_title, Color::MACOS_TEXT);

        // RAM usage meter with memory pressure detection
        let mem_col = if self.is_low_mem {
            Color::new(255, 80, 80)
        } else {
            Color::MACOS_TEXT_SECONDARY
        };
        let mem_x = width - 490;
        draw_icon(gfx, IconId::Cpu, (mem_x - 18) as i32, 7, 14, mem_col);
        gfx.draw_string(mem_x, 7, &self.mem_info_cache, mem_col);

        // FPS meter
        let fps_str = format!("{:.0} FPS", current_fps);
        let fps_x = width - 330;
        gfx.draw_string(fps_x, 7, &fps_str, Color::MACOS_TEXT_SECONDARY);

        // Network badge with real online/offline status
        let (net_label, net_col) = if self.is_net_online {
            ("Online", Color::new(50, 215, 75))
        } else {
            ("Offline", Color::new(255, 100, 100))
        };
        let net_x = width - 240;
        draw_icon(gfx, IconId::Wifi, (net_x - 18) as i32, 7, 14, net_col);
        gfx.draw_string(net_x, 7, net_label, net_col);

        // Real-time clock from system
        let mut time_str = "12:00:00".to_string();
        unsafe {
            let t = libc::time(std::ptr::null_mut());
            let tm = libc::localtime(&t);
            if !tm.is_null() {
                time_str = format!("{:02}:{:02}:{:02}", (*tm).tm_hour, (*tm).tm_min, (*tm).tm_sec);
            }
        }
        let clock_x = width - 95;
        draw_icon(gfx, IconId::Clock, (clock_x - 18) as i32, 7, 14, Color::MACOS_TEXT_SECONDARY);
        gfx.draw_string(clock_x, 7, &time_str, Color::MACOS_TEXT);

        // Render active layer elements onto desktop canvas
        if let Some(active_layer) = self.layer_editor.layer_manager.get_active_layer() {
            for element in &active_layer.elements {
                if !element.visibility {
                    continue;
                }
                // Clamp to screen bounds
                let ex = element.x.max(0).min(width as i32 - 1);
                let ey = element.y.max(0).min(height as i32 - 1);
                let ew = (element.width as i32).max(1).min(width as i32 - ex) as i32;
                let eh = (element.height as i32).max(1).min(height as i32 - ey) as i32;

                // Draw image if loaded, otherwise placeholder
                if let Some(ref path) = element.image_path {
                    if let Some(img) = self.layer_editor.image_cache.get(path) {
                        let pitch = gfx.pitch();
                        let gw = gfx.width();
                        let gh = gfx.height();
                        let buf = gfx.backbuffer_mut();
                        img.draw_to_buffer_scaled(buf, gw, gh, pitch, ex, ey, ew as u32, eh as u32, 1.0);
                    } else {
                        gfx.fill_rect(ex, ey, ew, eh, Color::new(80, 80, 100));
                        gfx.draw_string(ex as u32 + 2, ey as u32 + 2, "?", Color::new(200, 200, 200));
                    }
                } else {
                    gfx.fill_rect(ex, ey, ew, eh, Color::new(80, 80, 100));
                }
            }
        }

        // 3. Draw Unfocused Windows first (both built-in and external)
        if self.terminal.window.is_open && !self.terminal.window.is_focused {
            self.terminal.draw(gfx, &mouse_state);
        }
        if self.settings.window.is_open && !self.settings.window.is_focused {
            self.settings.draw(gfx, &mouse_state);
        }
        if self.files.window.is_open && !self.files.window.is_focused {
            self.files.draw(gfx, &mouse_state);
        }
        if self.packages.window.is_open && !self.packages.window.is_focused {
            self.packages.draw(gfx, &mouse_state);
        }
        if self.layer_editor.window.is_open && !self.layer_editor.window.is_focused {
            self.layer_editor.draw(gfx, &mouse_state);
        }

        // Draw unfocused external windows behind focused window
        self.wm.draw_unfocused(gfx, &mouse_state);

        // Draw Focused Window on top
        if self.terminal.window.is_open && self.terminal.window.is_focused {
            self.terminal.draw(gfx, &mouse_state);
        }
        if self.settings.window.is_open && self.settings.window.is_focused {
            self.settings.draw(gfx, &mouse_state);
        }
        if self.files.window.is_open && self.files.window.is_focused {
            self.files.draw(gfx, &mouse_state);
        }
        if self.packages.window.is_open && self.packages.window.is_focused {
            self.packages.draw(gfx, &mouse_state);
        }
        if self.layer_editor.window.is_open && self.layer_editor.window.is_focused {
            self.layer_editor.draw(gfx, &mouse_state);
        }

        // Draw focused external window on top
        self.wm.draw_focused(gfx, &mouse_state);

        // 4. Draw Floating Dock
        self.dock.draw(gfx, &mouse_state);

        // 5. Draw All Apps Fugo Launcher if open
        if self.dock.is_all_apps_open {
            let dw = 580;
            let dh = 380;
            let dx = (width as i32 - dw) / 2;
            let dy = (height as i32 - dh) / 2;

            gfx.draw_squircle_card(dx, dy, dw, dh, 14, Color::MACOS_WINDOW_BG, Color::MACOS_BORDER);

            gfx.draw_string((dx + 24) as u32, (dy + 14) as u32, "Applications", Color::MACOS_TEXT);

            // Search input box
            let search_box = Rect::new(dx + 24, dy + 36, dw - 48, 26);
            gfx.draw_squircle_card(search_box.x, search_box.y, search_box.w, search_box.h, 6, Color::MACOS_TITLEBAR, Color::MACOS_BORDER);
            draw_icon(gfx, IconId::Search, dx + 32, dy + 42, 14, Color::MACOS_TEXT_SECONDARY);
            let stext = if self.launcher_search.is_empty() {
                "Type to search apps..."
            } else {
                &self.launcher_search
            };
            let scol = if self.launcher_search.is_empty() {
                Color::MACOS_TEXT_SECONDARY
            } else {
                Color::MACOS_TEXT
            };
            gfx.draw_string((dx + 52) as u32, (dy + 43) as u32, stext, scol);

            // Category pills
            let cats = ["All", "System", "Internet", "Utilities"];
            for (i, cat) in cats.iter().enumerate() {
                let tab_rect = Rect::new(dx + 24 + (i as i32 * 85), dy + 70, 78, 22);
                let is_sel = self.launcher_category == i;
                let bg = if is_sel { Color::MACOS_ACCENT } else { Color::MACOS_HOVER };
                let border = if is_sel { Color::MACOS_BORDER_ACTIVE } else { Color::MACOS_BORDER };
                gfx.draw_squircle_card(tab_rect.x, tab_rect.y, tab_rect.w, tab_rect.h, 6, bg, border);
                let tx = tab_rect.x + (tab_rect.w - cat.len() as i32 * 8) / 2;
                gfx.draw_string(tx as u32, (tab_rect.y + 5) as u32, cat, if is_sel { Color::new(255, 255, 255) } else { Color::MACOS_TEXT });
            }

            // Filtered apps
            let filtered: Vec<&AppEntry> = self
                .cached_apps
                .iter()
                .filter(|app| {
                    let matches_cat = match self.launcher_category {
                        1 => app.categories.contains("System") || app.categories.contains("Settings") || app.name == "Settings" || app.name == "Files" || app.name == "Terminal",
                        2 => app.categories.contains("Network") || app.categories.contains("WebBrowser") || app.name.to_lowercase().contains("browser") || app.name.to_lowercase().contains("firefox") || app.name.to_lowercase().contains("net"),
                        3 => app.categories.contains("Utility") || app.categories.contains("Development") || app.categories.contains("Graphics") || app.name == "Packages" || app.name == "3D Abyss" || app.name == "htop",
                        _ => true,
                    };
                    let matches_search = self.launcher_search.is_empty()
                        || app.name.to_lowercase().contains(&self.launcher_search.to_lowercase())
                        || app.comment.to_lowercase().contains(&self.launcher_search.to_lowercase());
                    matches_cat && matches_search
                })
                .collect();

            let tile_w = 124;
            let tile_h = 80;
            for (i, app) in filtered.iter().enumerate().take(8) {
                let col = (i % 4) as i32;
                let row = (i / 4) as i32;
                let tx = dx + 24 + col * (tile_w + 14);
                let ty = dy + 104 + row * (tile_h + 14);
                let rect = Rect::new(tx, ty, tile_w, tile_h);

                let hover = rect.contains(mouse_state.x, mouse_state.y);
                let bg = if hover { Color::MACOS_HOVER } else { Color::MACOS_WINDOW_BG };
                let border = if hover { Color::MACOS_BORDER_ACTIVE } else { Color::MACOS_BORDER };

                gfx.draw_squircle_card(rect.x, rect.y, rect.w, rect.h, 8, bg, border);

                let icon_col = if hover { Color::MACOS_ACCENT } else { Color::new(230, 235, 245) };
                draw_icon(gfx, app.icon_id, rect.x + (rect.w - 28) / 2, rect.y + 12, 28, icon_col);
                let name_tx = rect.x + (rect.w - app.name.len() as i32 * 8) / 2;
                gfx.draw_string(name_tx.max(rect.x + 6) as u32, (rect.y + 48) as u32, &app.name, Color::MACOS_TEXT);
            }
        }

        // 6. Draw Mouse Cursor on top of everything
        self.mouse.draw_cursor(gfx);
    }
}
