use crate::color::Color;
use crate::font::get_glyph;
use crate::graphics::Graphics;
use crate::icons::{draw_icon, IconId};
use crate::mouse::MouseState;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Instant;

pub const TITLEBAR_HEIGHT: i32 = 28;
pub const BORDER_COLOR_FOCUSED: Color = Color::new(70, 130, 240);
pub const BORDER_COLOR_UNFOCUSED: Color = Color::new(55, 60, 72);
pub const TITLEBAR_COLOR_FOCUSED: Color = Color::new(45, 52, 68);
pub const TITLEBAR_COLOR_UNFOCUSED: Color = Color::new(32, 35, 45);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Maximized,
    Minimized,
    Closing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeMode {
    None,
    Right,
    Bottom,
    BottomRight,
}

pub enum XdoCmd {
    MouseMove { display: u32, x: i32, y: i32 },
    MouseDown { display: u32, button: u8 },
    MouseUp { display: u32, button: u8 },
    Click { display: u32, button: u8 },
    Key { display: u32, key: String },
    Resize { display: u32, width: u32, height: u32 },
}

pub struct ExternalAppHandle {
    pub display_num: u32,
    /// True when the app was launched as a native Wayland/compositor client (no Xvfb).
    pub is_native_wayland: bool,
    pub fb_dir: PathBuf,
    pub fb_file: PathBuf,
    pub child_proc: Option<Child>,
    pub xvfb_proc: Option<Child>,
    pub xvfb_w: u32,
    pub xvfb_h: u32,
    pub log_path: PathBuf,
    pub last_read: Instant,
    pub last_liveness_check: Instant,
    pub prev_mouse_x: i32,
    pub prev_mouse_y: i32,
    pub fb_file_handle: Option<File>,
    pub mmap_ptr: *mut u8,
    pub mmap_len: usize,
}

impl ExternalAppHandle {
    pub fn cleanup(&mut self) {
        if !self.mmap_ptr.is_null() && self.mmap_ptr != libc::MAP_FAILED as *mut u8 {
            unsafe {
                libc::munmap(self.mmap_ptr as *mut libc::c_void, self.mmap_len);
            }
            self.mmap_ptr = std::ptr::null_mut();
            self.mmap_len = 0;
        }
        self.fb_file_handle = None;
        if let Some(mut child) = self.child_proc.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(mut xvfb) = self.xvfb_proc.take() {
            let _ = xvfb.kill();
            let _ = xvfb.wait();
        }
        if !self.is_native_wayland {
            let _ = fs::remove_file(format!("/tmp/.X11-unix/X{}", self.display_num));
            let _ = fs::remove_dir_all(&self.fb_dir);
        }
    }
}

impl Drop for ExternalAppHandle {
    fn drop(&mut self) {
        self.cleanup();
    }
}

pub struct Window {
    pub id: u32,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub state: WindowState,
    pub normal_x: i32,
    pub normal_y: i32,
    pub normal_width: u32,
    pub normal_height: u32,
    pub surface: Vec<u8>, // BGRX pixel buffer (width * height * 4 bytes)
    pub mapped: bool,
    pub focused: bool,
    pub is_dragging: bool,
    pub drag_offset_x: i32,
    pub drag_offset_y: i32,
    pub resize_mode: ResizeMode,
    pub resize_start_mouse_x: i32,
    pub resize_start_mouse_y: i32,
    pub resize_start_w: u32,
    pub resize_start_h: u32,
    pub client_has_mouse_grab: bool,
    pub is_closed: bool,
    pub external_app: Option<ExternalAppHandle>,
}

impl Window {
    pub fn new(id: u32, title: &str, x: i32, y: i32, width: u32, height: u32) -> Self {
        let buf_size = (width * height * 4) as usize;
        Self {
            id,
            title: title.to_string(),
            x,
            y,
            width,
            height,
            state: WindowState::Normal,
            normal_x: x,
            normal_y: y,
            normal_width: width,
            normal_height: height,
            surface: vec![0u8; buf_size],
            mapped: true,
            focused: true,
            is_dragging: false,
            drag_offset_x: 0,
            drag_offset_y: 0,
            resize_mode: ResizeMode::None,
            resize_start_mouse_x: 0,
            resize_start_mouse_y: 0,
            resize_start_w: width,
            resize_start_h: height,
            client_has_mouse_grab: false,
            is_closed: false,
            external_app: None,
        }
    }

    pub fn client_rect(&self) -> (i32, i32, i32, i32) {
        (self.x + 1, self.y + TITLEBAR_HEIGHT, self.width as i32, self.height as i32)
    }

    pub fn full_rect(&self) -> (i32, i32, i32, i32) {
        (
            self.x,
            self.y,
            self.width as i32 + 2,
            self.height as i32 + TITLEBAR_HEIGHT + 1,
        )
    }

    pub fn close_button_rect(&self) -> (i32, i32, i32, i32) {
        (self.x + self.width as i32 + 2 - 24, self.y + 4, 20, 20)
    }

    pub fn maximize_button_rect(&self) -> (i32, i32, i32, i32) {
        (self.x + self.width as i32 + 2 - 48, self.y + 4, 20, 20)
    }

    pub fn minimize_button_rect(&self) -> (i32, i32, i32, i32) {
        (self.x + self.width as i32 + 2 - 72, self.y + 4, 20, 20)
    }

    pub fn fill_surface(&mut self, b: u8, g: u8, r: u8) {
        for chunk in self.surface.chunks_exact_mut(4) {
            chunk[0] = b;
            chunk[1] = g;
            chunk[2] = r;
            chunk[3] = 255;
        }
    }

