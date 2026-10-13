use crate::color::Color;
use crate::graphics::Graphics;
use crate::input::Action;
use crate::mouse::MouseState;
use crate::widget::Window;
use std::os::unix::io::RawFd;

pub struct PtySession {
    pub master_fd: RawFd,
    pub child_pid: libc::pid_t,
}

impl PtySession {
    pub fn spawn(cols: u16, rows: u16) -> Option<Self> {
        unsafe {
            let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            if master < 0 {
                return None;
            }
            if libc::grantpt(master) < 0 || libc::unlockpt(master) < 0 {
                libc::close(master);
                return None;
            }
            let pts_ptr = libc::ptsname(master);
            if pts_ptr.is_null() {
                libc::close(master);
                return None;
            }
            let pts_name = std::ffi::CStr::from_ptr(pts_ptr).to_owned();

            // Make master non-blocking
            let flags = libc::fcntl(master, libc::F_GETFL, 0);
            libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);

            // Configure initial winsize
            let ws = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: cols * 8,
                ws_ypixel: rows * 14,
            };
            libc::ioctl(master, libc::TIOCSWINSZ, &ws);

            let pid = libc::fork();
            if pid < 0 {
                libc::close(master);
                return None;
            }

            if pid == 0 {
                // Child process (shell)
                libc::close(master);
                libc::setsid();

                let slave = libc::open(pts_name.as_ptr(), libc::O_RDWR);
                if slave < 0 {
                    libc::_exit(1);
                }

                libc::ioctl(slave, libc::TIOCSCTTY, 0);

                libc::dup2(slave, libc::STDIN_FILENO);
                libc::dup2(slave, libc::STDOUT_FILENO);
                libc::dup2(slave, libc::STDERR_FILENO);
                if slave > 2 {
                    libc::close(slave);
                }

                let user_home = "/home/leviathan";
                let _ = std::fs::create_dir_all(user_home);
                if let Ok(c_home) = std::ffi::CString::new(user_home) {
                    libc::chdir(c_home.as_ptr());
                }

                std::env::set_var("TERM", "xterm-256color");
                std::env::set_var("HOME", user_home);
                std::env::set_var("USER", "leviathan");
                std::env::set_var("LOGNAME", "leviathan");
                std::env::set_var("SHELL", "/bin/bash");
                std::env::set_var("PATH", "/bin:/usr/bin:/sbin:/usr/sbin:/usr/local/bin:/usr/games");
                std::env::set_var("LD_LIBRARY_PATH", "/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu");
                std::env::set_var("PS1", "\\[\\033[01;32m\\]leviathan@leviathan-os\\[\\033[00m\\]:\\[\\033[01;34m\\]\\w\\[\\033[00m\\]\\$ ");

                // Drop privileges to UID 1000 if running as root
                if libc::geteuid() == 0 {
                    libc::setgid(1000);
                    libc::setuid(1000);
                }

                let shell = if std::path::Path::new("/bin/bash").exists() {
                    std::ffi::CString::new("/bin/bash").unwrap()
                } else {
                    std::ffi::CString::new("/bin/sh").unwrap()
                };

                let arg0 = std::ffi::CString::new("-bash").unwrap();
                let args = [arg0.as_ptr(), std::ptr::null()];

                libc::execvp(shell.as_ptr(), args.as_ptr());
                libc::_exit(127);
            }

            Some(Self {
                master_fd: master,
                child_pid: pid,
            })
        }
    }

    pub fn write_bytes(&self, data: &[u8]) {
        if self.master_fd >= 0 && !data.is_empty() {
            unsafe {
                libc::write(self.master_fd, data.as_ptr() as *const libc::c_void, data.len());
            }
        }
    }

    pub fn read_bytes(&self, buf: &mut [u8]) -> isize {
        if self.master_fd < 0 {
            return -1;
        }
        unsafe { libc::read(self.master_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) }
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        if self.master_fd >= 0 {
            let ws = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: cols * 8,
                ws_ypixel: rows * 14,
            };
            unsafe {
                libc::ioctl(self.master_fd, libc::TIOCSWINSZ, &ws);
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        if self.child_pid <= 0 {
            return false;
        }
        let mut status = 0;
        let res = unsafe { libc::waitpid(self.child_pid, &mut status, libc::WNOHANG) };
        res == 0
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if self.master_fd >= 0 {
            unsafe {
                libc::close(self.master_fd);
            }
            self.master_fd = -1;
        }
        if self.child_pid > 0 {
            unsafe {
                libc::kill(self.child_pid, libc::SIGHUP);
                let mut status = 0;
                libc::waitpid(self.child_pid, &mut status, libc::WNOHANG);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TermCell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
}

impl Default for TermCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::new(225, 230, 240),
            bg: Color::new(14, 18, 26),
            bold: false,
        }
    }
}