    pub fn draw_text_on_surface(&mut self, x: i32, y: i32, text: &str, b: u8, g: u8, r: u8) {
        let sw = self.width as i32;
        let sh = self.height as i32;
        for (i, ch) in text.chars().enumerate() {
            let cx = x + (i as i32) * 8;
            if cx + 8 <= 0 || cx >= sw {
                continue;
            }
            let glyph = get_glyph(ch);
            for row in 0..8 {
                let py = y + row as i32;
                if py < 0 || py >= sh {
                    continue;
                }
                let byte = glyph[row];
                for col in 0..8 {
                    if (byte & (0x80 >> col)) != 0 {
                        let px = cx + col as i32;
                        if px >= 0 && px < sw {
                            let offset = ((py as usize) * (self.width as usize) + (px as usize)) * 4;
                            if offset + 4 <= self.surface.len() {
                                self.surface[offset] = b;
                                self.surface[offset + 1] = g;
                                self.surface[offset + 2] = r;
                                self.surface[offset + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn refresh_external_pixels(&mut self) {
        if let Some(ref mut app) = self.external_app {
            // Lazy mmap initialisation on first frame
            if app.mmap_ptr.is_null() && app.fb_file.exists() {
                if let Ok(c_path) = std::ffi::CString::new(app.fb_file.to_string_lossy().as_bytes()) {
                    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY) };
                    if fd >= 0 {
                        let mut st: libc::stat = unsafe { std::mem::zeroed() };
                        if unsafe { libc::fstat(fd, &mut st) } == 0 && st.st_size > 3232 {
                            let len = st.st_size as usize;
                            let ptr = unsafe {
                                libc::mmap(
                                    std::ptr::null_mut(),
                                    len,
                                    libc::PROT_READ,
                                    libc::MAP_SHARED,
                                    fd,
                                    0,
                                )
                            };
                            if ptr != libc::MAP_FAILED {
                                app.mmap_ptr = ptr as *mut u8;
                                app.mmap_len = len;
                            }
                        }
                        unsafe { libc::close(fd); }
                    }
                }
            }

            if !app.mmap_ptr.is_null() {
                let xvfb_w = app.xvfb_w as usize;
                let copy_w = (self.width as usize).min(xvfb_w);
                let copy_h = self.height as usize;
                let copy_row_bytes = copy_w * 4;
                let xvfb_stride = xvfb_w * 4;

                let min_file_bytes = 3232 + (copy_h * xvfb_stride);
                if app.mmap_len >= min_file_bytes {
                    let src_slice = unsafe { std::slice::from_raw_parts(app.mmap_ptr, app.mmap_len) };
                    for y in 0..copy_h {
                        let src_offset = 3232 + y * xvfb_stride;
                        let dst_offset = y * (self.width as usize) * 4;
                        if src_offset + copy_row_bytes <= app.mmap_len
                            && dst_offset + copy_row_bytes <= self.surface.len()
                        {
                            self.surface[dst_offset..dst_offset + copy_row_bytes]
                                .copy_from_slice(&src_slice[src_offset..src_offset + copy_row_bytes]);
                        }
                    }
                }
            }
        }
    }

    pub fn check_external_liveness(&mut self) {
        if let Some(ref mut app) = self.external_app {
            if app.last_liveness_check.elapsed().as_millis() < 1000 {
                return;
            }
            app.last_liveness_check = Instant::now();

            if let Some(ref mut child) = app.child_proc {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        println!(
                            "WindowManager: Child process for window '{}' exited with status: {}",
                            self.title, status
                        );
                        app.child_proc = None;
                        if !self.title.contains("[Exited]") {
                            self.title = format!("{} [Exited: {}]", self.title, status);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!(
                            "WindowManager: Failed to wait on child for window '{}': {}",
                            self.title, e
                        );
                        app.child_proc = None;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppLaunchConfig {
    pub cmd: String,
    pub args: Vec<String>,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub use_sandbox: bool,
    pub run_as_user: bool,
    pub working_dir: Option<String>,
}

impl Default for AppLaunchConfig {
    fn default() -> Self {
        Self {
            cmd: String::new(),
            args: Vec::new(),
            title: String::new(),
            width: 720,
            height: 420,
            use_sandbox: false,
            run_as_user: true,
            working_dir: Some("/home/leviathan".to_string()),
        }
    }
}

pub struct WindowManager {
    pub windows: Vec<Window>,
    pub focused_id: Option<u32>,
    pub next_id: u32,
    pub next_display: u32,
    pub xdo_tx: Sender<XdoCmd>,
}

impl WindowManager {
    pub fn new() -> Self {
        let _ = fs::create_dir_all("/tmp/.X11-unix");

        let (xdo_tx, xdo_rx) = channel::<XdoCmd>();

        // Background worker executing xdotool commands with clean DISPLAY environment
        thread::spawn(move || {
            while let Ok(cmd) = xdo_rx.recv() {
                match cmd {
                    XdoCmd::MouseMove { display, x, y } => {
                        let disp = format!(":{}", display);
                        let _ = Command::new("/usr/bin/xdotool")
                            .args(["mousemove", &x.to_string(), &y.to_string()])
                            .env("DISPLAY", &disp)
                            .env("LD_LIBRARY_PATH", "/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                    XdoCmd::MouseDown { display, button } => {
                        let disp = format!(":{}", display);
                        let _ = Command::new("/usr/bin/xdotool")
                            .args(["mousedown", &button.to_string()])
                            .env("DISPLAY", &disp)
                            .env("LD_LIBRARY_PATH", "/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                    XdoCmd::MouseUp { display, button } => {
                        let disp = format!(":{}", display);
                        let _ = Command::new("/usr/bin/xdotool")
                            .args(["mouseup", &button.to_string()])
                            .env("DISPLAY", &disp)
                            .env("LD_LIBRARY_PATH", "/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                    XdoCmd::Click { display, button } => {
                        let disp = format!(":{}", display);
                        let _ = Command::new("/usr/bin/xdotool")
                            .args(["click", &button.to_string()])
                            .env("DISPLAY", &disp)
                            .env("LD_LIBRARY_PATH", "/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                    XdoCmd::Key { display, key } => {
                        let disp = format!(":{}", display);
                        let _ = Command::new("/usr/bin/xdotool")
                            .args(["key", &key])
                            .env("DISPLAY", &disp)
                            .env("LD_LIBRARY_PATH", "/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                    XdoCmd::Resize { display, width, height } => {
                        let disp = format!(":{}", display);
                        // Resize all managed application windows to the new dimensions
                        let _ = Command::new("/usr/bin/xdotool")
                            .args([
                                "search",
                                "--name",
                                ".*",
                                "windowsize",
                                "%@",
                                &width.to_string(),
                                &height.to_string(),
                                "windowmove",
                                "%@",
                                "0",
                                "0",
                            ])
                            .env("DISPLAY", &disp)
                            .env("LD_LIBRARY_PATH", "/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                }
            }
        });

        let mut wm = Self {
            windows: Vec::new(),
            focused_id: None,
            next_id: 1,
            next_display: 10,
            xdo_tx,
        };

        // Create initial minimal test window on boot
        wm.create_test_window();

        wm
    }

    pub fn create_test_window(&mut self) -> u32 {
        let id = self.create_window("Leviathan Test Window", 160, 100, 420, 220);
        if let Some(win) = self.get_window_mut(id) {
            win.fill_surface(45, 30, 22);

            let msg = "WINDOW MANAGER TEST";
            let msg_x = (420 - (msg.len() as i32 * 8)) / 2;
            win.draw_text_on_surface(msg_x, 90, msg, 80, 220, 255);

            let sub = "[X] Close  |  Click & Drag Titlebar";
            let sub_x = (420 - (sub.len() as i32 * 8)) / 2;
            win.draw_text_on_surface(sub_x, 120, sub, 200, 200, 210);
        }
        id
    }

    pub fn get_window_mut(&mut self, id: u32) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.id == id)
    }

    pub fn get_window(&self, id: u32) -> Option<&Window> {
        self.windows.iter().find(|w| w.id == id)
    }

    pub fn create_window(&mut self, title: &str, x: i32, y: i32, width: u32, height: u32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        for w in &mut self.windows {
            w.focused = false;
        }

        let win = Window::new(id, title, x, y, width, height);
        self.windows.push(win);
        self.focused_id = Some(id);
        id
    }

    pub fn destroy_window(&mut self, id: u32) {
        if let Some(pos) = self.windows.iter().position(|w| w.id == id) {
            let mut win = self.windows.remove(pos);
            win.state = WindowState::Closing;
            if let Some(mut app) = win.external_app.take() {
                app.cleanup();
            }
        }
        if self.focused_id == Some(id) {
            self.focused_id = self.windows.iter().rev().find(|w| w.mapped).map(|w| w.id);
            if let Some(new_id) = self.focused_id {
                self.focus_window(new_id);
            }
        }
    }

    pub fn map_window(&mut self, id: u32) {
        if let Some(win) = self.get_window_mut(id) {
            win.mapped = true;
            win.state = WindowState::Normal;
        }
    }

    pub fn unmap_window(&mut self, id: u32) {
        if let Some(win) = self.get_window_mut(id) {
            win.mapped = false;
            win.focused = false;
            win.state = WindowState::Minimized;
        }
        if self.focused_id == Some(id) {
            self.focused_id = self.windows.iter().rev().find(|w| w.mapped).map(|w| w.id);
            if let Some(new_id) = self.focused_id {
                self.focus_window(new_id);
            }
        }
    }

    pub fn move_window(&mut self, id: u32, x: i32, y: i32) {
        if let Some(win) = self.get_window_mut(id) {
            win.x = x;
            win.y = y;
        }
    }

    pub fn resize_window(&mut self, id: u32, width: u32, height: u32) {
        let mut display_opt = None;
        if let Some(win) = self.get_window_mut(id) {
            win.width = width;
            win.height = height;
            win.surface.resize((width * height * 4) as usize, 0);

            if let Some(ref app) = win.external_app {
                display_opt = Some(app.display_num);
            }
        }
        if let Some(disp) = display_opt {
            let _ = self.xdo_tx.send(XdoCmd::Resize {
                display: disp,
                width,
                height,
            });
        }
    }

    pub fn toggle_maximize(&mut self, id: u32, screen_w: i32, screen_h: i32) {
        let mut resize_cmd = None;
        if let Some(win) = self.get_window_mut(id) {
            match win.state {
                WindowState::Maximized => {
                    // Restore to normal geometry
                    win.x = win.normal_x;
                    win.y = win.normal_y;
                    let w = win.normal_width;
                    let h = win.normal_height;
                    win.width = w;
                    win.height = h;
                    win.surface.resize((w * h * 4) as usize, 0);
                    win.state = WindowState::Normal;

                    if let Some(ref app) = win.external_app {
                        resize_cmd = Some((app.display_num, w, h));
                    }
                }
                _ => {
                    // Save current normal geometry before maximizing
                    win.normal_x = win.x;
                    win.normal_y = win.y;
                    win.normal_width = win.width;
                    win.normal_height = win.height;

                    // Available desktop display: accounting for top bar (28px) and floating dock (60px)
                    let max_x = 0;
                    let max_y = 28;
                    let max_w = (screen_w - 2).max(200) as u32;
                    let max_h = (screen_h - 28 - 60 - 2).max(150) as u32;

                    win.x = max_x;
                    win.y = max_y;
                    win.width = max_w;
                    win.height = max_h;
                    win.surface.resize((max_w * max_h * 4) as usize, 0);
                    win.state = WindowState::Maximized;

                    if let Some(ref app) = win.external_app {
                        resize_cmd = Some((app.display_num, max_w, max_h));
                    }
                }
            }
        }
        if let Some((disp, w, h)) = resize_cmd {
            let _ = self.xdo_tx.send(XdoCmd::Resize {
                display: disp,
                width: w,
                height: h,
            });
        }
    }

    pub fn focus_window(&mut self, id: u32) {
        for w in &mut self.windows {
            w.focused = w.id == id;
        }
        self.focused_id = Some(id);
    }

    pub fn raise_to_front(&mut self, id: u32) {
        if let Some(pos) = self.windows.iter().position(|w| w.id == id) {
            let win = self.windows.remove(pos);
            self.windows.push(win);
        }
    }

    pub fn has_any_focused(&self) -> bool {
        self.windows.iter().any(|w| w.mapped && w.focused && !w.is_closed)
    }

    pub fn has_focused_window_at(&self, px: i32, py: i32) -> bool {
        self.windows.iter().any(|w| {
            if !w.mapped || !w.focused || w.is_closed {
                return false;
            }
            let (x, y, w_box, h_box) = w.full_rect();
            px >= x && px < x + w_box && py >= y && py < y + h_box
        })
    }

    pub fn try_focus_at(&mut self, px: i32, py: i32) -> bool {
        let mut hit_id = None;
        for win in self.windows.iter().rev() {
            if !win.mapped || win.is_closed {
                continue;
            }
            let (x, y, w_box, h_box) = win.full_rect();
            if px >= x && px < x + w_box && py >= y && py < y + h_box {
                hit_id = Some(win.id);
                break;
            }
        }

        if let Some(id) = hit_id {
            self.focus_window(id);
            self.raise_to_front(id);
            true
        } else {
            for w in &mut self.windows {
                w.focused = false;
            }
            self.focused_id = None;
            false
        }
    }

    pub fn cycle_focus(&mut self) {
        let mapped_ids: Vec<u32> = self.windows.iter().filter(|w| w.mapped && !w.is_closed).map(|w| w.id).collect();
        if mapped_ids.len() <= 1 {
            return;
        }
        let cur_pos = self.focused_id.and_then(|id| mapped_ids.iter().position(|&x| x == id)).unwrap_or(0);
        let next_idx = (cur_pos + 1) % mapped_ids.len();
        let next_id = mapped_ids[next_idx];
        self.focus_window(next_id);
        self.raise_to_front(next_id);
    }

    pub fn close_focused(&mut self) -> bool {
        if let Some(id) = self.focused_id {
            self.destroy_window(id);
            true
        } else {
            false
        }
    }

    pub fn minimize_focused(&mut self) -> bool {
        if let Some(id) = self.focused_id {
            self.unmap_window(id);
            true
        } else {
            false
        }
    }

    pub fn maximize_focused(&mut self, screen_w: i32, screen_h: i32) -> bool {
        if let Some(id) = self.focused_id {
            if let Some(win) = self.get_window(id) {
                if win.state != WindowState::Maximized {
                    self.toggle_maximize(id, screen_w, screen_h);
                    return true;
                }
            }
        }
        false
    }

    pub fn restore_or_minimize_focused(&mut self, screen_w: i32, screen_h: i32) -> bool {
        if let Some(id) = self.focused_id {
            if let Some(win) = self.get_window(id) {
                if win.state == WindowState::Maximized {
                    self.toggle_maximize(id, screen_w, screen_h);
                } else {
                    self.unmap_window(id);
                }
                return true;
            }
        }
        false
    }

    pub fn is_mouse_over_external_client(&self, px: i32, py: i32) -> bool {
        for win in self.windows.iter().rev() {
            if !win.mapped || win.is_closed {
                continue;
            }
            if win.external_app.is_none() {
                continue;
            }
            let (cl_x, cl_y, cl_w, cl_h) = win.client_rect();
            if px >= cl_x && px < cl_x + cl_w && py >= cl_y && py < cl_y + cl_h {
                return true;
            }
        }
        false
    }


    pub fn spawn_app(
        &mut self,
        cmd: &str,
        args: &[&str],
        title: &str,
        w: u32,
        h: u32,
        screen_w: i32,
        screen_h: i32,
    ) -> u32 {
        let config = AppLaunchConfig {
            cmd: cmd.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            title: title.to_string(),
            width: w,
            height: h,
            use_sandbox: false,
            run_as_user: true,
            working_dir: Some("/home/leviathan".to_string()),
        };
        self.spawn_app_with_config(&config, screen_w, screen_h)
    }

    pub fn spawn_app_with_config(
        &mut self,
        config: &AppLaunchConfig,
        screen_w: i32,
        screen_h: i32,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let w = config.width;
        let h = config.height;
        let win_x = ((screen_w - (w as i32 + 2)) / 2).max(20) + ((id as i32 % 5) * 24);
        let win_y = ((screen_h - (h as i32 + TITLEBAR_HEIGHT + 1)) / 2).max(36) + ((id as i32 % 5) * 24);

        // Resolve the executable from PATH; no application-specific hardcoding.
        let resolved_cmd = if PathBuf::from(&config.cmd).is_absolute() && PathBuf::from(&config.cmd).exists() {
            config.cmd.clone()
        } else {
            let candidate_dirs = [
                "/usr/bin", "/bin", "/usr/local/bin",
                "/usr/games", "/sbin", "/usr/sbin",
                "/opt/hypr/bin",
            ];
            let mut found = config.cmd.clone();
            for dir in candidate_dirs {
                let candidate = Path::new(dir).join(&config.cmd);
                if candidate.exists() {
                    found = candidate.to_string_lossy().to_string();
                    break;
                }
            }
            found
        };

        // Determine effective XDG_RUNTIME_DIR: inherit from session or compute from uid.
        let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
            format!("/run/user/{}", unsafe { libc::getuid() })
        });

        // Detect whether a live Wayland compositor is available.
        let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
        let is_wayland = !wayland_display.is_empty();

        let title = config.title.clone();
        let safe_title = title.replace(|c: char| !c.is_alphanumeric() && c != '_', "_").to_lowercase();
        let log_path = PathBuf::from(format!("/tmp/app_{}.log", safe_title));

        if is_wayland {
            // ── Native Wayland path ───────────────────────────────────────────────────
            // The compositor is Hyprland; launch the app directly so it appears as a
            // first-class Wayland (or XWayland) client.  No Xvfb, no framebuffer copy.
            println!(
                "WindowManager: Launching '{}' natively on Wayland compositor ({})",
                resolved_cmd, wayland_display,
            );

            let user_home = "/home/leviathan";
            let _ = fs::create_dir_all(user_home);

            let mut cmd_builder = Command::new(&resolved_cmd);
            cmd_builder.args(&config.args);

            // Preserve the full session environment so Wayland, D-Bus, and
            // XDG services all remain reachable.
            let cwd = config.working_dir.as_deref().unwrap_or(user_home);
            if PathBuf::from(cwd).is_dir() {
                cmd_builder.current_dir(cwd);
            }
            cmd_builder.env("HOME", user_home);
            cmd_builder.env("USER", "leviathan");
            cmd_builder.env("LOGNAME", "leviathan");
            cmd_builder.env("SHELL", "/bin/bash");
            cmd_builder.env("XDG_RUNTIME_DIR", &xdg_runtime_dir);
            cmd_builder.env("XDG_CONFIG_HOME", "/home/leviathan/.config");
            cmd_builder.env("XDG_DATA_HOME", "/home/leviathan/.local/share");
            cmd_builder.env("XDG_CACHE_HOME", "/home/leviathan/.cache");
            cmd_builder.env("PATH", "/opt/hypr/bin:/usr/local/bin:/usr/bin:/bin:/usr/games:/sbin:/usr/sbin");
            cmd_builder.env(
                "LD_LIBRARY_PATH",
                "/opt/hypr/lib:/usr/lib/firefox:/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu",
            );
            // WAYLAND_DISPLAY, DBUS_SESSION_BUS_ADDRESS, XDG_CURRENT_DESKTOP, etc.
            // are inherited from the parent — intentionally not overridden here.

            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                if unsafe { libc::geteuid() == 0 } && config.run_as_user {
                    cmd_builder.uid(1000);
                    cmd_builder.gid(1000);
                }
            }

            if let Ok(log_f) = OpenOptions::new().create(true).write(true).truncate(true).open(&log_path) {
                if let Ok(err_f) = log_f.try_clone() {
                    cmd_builder.stdout(Stdio::from(log_f));
                    cmd_builder.stderr(Stdio::from(err_f));
                }
            }

            let child_proc = match cmd_builder.spawn() {
                Ok(cp) => {
                    println!("WindowManager: '{}' launched (pid {})", resolved_cmd, cp.id());
                    Some(cp)
                }
                Err(e) => {
                    eprintln!("WindowManager: Failed to launch '{}': {}", resolved_cmd, e);
                    None
                }
            };

            let mut win = Window::new(id, &title, win_x, win_y, w, h);
            win.fill_surface(25, 30, 35);
            if child_proc.is_some() {
                win.draw_text_on_surface(20, 30, &format!("Launched: {}", title), 140, 200, 180);
                win.draw_text_on_surface(20, 60, "Running in compositor session.", 130, 155, 170);
                win.draw_text_on_surface(20, 90, &format!("Log: {}", log_path.display()), 100, 115, 130);
            } else {
                win.title = format!("{} [Failed to Launch]", title);
                win.draw_text_on_surface(20, 30, "Application Failed to Start", 255, 80, 80);
                let err_msg = format!("'{}' not found or could not execute.", config.cmd);
                win.draw_text_on_surface(20, 60, &err_msg, 200, 200, 210);
                win.draw_text_on_surface(20, 90, "Check Packages app to install it.", 160, 160, 180);
            }

            win.external_app = Some(ExternalAppHandle {
                display_num: 0,
                is_native_wayland: true,
                fb_dir: PathBuf::new(),
                fb_file: PathBuf::new(),
                child_proc,
                xvfb_proc: None,
                xvfb_w: w,
                xvfb_h: h,
                log_path,
                last_read: Instant::now(),
                last_liveness_check: Instant::now(),
                prev_mouse_x: -1,
                prev_mouse_y: -1,
                fb_file_handle: None,
                mmap_ptr: std::ptr::null_mut(),
                mmap_len: 0,
            });

            for w_item in &mut self.windows {
                w_item.focused = false;
            }
            self.windows.push(win);
            self.focused_id = Some(id);
            return id;
        }

        // ── Legacy Xvfb path (standalone /dev/fb0 mode, no compositor) ────────────
        let display_num = self.next_display;
        self.next_display += 1;

        let fb_dir = PathBuf::from(format!("/tmp/xvfb_d{}", display_num));
        let _ = fs::create_dir_all(&fb_dir);
        let fb_file = fb_dir.join("Xvfb_screen0");

        let xvfb_w = (screen_w as u32).max(1280);
        let xvfb_h = (screen_h as u32).max(800);

        println!(
            "WindowManager: Spawning '{}' on display :{} (canvas: {}x{}, win: {}x{}) -> log: {:?}",
            config.cmd, display_num, xvfb_w, xvfb_h, w, h, log_path
        );

        let xvfb_bin = if PathBuf::from("/usr/bin/Xvfb").exists() { "/usr/bin/Xvfb" } else { "/bin/Xvfb" };

        let xvfb_proc = Command::new(xvfb_bin)
            .args([
                &format!(":{}", display_num),
                "-screen", "0",
                &format!("{}x{}x24", xvfb_w, xvfb_h),
                "-fbdir", &fb_dir.to_string_lossy(),
                "-ac", "-noreset", "-nolisten", "tcp",
            ])
            .env("LD_LIBRARY_PATH", "/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();

        // Wait up to 3 s for the X11 socket
        let socket_path = format!("/tmp/.X11-unix/X{}", display_num);
        let start = Instant::now();
        while start.elapsed().as_millis() < 3000 {
            if PathBuf::from(&socket_path).exists() {
                thread::sleep(std::time::Duration::from_millis(60));
                break;
            }
            thread::sleep(std::time::Duration::from_millis(30));
        }

        let mut cmd_builder = Command::new(&resolved_cmd);
        let final_args = config.args.clone();
        cmd_builder.args(&final_args);
        cmd_builder.env("DISPLAY", format!(":{}", display_num));

        if config.use_sandbox {
            let sandbox_dir = format!("/tmp/sandbox_d{}", display_num);
            let _ = fs::create_dir_all(&sandbox_dir);
            cmd_builder.current_dir(&sandbox_dir);
            cmd_builder.env("HOME", &sandbox_dir);
            cmd_builder.env("USER", "sandbox");
            cmd_builder.env("LOGNAME", "sandbox");
            cmd_builder.env("XDG_RUNTIME_DIR", &xdg_runtime_dir);
        } else {
            let user_home = "/home/leviathan";
            let _ = fs::create_dir_all(user_home);
            for subdir in &[
                "/home/leviathan/Downloads", "/home/leviathan/Desktop",
                "/home/leviathan/Documents", "/home/leviathan/.config",
                "/home/leviathan/.local/share", "/home/leviathan/.cache",
                "/home/leviathan/.mozilla/firefox",
            ] {
                let _ = fs::create_dir_all(subdir);
            }

            #[cfg(unix)]
            {
                for p in &[
                    "/home/leviathan/Downloads", "/home/leviathan/Desktop",
                    "/home/leviathan/Documents", "/home/leviathan/.config",
                    "/home/leviathan/.local", "/home/leviathan/.local/share",
                    "/home/leviathan/.cache", "/home/leviathan/.mozilla",
                    "/home/leviathan/.mozilla/firefox",
                ] {
                    if let Ok(cs) = std::ffi::CString::new(*p) {
                        unsafe { libc::chown(cs.as_ptr(), 1000, 1000); }
                    }
                }
            }

            let cwd = config.working_dir.as_deref().unwrap_or(user_home);
            cmd_builder.current_dir(cwd);
            cmd_builder.env("HOME", user_home);
            cmd_builder.env("USER", "leviathan");
            cmd_builder.env("LOGNAME", "leviathan");
            cmd_builder.env("SHELL", "/bin/bash");
            cmd_builder.env("XDG_RUNTIME_DIR", &xdg_runtime_dir);
            cmd_builder.env("XDG_CONFIG_HOME", "/home/leviathan/.config");
            cmd_builder.env("XDG_DATA_HOME", "/home/leviathan/.local/share");
            cmd_builder.env("XDG_CACHE_HOME", "/home/leviathan/.cache");
            cmd_builder.env("XDG_DOWNLOAD_DIR", "/home/leviathan/Downloads");
            cmd_builder.env("XDG_DESKTOP_DIR", "/home/leviathan/Desktop");
            cmd_builder.env("XDG_DOCUMENTS_DIR", "/home/leviathan/Documents");

            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                if unsafe { libc::geteuid() == 0 } && config.run_as_user {
                    cmd_builder.uid(1000);
                    cmd_builder.gid(1000);
                }
            }
        }

        // Force Xvfb X11 display: prevent Wayland-capable apps from bypassing it.
        cmd_builder.env_remove("WAYLAND_DISPLAY");
        cmd_builder.env_remove("WAYLAND_SOCKET");
        cmd_builder.env("QT_QPA_PLATFORM", "xcb");
        cmd_builder.env("GDK_BACKEND", "x11");
        cmd_builder.env("SDL_VIDEODRIVER", "x11");
        cmd_builder.env("MOZ_ENABLE_WAYLAND", "0");
        cmd_builder.env("MOZ_DISABLE_CONTENT_SANDBOX", "1");
        cmd_builder.env("MOZ_DISABLE_GMP_SANDBOX", "1");
        cmd_builder.env("MOZ_DISABLE_RDD_SANDBOX", "1");
        cmd_builder.env("MOZ_DISABLE_SOCKET_PROCESS_SANDBOX", "1");
        cmd_builder.env("LIBGL_ALWAYS_SOFTWARE", "1");
        cmd_builder.env("NO_AT_BRIDGE", "1");
        cmd_builder.env(
            "LD_LIBRARY_PATH",
            "/usr/lib/firefox:/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu",
        );
        cmd_builder.env("PATH", "/usr/local/bin:/usr/bin:/bin:/usr/games:/sbin:/usr/sbin");

        if let Ok(log_f) = OpenOptions::new().create(true).write(true).truncate(true).open(&log_path) {
            if let Ok(err_f) = log_f.try_clone() {
                cmd_builder.stdout(Stdio::from(log_f));
                cmd_builder.stderr(Stdio::from(err_f));
            }
        }

        let child_proc = match cmd_builder.spawn() {
            Ok(cp) => Some(cp),
            Err(e) => {
                eprintln!("WindowManager: Failed to spawn '{}': {}", resolved_cmd, e);
                None
            }
        };

        let mut win = Window::new(id, &title, win_x, win_y, w, h);
        win.fill_surface(35, 30, 25);
        if child_proc.is_none() {
            win.draw_text_on_surface(20, 30, "Application Failed to Start", 255, 80, 80);
            let err_msg = format!("Command '{}' not found or failed to execute.", config.cmd);
            win.draw_text_on_surface(20, 60, &err_msg, 200, 200, 210);
            win.draw_text_on_surface(20, 90, "You can install packages using the Packages app.", 160, 160, 180);
            win.title = format!("{} [Failed to Launch]", config.title);
        } else {
            win.draw_text_on_surface(20, 30, &format!("Launching {}...", title), 180, 200, 220);
        }

        win.external_app = Some(ExternalAppHandle {
            display_num,
            is_native_wayland: false,
            fb_dir,
            fb_file,
            child_proc,
            xvfb_proc,
            xvfb_w,
            xvfb_h,
            log_path,
            last_read: Instant::now(),
            last_liveness_check: Instant::now(),
            prev_mouse_x: -1,
            prev_mouse_y: -1,
            fb_file_handle: None,
            mmap_ptr: std::ptr::null_mut(),
            mmap_len: 0,
        });

        for w_item in &mut self.windows {
            w_item.focused = false;
        }
        self.windows.push(win);
        self.focused_id = Some(id);
        id
    }


    pub fn update(&mut self, mouse: &MouseState, screen_w: i32, screen_h: i32) {
        // 1. Interactive window moving and resizing
        for win in &mut self.windows {
            if win.is_dragging && mouse.left_pressed {
                let new_x = mouse.x - win.drag_offset_x;
                let new_y = mouse.y - win.drag_offset_y;
                win.x = new_x.clamp(-(win.width as i32) + 60, screen_w - 60);
                win.y = new_y.clamp(28, screen_h - 40);
            }
            if win.resize_mode != ResizeMode::None && mouse.left_pressed {
                let dx = mouse.x - win.resize_start_mouse_x;
                let dy = mouse.y - win.resize_start_mouse_y;
                let mut new_w = win.resize_start_w;
                let mut new_h = win.resize_start_h;

                match win.resize_mode {
                    ResizeMode::Right => {
                        new_w = (win.resize_start_w as i32 + dx).clamp(200, screen_w - win.x - 4) as u32;
                    }
                    ResizeMode::Bottom => {
                        new_h = (win.resize_start_h as i32 + dy).clamp(150, screen_h - win.y - 32) as u32;
                    }
                    ResizeMode::BottomRight => {
                        new_w = (win.resize_start_w as i32 + dx).clamp(200, screen_w - win.x - 4) as u32;
                        new_h = (win.resize_start_h as i32 + dy).clamp(150, screen_h - win.y - 32) as u32;
                    }
                    ResizeMode::None => {}
                }

                if new_w != win.width || new_h != win.height {
                    win.width = new_w;
                    win.height = new_h;
                    win.surface.resize((new_w * new_h * 4) as usize, 0);
                    if let Some(ref app) = win.external_app {
                        let _ = self.xdo_tx.send(XdoCmd::Resize {
                            display: app.display_num,
                            width: new_w,
                            height: new_h,
                        });
                    }
                }
            }

            if mouse.left_released {
                win.is_dragging = false;
                win.resize_mode = ResizeMode::None;
            }
        }

        // 2. Mouse click hit testing
        if mouse.left_clicked {
            let mut clicked_id = None;
            let mut action = 0; // 0=none, 1=close, 2=maximize, 3=minimize, 4=drag_title, 5=resize_br, 6=resize_r, 7=resize_b, 8=client
            let mut drag_ox = 0;
            let mut drag_oy = 0;

            for win in self.windows.iter().rev() {
                if !win.mapped || win.is_closed {
                    continue;
                }

                // Close button
                let (cx, cy, cw, ch) = win.close_button_rect();
                if mouse.x >= cx && mouse.x < cx + cw && mouse.y >= cy && mouse.y < cy + ch {
                    clicked_id = Some(win.id);
                    action = 1;
                    break;
                }

                // Maximize button
                let (mx, my, mw, mh) = win.maximize_button_rect();
                if mouse.x >= mx && mouse.x < mx + mw && mouse.y >= my && mouse.y < my + mh {
                    clicked_id = Some(win.id);
                    action = 2;
                    break;
                }

                // Minimize button
                let (mn_x, mn_y, mn_w, mn_h) = win.minimize_button_rect();
                if mouse.x >= mn_x && mouse.x < mn_x + mn_w && mouse.y >= mn_y && mouse.y < mn_y + mn_h {
                    clicked_id = Some(win.id);
                    action = 3;
                    break;
                }

                let (fx, fy, fw, fh) = win.full_rect();
                if mouse.x >= fx && mouse.x < fx + fw && mouse.y >= fy && mouse.y < fy + fh {
                    clicked_id = Some(win.id);

                    // Check borders for resizing (if in Normal state)
                    if win.state == WindowState::Normal {
                        let on_right = mouse.x >= fx + fw - 8;
                        let on_bottom = mouse.y >= fy + fh - 8;
                        if on_right && on_bottom {
                            action = 5; // Bottom-Right corner
                            break;
                        } else if on_right {
                            action = 6; // Right edge
                            break;
                        } else if on_bottom {
                            action = 7; // Bottom edge
                            break;
                        }
                    }

                    // Titlebar
                    if mouse.y < fy + TITLEBAR_HEIGHT {
                        action = 4;
                        drag_ox = mouse.x - win.x;
                        drag_oy = mouse.y - win.y;
                    } else {
                        action = 8; // Client area
                    }
                    break;
                }
            }

            if let Some(id) = clicked_id {
                match action {
                    1 => {
                        // Close [X]
                        self.destroy_window(id);
                    }
                    2 => {
                        // Maximize / Restore [□]
                        self.focus_window(id);
                        self.raise_to_front(id);
                        self.toggle_maximize(id, screen_w, screen_h);
                    }
                    3 => {
                        // Minimize [-]
                        self.unmap_window(id);
                    }
                    4 => {
                        // Titlebar drag
                        self.focus_window(id);
                        self.raise_to_front(id);
                        if let Some(win) = self.get_window_mut(id) {
                            if win.state == WindowState::Normal {
                                win.is_dragging = true;
                                win.drag_offset_x = drag_ox;
                                win.drag_offset_y = drag_oy;
                            }
                        }
                    }
                    5..=7 => {
                        // Border resize
                        self.focus_window(id);
                        self.raise_to_front(id);
                        if let Some(win) = self.get_window_mut(id) {
                            win.resize_mode = match action {
                                5 => ResizeMode::BottomRight,
                                6 => ResizeMode::Right,
                                7 => ResizeMode::Bottom,
                                _ => ResizeMode::None,
                            };
                            win.resize_start_mouse_x = mouse.x;
                            win.resize_start_mouse_y = mouse.y;
                            win.resize_start_w = win.width;
                            win.resize_start_h = win.height;
                        }
                    }
                    8 => {
                        // Client area click
                        self.focus_window(id);
                        self.raise_to_front(id);
                        if let Some(win) = self.get_window_mut(id) {
                            win.client_has_mouse_grab = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        // 3. External app maintenance and complete input forwarding
        for win in &mut self.windows {
            if !win.mapped || win.is_closed {
                continue;
            }

            win.check_external_liveness();

            if win.external_app.is_some() {
                // Throttled pixel refresh at ~30 FPS
                let should_read = win
                    .external_app
                    .as_ref()
                    .map(|app| app.last_read.elapsed().as_millis() >= 33)
                    .unwrap_or(false);

                if should_read {
                    if let Some(ref mut app) = win.external_app {
                        app.last_read = Instant::now();
                    }
                    win.refresh_external_pixels();
                }

                // Mouse forwarding via xdotool — only for Xvfb-backed windows.
                // Native Wayland clients receive input directly from the compositor.
                let is_native = win.external_app.as_ref().map(|a| a.is_native_wayland).unwrap_or(false);
                let (cl_x, cl_y, cl_w, cl_h) = win.client_rect();
                let mouse_in_client = mouse.x >= cl_x && mouse.x < cl_x + cl_w && mouse.y >= cl_y && mouse.y < cl_y + cl_h;

                if !is_native && (mouse_in_client || win.client_has_mouse_grab) {
                    let lx = (mouse.x - cl_x).clamp(0, cl_w - 1);
                    let ly = (mouse.y - cl_y).clamp(0, cl_h - 1);

                    if let Some(ref mut app) = win.external_app {
                        let disp = app.display_num;

                        // Mouse Move
                        if lx != app.prev_mouse_x || ly != app.prev_mouse_y {
                            app.prev_mouse_x = lx;
                            app.prev_mouse_y = ly;
                            let _ = self.xdo_tx.send(XdoCmd::MouseMove { display: disp, x: lx, y: ly });
                        }

                        // Left button down & up
                        if mouse.left_clicked {
                            let _ = self.xdo_tx.send(XdoCmd::MouseDown { display: disp, button: 1 });
                        }
                        if mouse.left_released {
                            let _ = self.xdo_tx.send(XdoCmd::MouseUp { display: disp, button: 1 });
                            win.client_has_mouse_grab = false;
                        }

                        // Middle button down & up
                        if mouse.middle_clicked {
                            let _ = self.xdo_tx.send(XdoCmd::MouseDown { display: disp, button: 2 });
                        }
                        if mouse.middle_released {
                            let _ = self.xdo_tx.send(XdoCmd::MouseUp { display: disp, button: 2 });
                        }

                        // Right button down & up
                        if mouse.right_clicked {
                            let _ = self.xdo_tx.send(XdoCmd::MouseDown { display: disp, button: 3 });
                        }
                        if mouse.right_released {
                            let _ = self.xdo_tx.send(XdoCmd::MouseUp { display: disp, button: 3 });
                        }

                        // Mouse Scroll Wheel (4 = up, 5 = down)
                        if mouse.scroll_delta > 0 {
                            for _ in 0..mouse.scroll_delta {
                                let _ = self.xdo_tx.send(XdoCmd::Click { display: disp, button: 4 });
                            }
                        } else if mouse.scroll_delta < 0 {
                            for _ in 0..(-mouse.scroll_delta) {
                                let _ = self.xdo_tx.send(XdoCmd::Click { display: disp, button: 5 });
                            }
                        }
                    }
                }
            }
        }

        // Clean up closed windows
        let closed_ids: Vec<u32> = self.windows.iter().filter(|w| w.is_closed).map(|w| w.id).collect();
        for id in closed_ids {
            self.destroy_window(id);
        }
    }

    pub fn handle_key(&mut self, ch: char) {
        if let Some(focused_id) = self.focused_id {
            if let Some(win) = self.get_window(focused_id) {
                if let Some(ref app) = win.external_app {
                    let key_str = match ch {
                        '\n' | '\r' => "Return".to_string(),
                        '\x08' | '\x7f' => "BackSpace".to_string(),
                        '\x1b' => "Escape".to_string(),
                        '\t' => "Tab".to_string(),
                        '\x01' => "ctrl+a".to_string(),
                        '\x03' => "ctrl+c".to_string(),
                        '\x16' => "ctrl+v".to_string(),
                        '\x18' => "ctrl+x".to_string(),
                        '\x1a' => "ctrl+z".to_string(),
                        '\x17' => "ctrl+w".to_string(),
                        '\x14' => "ctrl+t".to_string(),
                        '\x12' => "ctrl+r".to_string(),
                        '\x06' => "ctrl+f".to_string(),
                        '\x0c' => "ctrl+l".to_string(),
                        ' ' => "space".to_string(),
                        '/' => "slash".to_string(),
                        '.' => "period".to_string(),
                        ',' => "comma".to_string(),
                        ':' => "colon".to_string(),
                        ';' => "semicolon".to_string(),
                        '-' => "minus".to_string(),
                        '=' => "equal".to_string(),
                        '[' => "bracketleft".to_string(),
                        ']' => "bracketright".to_string(),
                        '\'' => "apostrophe".to_string(),
                        '\\' => "backslash".to_string(),
                        '`' => "grave".to_string(),
                        '@' => "at".to_string(),
                        '#' => "numbersign".to_string(),
                        '!' => "exclam".to_string(),
                        '$' => "dollar".to_string(),
                        '%' => "percent".to_string(),
                        '^' => "asciicircum".to_string(),
                        '&' => "ampersand".to_string(),
                        '*' => "asterisk".to_string(),
                        '(' => "parenleft".to_string(),
                        ')' => "parenright".to_string(),
                        '_' => "underscore".to_string(),
                        '+' => "plus".to_string(),
                        '{' => "braceleft".to_string(),
                        '}' => "braceright".to_string(),
                        '|' => "bar".to_string(),
                        '\"' => "quotedbl".to_string(),
                        '~' => "asciitilde".to_string(),
                        '<' => "less".to_string(),
                        '>' => "greater".to_string(),
                        '?' => "question".to_string(),
                        _ => ch.to_string(),
                    };

                    let _ = self.xdo_tx.send(XdoCmd::Key {
                        display: app.display_num,
                        key: key_str,
                    });
                }
            }
        }
    }

    /// Renders all windows from back to front
    pub fn draw(&self, gfx: &mut Graphics, mouse: &MouseState) {
        for win in &self.windows {
            if !win.mapped || win.is_closed {
                continue;
            }

            let full_w = (win.width + 2) as i32;
            let full_h = (win.height + TITLEBAR_HEIGHT as u32 + 1) as i32;

            // 1. Titlebar
            let tb_color = if win.focused {
                TITLEBAR_COLOR_FOCUSED
            } else {
                TITLEBAR_COLOR_UNFOCUSED
            };
            gfx.fill_rect(win.x, win.y, full_w, TITLEBAR_HEIGHT, tb_color);

            // Title text
            let title_color = if win.focused {
                Color::new(240, 240, 250)
            } else {
                Color::new(160, 165, 180)
            };
            gfx.draw_string((win.x + 10) as u32, (win.y + 8) as u32, &win.title, title_color);

            // Minimize button [-]
            let (mn_x, mn_y, mn_w, mn_h) = win.minimize_button_rect();
            let hover_min = mouse.x >= mn_x && mouse.x < mn_x + mn_w && mouse.y >= mn_y && mouse.y < mn_y + mn_h;
            let min_bg = if hover_min { Color::new(75, 80, 95) } else { Color::new(55, 60, 72) };
            let min_fg = if hover_min { Color::new(255, 255, 255) } else { Color::new(200, 205, 215) };
            gfx.fill_rect(mn_x, mn_y, mn_w, mn_h, min_bg);
            draw_icon(gfx, IconId::Minimize, mn_x + (mn_w - 12) / 2, mn_y + (mn_h - 12) / 2, 12, min_fg);

            // Maximize / Restore button [□]
            let (mx, my, mw, mh) = win.maximize_button_rect();
            let hover_max = mouse.x >= mx && mouse.x < mx + mw && mouse.y >= my && mouse.y < my + mh;
            let max_bg = if hover_max { Color::new(75, 80, 95) } else { Color::new(55, 60, 72) };
            let max_fg = if hover_max { Color::new(255, 255, 255) } else { Color::new(200, 205, 215) };
            let max_icon = if win.state == WindowState::Maximized { IconId::Restore } else { IconId::Maximize };
            gfx.fill_rect(mx, my, mw, mh, max_bg);
            draw_icon(gfx, max_icon, mx + (mw - 12) / 2, my + (mh - 12) / 2, 12, max_fg);

            // Close button [X]
            let (cx, cy, cw, ch) = win.close_button_rect();
            let hover_close = mouse.x >= cx && mouse.x < cx + cw && mouse.y >= cy && mouse.y < cy + ch;
            let close_bg = if hover_close { Color::new(220, 50, 50) } else { Color::new(175, 40, 40) };
            let close_fg = if hover_close { Color::new(255, 255, 255) } else { Color::new(240, 240, 240) };
            gfx.fill_rect(cx, cy, cw, ch, close_bg);
            draw_icon(gfx, IconId::Close, cx + (cw - 12) / 2, cy + (ch - 12) / 2, 12, close_fg);

            // 2. Client area surface blit
            let (cl_x, cl_y, cl_w, cl_h) = win.client_rect();
            gfx.blit_bgrx_rect(cl_x, cl_y, cl_w, cl_h, &win.surface);

            // 3. Clean 1px border around whole window
            let border_color = if win.focused {
                BORDER_COLOR_FOCUSED
            } else {
                BORDER_COLOR_UNFOCUSED
            };
            gfx.draw_rect(win.x, win.y, full_w, full_h, border_color);

            // Resize handle notch in bottom-right corner (if Normal state)
            if win.state == WindowState::Normal {
                let rx = win.x + full_w - 6;
                let ry = win.y + full_h - 6;
                gfx.draw_pixel_fast(rx as u32, (ry + 4) as u32, border_color);
                gfx.draw_pixel_fast((rx + 2) as u32, (ry + 2) as u32, border_color);
                gfx.draw_pixel_fast((rx + 4) as u32, ry as u32, border_color);
            }
        }
    }

    pub fn draw_unfocused(&self, gfx: &mut Graphics, mouse: &MouseState) {
        for win in &self.windows {
            if win.mapped && !win.focused && !win.is_closed {
                let full_w = (win.width + 2) as i32;
                let full_h = (win.height + TITLEBAR_HEIGHT as u32 + 1) as i32;

                gfx.fill_squircle(win.x, win.y, full_w, full_h, 12, TITLEBAR_COLOR_UNFOCUSED);
                gfx.draw_string((win.x + 10) as u32, (win.y + 8) as u32, &win.title, Color::new(160, 165, 180));

                let (mn_x, mn_y, mn_w, mn_h) = win.minimize_button_rect();
                gfx.fill_squircle(mn_x, mn_y, mn_w, mn_h, 6, Color::new(55, 60, 72));
                draw_icon(gfx, IconId::Minimize, mn_x + (mn_w - 12) / 2, mn_y + (mn_h - 12) / 2, 12, Color::new(180, 185, 195));

                let (mx, my, mw, mh) = win.maximize_button_rect();
                gfx.fill_squircle(mx, my, mw, mh, 6, Color::new(55, 60, 72));
                let max_icon = if win.state == WindowState::Maximized { IconId::Restore } else { IconId::Maximize };
                draw_icon(gfx, max_icon, mx + (mw - 12) / 2, my + (mh - 12) / 2, 12, Color::new(180, 185, 195));

                let (cx, cy, cw, ch) = win.close_button_rect();
                let hover_close = mouse.x >= cx && mouse.x < cx + cw && mouse.y >= cy && mouse.y < cy + ch;
                let close_bg = if hover_close { Color::new(220, 50, 50) } else { Color::new(175, 40, 40) };
                let close_fg = if hover_close { Color::new(255, 255, 255) } else { Color::new(220, 220, 220) };
                gfx.fill_squircle(cx, cy, cw, ch, 6, close_bg);
                draw_icon(gfx, IconId::Close, cx + (cw - 12) / 2, cy + (ch - 12) / 2, 12, close_fg);

                let (cl_x, cl_y, cl_w, cl_h) = win.client_rect();
                gfx.blit_bgrx_rect(cl_x, cl_y, cl_w, cl_h, &win.surface);

                gfx.draw_squircle(win.x, win.y, full_w, full_h, 12, BORDER_COLOR_UNFOCUSED);
            }
        }
    }

    pub fn draw_focused(&self, gfx: &mut Graphics, mouse: &MouseState) {
        for win in &self.windows {
            if win.mapped && win.focused && !win.is_closed {
                let full_w = (win.width + 2) as i32;
                let full_h = (win.height + TITLEBAR_HEIGHT as u32 + 1) as i32;

                gfx.fill_squircle(win.x, win.y, full_w, full_h, 12, TITLEBAR_COLOR_FOCUSED);
                gfx.draw_string((win.x + 10) as u32, (win.y + 8) as u32, &win.title, Color::new(240, 240, 250));

                let (mn_x, mn_y, mn_w, mn_h) = win.minimize_button_rect();
                let hover_min = mouse.x >= mn_x && mouse.x < mn_x + mn_w && mouse.y >= mn_y && mouse.y < mn_y + mn_h;
                let min_bg = if hover_min { Color::new(75, 80, 95) } else { Color::new(55, 60, 72) };
                let min_fg = if hover_min { Color::new(255, 255, 255) } else { Color::new(200, 205, 215) };
                gfx.fill_squircle(mn_x, mn_y, mn_w, mn_h, 6, min_bg);
                draw_icon(gfx, IconId::Minimize, mn_x + (mn_w - 12) / 2, mn_y + (mn_h - 12) / 2, 12, min_fg);

                let (mx, my, mw, mh) = win.maximize_button_rect();
                let hover_max = mouse.x >= mx && mouse.x < mx + mw && mouse.y >= my && mouse.y < my + mh;
                let max_bg = if hover_max { Color::new(75, 80, 95) } else { Color::new(55, 60, 72) };
                let max_fg = if hover_max { Color::new(255, 255, 255) } else { Color::new(200, 205, 215) };
                let max_icon = if win.state == WindowState::Maximized { IconId::Restore } else { IconId::Maximize };
                gfx.fill_squircle(mx, my, mw, mh, 6, max_bg);
                draw_icon(gfx, max_icon, mx + (mw - 12) / 2, my + (mh - 12) / 2, 12, max_fg);

                let (cx, cy, cw, ch) = win.close_button_rect();
                let hover_close = mouse.x >= cx && mouse.x < cx + cw && mouse.y >= cy && mouse.y < cy + ch;
                let close_bg = if hover_close { Color::new(220, 50, 50) } else { Color::new(175, 40, 40) };
                let close_fg = if hover_close { Color::new(255, 255, 255) } else { Color::new(240, 240, 240) };
                gfx.fill_squircle(cx, cy, cw, ch, 6, close_bg);
                draw_icon(gfx, IconId::Close, cx + (cw - 12) / 2, cy + (ch - 12) / 2, 12, close_fg);

                let (cl_x, cl_y, cl_w, cl_h) = win.client_rect();
                gfx.blit_bgrx_rect(cl_x, cl_y, cl_w, cl_h, &win.surface);

                gfx.draw_squircle(win.x, win.y, full_w, full_h, 12, BORDER_COLOR_FOCUSED);

                if win.state == WindowState::Normal {
                    let rx = win.x + full_w - 6;
                    let ry = win.y + full_h - 6;
                    gfx.draw_pixel_fast(rx as u32, (ry + 4) as u32, BORDER_COLOR_FOCUSED);
                    gfx.draw_pixel_fast((rx + 2) as u32, (ry + 2) as u32, BORDER_COLOR_FOCUSED);
                    gfx.draw_pixel_fast((rx + 4) as u32, ry as u32, BORDER_COLOR_FOCUSED);
                }
            }
        }
    }
}