enum AnsiState {
    Ground,
    Escape,
    Csi,
    Osc,
}

pub struct TerminalApp {
    pub window: Window,
    pub pty: Option<PtySession>,
    pub cols: usize,
    pub rows: usize,
    pub grid: Vec<Vec<TermCell>>,
    pub scrollback: Vec<Vec<TermCell>>,
    pub scroll_offset: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
    cur_fg: Color,
    cur_bg: Color,
    cur_bold: bool,
    cursor_visible: bool,
    cursor_blink: f32,
    ansi_state: AnsiState,
    csi_params: Vec<u32>,
    current_param: u32,
    has_param: bool,
    csi_private: bool,
    osc_buf: Vec<u8>,
    pub pending_gui_launch: Option<(String, Vec<String>, String)>,
    pub input_line: String, // Kept for compatibility with launcher/files delegation
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub is_selecting: bool,
}

impl TerminalApp {
    const DEFAULT_FG: Color = Color::new(225, 230, 240);
    const DEFAULT_BG: Color = Color::new(14, 18, 26);

    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        let cols = ((w - 20) / 8).max(20) as usize;
        let rows = ((h - 40) / 14).max(5) as usize;

        let mut grid = Vec::with_capacity(rows);
        for _ in 0..rows {
            grid.push(vec![TermCell::default(); cols]);
        }

        let pty = PtySession::spawn(cols as u16, rows as u16);

        Self {
            window: Window::new("leviathan@leviathan-os: ~ (Terminal)", x, y, w, h),
            pty,
            cols,
            rows,
            grid,
            scrollback: Vec::new(),
            scroll_offset: 0,
            cursor_x: 0,
            cursor_y: 0,
            cur_fg: Self::DEFAULT_FG,
            cur_bg: Self::DEFAULT_BG,
            cur_bold: false,
            cursor_visible: true,
            cursor_blink: 0.0,
            ansi_state: AnsiState::Ground,
            csi_params: Vec::new(),
            current_param: 0,
            has_param: false,
            csi_private: false,
            osc_buf: Vec::new(),
            pending_gui_launch: None,
            input_line: String::new(),
            selection_start: None,
            selection_end: None,
            is_selecting: false,
        }
    }

    pub fn execute_command(&mut self) {
        if !self.input_line.is_empty() {
            let mut line = self.input_line.clone();
            line.push('\n');
            self.send_bytes(line.as_bytes());
            self.input_line.clear();
        }
    }

    pub fn send_bytes(&mut self, data: &[u8]) {
        if let Some(ref pty) = self.pty {
            pty.write_bytes(data);
            self.scroll_offset = 0;
        }
    }

    pub fn is_cell_selected(&self, row: usize, col: usize) -> bool {
        if let (Some((sr, sc)), Some((er, ec))) = (self.selection_start, self.selection_end) {
            let (start, end) = if (sr, sc) <= (er, ec) {
                ((sr, sc), (er, ec))
            } else {
                ((er, ec), (sr, sc))
            };
            if row < start.0 || row > end.0 {
                return false;
            }
            if row == start.0 && row == end.0 {
                col >= start.1 && col <= end.1
            } else if row == start.0 {
                col >= start.1
            } else if row == end.0 {
                col <= end.1
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.is_selecting = false;
    }

    pub fn get_selected_text(&self) -> String {
        if let (Some((sr, sc)), Some((er, ec))) = (self.selection_start, self.selection_end) {
            let (start, end) = if (sr, sc) <= (er, ec) {
                ((sr, sc), (er, ec))
            } else {
                ((er, ec), (sr, sc))
            };

            let mut text = String::new();
            for r in start.0..=end.0 {
                if let Some(cells) = self.grid.get(r) {
                    let c_start = if r == start.0 { start.1 } else { 0 };
                    let c_end = if r == end.0 { end.1.min(cells.len().saturating_sub(1)) } else { cells.len().saturating_sub(1) };
                    let mut line_str = String::new();
                    for c in c_start..=c_end {
                        if let Some(cell) = cells.get(c) {
                            line_str.push(cell.ch);
                        }
                    }
                    text.push_str(line_str.trim_end());
                    if r != end.0 {
                        text.push('\n');
                    }
                }
            }
            text
        } else {
            String::new()
        }
    }

    pub fn handle_char(&mut self, ch: char) {
        if !self.window.is_open || !self.window.is_focused {
            return;
        }

        match ch {
            '\n' | '\r' => self.send_bytes(b"\r"),
            '\x08' | '\x7f' => self.send_bytes(b"\x7f"),
            '\t' => self.send_bytes(b"\t"),
            '\x03' => {
                // Ctrl+C: copy selection if text is selected, else send SIGINT
                if self.selection_start.is_some() && self.selection_end.is_some() {
                    let text = self.get_selected_text();
                    if !text.is_empty() {
                        crate::clipboard::Clipboard::set_text(&text);
                        self.clear_selection();
                        return;
                    }
                }
                self.send_bytes(b"\x03");
            }
            '\x16' => {
                // Ctrl+V: paste from system clipboard
                let paste = crate::clipboard::Clipboard::get_text();
                if !paste.is_empty() {
                    self.send_bytes(paste.as_bytes());
                }
            }
            '\x01' => {
                // Ctrl+A: select all visible rows
                self.selection_start = Some((0, 0));
                self.selection_end = Some((self.rows.saturating_sub(1), self.cols.saturating_sub(1)));
            }
            '\x18' => {
                // Ctrl+X: cut (copy selection and clear selection)
                let text = self.get_selected_text();
                if !text.is_empty() {
                    crate::clipboard::Clipboard::set_text(&text);
                    self.clear_selection();
                }
            }
            '\x04' => self.send_bytes(b"\x04"), // Ctrl+D
            '\x0c' => self.send_bytes(b"\x0c"), // Ctrl+L
            '\x15' => self.send_bytes(b"\x15"), // Ctrl+U
            '\x17' => self.send_bytes(b"\x17"), // Ctrl+W
            c if !c.is_control() => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                self.send_bytes(s.as_bytes());
            }
            _ => {}
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        if !self.window.is_open || !self.window.is_focused {
            return;
        }

        match action {
            Action::ArrowUp => self.send_bytes(b"\x1b[A"),
            Action::ArrowDown => self.send_bytes(b"\x1b[B"),
            Action::ArrowRight => self.send_bytes(b"\x1b[C"),
            Action::ArrowLeft => self.send_bytes(b"\x1b[D"),
            Action::Home => self.send_bytes(b"\x1b[H"),
            Action::End => self.send_bytes(b"\x1b[F"),
            Action::PageUp => {
                self.scroll_offset = (self.scroll_offset + 10).min(self.scrollback.len());
            }
            Action::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            Action::Backspace => self.send_bytes(b"\x7f"),
            Action::Enter     => self.send_bytes(b"\r"),
            Action::Delete    => self.send_bytes(b"\x1b[3~"),
            Action::Escape    => self.send_bytes(b"\x1b"),
            Action::Char(b) => self.handle_char(b as char),
            _ => {}
        }
    }

    fn ensure_cell(&mut self, row: usize, col: usize) {
        if row < self.grid.len() {
            if col >= self.grid[row].len() {
                self.grid[row].resize(col + 1, TermCell::default());
            }
        }
    }

    fn scroll_up(&mut self) {
        if !self.grid.is_empty() {
            let top_line = self.grid.remove(0);
            self.scrollback.push(top_line);
            if self.scrollback.len() > 2000 {
                self.scrollback.remove(0);
            }
            self.grid.push(vec![TermCell::default(); self.cols]);
        }
    }

    fn process_byte(&mut self, b: u8) {
        match self.ansi_state {
            AnsiState::Ground => match b {
                0x1b => self.ansi_state = AnsiState::Escape,
                b'\r' => self.cursor_x = 0,
                b'\n' => {
                    if self.cursor_y + 1 >= self.rows {
                        self.scroll_up();
                        self.cursor_y = self.rows - 1;
                    } else {
                        self.cursor_y += 1;
                    }
                }
                0x08 => self.cursor_x = self.cursor_x.saturating_sub(1),
                0x07 => {} // Bell - ignore
                b'\t' => {
                    self.cursor_x = ((self.cursor_x / 8) + 1) * 8;
                    if self.cursor_x >= self.cols {
                        self.cursor_x = 0;
                        if self.cursor_y + 1 >= self.rows {
                            self.scroll_up();
                        } else {
                            self.cursor_y += 1;
                        }
                    }
                }
                32..=126 => {
                    if self.cursor_x >= self.cols {
                        self.cursor_x = 0;
                        if self.cursor_y + 1 >= self.rows {
                            self.scroll_up();
                        } else {
                            self.cursor_y += 1;
                        }
                    }
                    self.ensure_cell(self.cursor_y, self.cursor_x);
                    self.grid[self.cursor_y][self.cursor_x] = TermCell {
                        ch: b as char,
                        fg: self.cur_fg,
                        bg: self.cur_bg,
                        bold: self.cur_bold,
                    };
                    self.cursor_x += 1;
                }
                _ => {}
            },
            AnsiState::Escape => match b {
                b'[' => {
                    self.ansi_state = AnsiState::Csi;
                    self.csi_params.clear();
                    self.current_param = 0;
                    self.has_param = false;
                    self.csi_private = false;
                }
                b']' => {
                    self.ansi_state = AnsiState::Osc;
                    self.osc_buf.clear();
                }
                _ => self.ansi_state = AnsiState::Ground,
            },
            AnsiState::Csi => match b {
                b'?' => self.csi_private = true,
                b'0'..=b'9' => {
                    self.current_param = self.current_param * 10 + (b - b'0') as u32;
                    self.has_param = true;
                }
                b';' => {
                    self.csi_params.push(if self.has_param { self.current_param } else { 0 });
                    self.current_param = 0;
                    self.has_param = false;
                }
                b'm' => {
                    if self.has_param {
                        self.csi_params.push(self.current_param);
                    }
                    if self.csi_params.is_empty() {
                        self.csi_params.push(0);
                    }
                    self.apply_sgr();
                    self.ansi_state = AnsiState::Ground;
                }
                b'H' | b'f' => {
                    if self.has_param {
                        self.csi_params.push(self.current_param);
                    }
                    let r = self.csi_params.first().copied().unwrap_or(1);
                    let c = self.csi_params.get(1).copied().unwrap_or(1);
                    self.cursor_y = (r.saturating_sub(1) as usize).min(self.rows.saturating_sub(1));
                    self.cursor_x = (c.saturating_sub(1) as usize).min(self.cols.saturating_sub(1));
                    self.ansi_state = AnsiState::Ground;
                }
                b'A' => {
                    let n = if self.has_param { self.current_param as usize } else { 1 };
                    self.cursor_y = self.cursor_y.saturating_sub(n);
                    self.ansi_state = AnsiState::Ground;
                }
                b'B' => {
                    let n = if self.has_param { self.current_param as usize } else { 1 };
                    self.cursor_y = (self.cursor_y + n).min(self.rows.saturating_sub(1));
                    self.ansi_state = AnsiState::Ground;
                }
                b'C' => {
                    let n = if self.has_param { self.current_param as usize } else { 1 };
                    self.cursor_x = (self.cursor_x + n).min(self.cols.saturating_sub(1));
                    self.ansi_state = AnsiState::Ground;
                }
                b'D' => {
                    let n = if self.has_param { self.current_param as usize } else { 1 };
                    self.cursor_x = self.cursor_x.saturating_sub(n);
                    self.ansi_state = AnsiState::Ground;
                }
                b'J' => {
                    let mode = if self.has_param { self.current_param } else { 0 };
                    if mode == 2 || mode == 3 {
                        for row in &mut self.grid {
                            for cell in row.iter_mut() {
                                *cell = TermCell::default();
                            }
                        }
                        self.cursor_x = 0;
                        self.cursor_y = 0;
                    }
                    self.ansi_state = AnsiState::Ground;
                }
                b'K' => {
                    let mode = if self.has_param { self.current_param } else { 0 };
                    if self.cursor_y < self.grid.len() {
                        let start = if mode == 1 || mode == 2 { 0 } else { self.cursor_x };
                        let end = if mode == 1 { self.cursor_x + 1 } else { self.cols };
                        for x in start..end.min(self.grid[self.cursor_y].len()) {
                            self.grid[self.cursor_y][x] = TermCell::default();
                        }
                    }
                    self.ansi_state = AnsiState::Ground;
                }
                b'h' => {
                    if self.csi_private && self.current_param == 25 {
                        self.cursor_visible = true;
                    }
                    self.ansi_state = AnsiState::Ground;
                }
                b'l' => {
                    if self.csi_private && self.current_param == 25 {
                        self.cursor_visible = false;
                    }
                    self.ansi_state = AnsiState::Ground;
                }
                _ => self.ansi_state = AnsiState::Ground,
            },
            AnsiState::Osc => {
                if b == 0x07 || b == b'\\' {
                    if let Ok(s) = std::str::from_utf8(&self.osc_buf) {
                        if let Some((code, title)) = s.split_once(';') {
                            if (code == "0" || code == "2") && !title.is_empty() {
                                self.window.title = format!("{} (Terminal)", title);
                            }
                        }
                    }
                    self.ansi_state = AnsiState::Ground;
                } else {
                    self.osc_buf.push(b);
                    if self.osc_buf.len() > 256 {
                        self.ansi_state = AnsiState::Ground;
                    }
                }
            }
        }
    }

    fn apply_sgr(&mut self) {
        for &param in &self.csi_params {
            match param {
                0 => {
                    self.cur_fg = Self::DEFAULT_FG;
                    self.cur_bg = Self::DEFAULT_BG;
                    self.cur_bold = false;
                }
                1 => self.cur_bold = true,
                22 => self.cur_bold = false,
                30 => self.cur_fg = Color::new(30, 30, 30),
                31 => self.cur_fg = Color::new(235, 75, 75),
                32 => self.cur_fg = Color::new(60, 220, 100),
                33 => self.cur_fg = Color::new(245, 200, 60),
                34 => self.cur_fg = Color::new(75, 150, 245),
                35 => self.cur_fg = Color::new(200, 100, 230),
                36 => self.cur_fg = Color::new(0, 210, 230),
                37 => self.cur_fg = Color::new(230, 235, 245),
                39 => self.cur_fg = Self::DEFAULT_FG,
                40..=47 => self.cur_bg = Color::new(20, 26, 36),
                49 => self.cur_bg = Self::DEFAULT_BG,
                90 => self.cur_fg = Color::new(120, 125, 135),
                91 => self.cur_fg = Color::new(255, 95, 95),
                92 => self.cur_fg = Color::new(80, 240, 120),
                93 => self.cur_fg = Color::new(255, 220, 80),
                94 => self.cur_fg = Color::new(95, 175, 255),
                95 => self.cur_fg = Color::new(220, 120, 255),
                96 => self.cur_fg = Color::new(50, 230, 255),
                97 => self.cur_fg = Color::new(255, 255, 255),
                _ => {}
            }
        }
    }

    pub fn update(&mut self, mouse: &MouseState, screen_w: i32, screen_h: i32, dt: f32) {
        self.cursor_blink = (self.cursor_blink + dt * 2.0) % 2.0;
        self.window.update_drag(mouse, screen_w, screen_h);

        if !self.window.is_open {
            return;
        }

        // 1. Resize handling
        let client = self.window.client_rect();
        let new_cols = ((client.w - 16) / 8).max(20) as usize;
        let new_rows = ((client.h - 12) / 14).max(5) as usize;

        if new_cols != self.cols || new_rows != self.rows {
            self.cols = new_cols;
            self.rows = new_rows;
            if let Some(ref pty) = self.pty {
                pty.resize(new_cols as u16, new_rows as u16);
            }
            self.grid.resize(self.rows, vec![TermCell::default(); self.cols]);
            for row in &mut self.grid {
                row.resize(self.cols, TermCell::default());
            }
            if self.cursor_y >= self.rows {
                self.cursor_y = self.rows.saturating_sub(1);
            }
            if self.cursor_x >= self.cols {
                self.cursor_x = self.cols.saturating_sub(1);
            }
        }

        // 2. Mouse selection & right-click paste
        let term_x = client.x + 8;
        let term_y = client.y + 6;
        if self.window.is_focused && mouse.x >= term_x && mouse.x < client.x + client.w && mouse.y >= term_y && mouse.y < client.y + client.h {
            let col = ((mouse.x - term_x) / 8).clamp(0, self.cols.saturating_sub(1) as i32) as usize;
            let row = ((mouse.y - term_y) / 14).clamp(0, self.rows.saturating_sub(1) as i32) as usize;

            if mouse.left_pressed {
                if !self.is_selecting {
                    self.is_selecting = true;
                    self.selection_start = Some((row, col));
                    self.selection_end = Some((row, col));
                } else {
                    self.selection_end = Some((row, col));
                }
            } else {
                self.is_selecting = false;
            }

            // Right click paste
            if mouse.right_clicked {
                let paste = crate::clipboard::Clipboard::get_text();
                if !paste.is_empty() {
                    self.send_bytes(paste.as_bytes());
                }
            }
        } else if !mouse.left_pressed {
            self.is_selecting = false;
        }

        // 3. Mouse wheel scrolling
        if self.window.rect.contains(mouse.x, mouse.y) {
            if mouse.scroll_delta > 0 {
                self.scroll_offset = (self.scroll_offset + (mouse.scroll_delta as usize * 3)).min(self.scrollback.len());
            } else if mouse.scroll_delta < 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub((-mouse.scroll_delta as usize) * 3);
            }
        }

        // 4. Read output from PTY
        let master_fd = self.pty.as_ref().map(|p| p.master_fd);
        if let Some(fd) = master_fd {
            let mut buf = [0u8; 4096];
            loop {
                let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
                if n > 0 {
                    for &b in &buf[..n as usize] {
                        self.process_byte(b);
                    }
                } else {
                    break;
                }
            }

            let is_alive = self.pty.as_ref().map(|p| p.is_alive()).unwrap_or(false);
            if !is_alive && !self.window.title.contains("[Exited]") {
                self.window.title = format!("{} [Exited]", self.window.title);
            }
        }
    }

    pub fn draw(&self, gfx: &mut Graphics, mouse: &MouseState) {
        if !self.window.is_open || self.window.is_minimized {
            return;
        }

        self.window.draw_frame(gfx, mouse);

        let client = self.window.client_rect();
        client.draw_fill(gfx, Self::DEFAULT_BG);

        let left_x = (client.x + 8) as u32;
        let top_y = (client.y + 6) as u32;

        let visible_lines = self.rows;
        let total_scroll = self.scrollback.len();

        for r in 0..visible_lines {
            let row_y = top_y + (r as u32 * 14);
            if (row_y as i32) + 14 > client.y + client.h {
                break;
            }

            let line: Option<&Vec<TermCell>> = if self.scroll_offset > 0 {
                let scroll_idx = total_scroll.saturating_sub(self.scroll_offset).wrapping_add(r);
                if scroll_idx < total_scroll {
                    self.scrollback.get(scroll_idx)
                } else {
                    let grid_idx = scroll_idx - total_scroll;
                    self.grid.get(grid_idx)
                }
            } else {
                self.grid.get(r)
            };

            if let Some(cells) = line {
                for (c, cell) in cells.iter().enumerate().take(self.cols) {
                    let cell_x = left_x + (c as u32 * 8);
                    if (cell_x as i32) + 8 > client.x + client.w {
                        break;
                    }

                    let is_sel = self.scroll_offset == 0 && self.is_cell_selected(r, c);
                    let bg_color = if is_sel { Color::new(50, 100, 180) } else { cell.bg };
                    let fg_color = if is_sel { Color::new(255, 255, 255) } else { cell.fg };

                    if bg_color != Self::DEFAULT_BG || is_sel {
                        gfx.fill_rect(cell_x as i32, row_y as i32, 8, 14, bg_color);
                    }

                    if cell.ch != ' ' {
                        gfx.draw_string(cell_x, row_y + 1, &cell.ch.to_string(), fg_color);
                    }
                }
            }
        }

        // Draw cursor if not scrolled and cursor is visible
        if self.scroll_offset == 0 && self.cursor_visible && self.window.is_focused && self.cursor_blink < 1.0 {
            let cur_px = left_x as i32 + (self.cursor_x as i32 * 8);
            let cur_py = top_y as i32 + (self.cursor_y as i32 * 14);
            if cur_px + 8 <= client.x + client.w && cur_py + 14 <= client.y + client.h {
                gfx.fill_rect(cur_px, cur_py + 1, 8, 12, Color::new(0, 210, 230));
            }
        }

        // Scroll indicator if scrolled back
        if self.scroll_offset > 0 {
            let badge = format!("[▲ {}]", self.scroll_offset);
            gfx.draw_string(
                (client.x + client.w - (badge.len() as i32 * 8) - 10) as u32,
                (client.y + 4) as u32,
                &badge,
                Color::new(0, 210, 230),
            );
        }
    }
}
