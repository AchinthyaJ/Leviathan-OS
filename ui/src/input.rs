use std::cell::Cell;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};

pub const EV_KEY: u16 = 0x01;

// Linux input key codes (linux/input-event-codes.h)
pub const KEY_ESC: u16 = 1;
pub const KEY_1: u16 = 2;
pub const KEY_2: u16 = 3;
pub const KEY_3: u16 = 4;
pub const KEY_Q: u16 = 16;
pub const KEY_ENTER: u16 = 28;
pub const KEY_C: u16 = 46;
pub const KEY_L: u16 = 38;
pub const KEY_SPACE: u16 = 57;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Exit,
    CycleCamera,
    CycleShading,
    Scene1,
    Scene2,
    Scene3,
    TogglePause,
    Char(u8),
    Super,
    SuperT,
    SuperE,
    SuperM,
    SuperUp,
    SuperDown,
    AltTab,
    AltF4,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
    Backspace,
    Enter,
    Escape,
    Delete,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InputEvent {
    pub time_sec: usize,
    pub time_usec: usize,
    pub type_: u16,
    pub code: u16,
    pub value: i32, // 0 = release, 1 = press, 2 = autorepeat
}

pub struct InputManager {
    evdev_fds: Vec<RawFd>,
    tty_fds: Vec<RawFd>,
    orig_termios: Vec<(RawFd, libc::termios)>,
    shift_pressed: Cell<bool>,
    caps_lock: Cell<bool>,
    ctrl_pressed: Cell<bool>,
    alt_pressed: Cell<bool>,
    super_pressed: Cell<bool>,
    has_keyboard_evdev: bool,
}

impl InputManager {
    fn is_keyboard_device(fd: RawFd, path_str: &str) -> bool {
        let mut key_bits = [0u8; 64];
        let ret = unsafe { libc::ioctl(fd, 0x80404521u32 as _, key_bits.as_mut_ptr()) };
        if ret >= 0 {
            let has_a = (key_bits[30 / 8] & (1 << (30 % 8))) != 0;
            let has_space = (key_bits[57 / 8] & (1 << (57 % 8))) != 0;
            let has_enter = (key_bits[28 / 8] & (1 << (28 % 8))) != 0;
            if has_a || has_space || has_enter {
                return true;
            }
        }
        let dev_name_path = format!("/sys/class/input/{}/device/name", path_str.trim_start_matches("/dev/input/"));
        if let Ok(dev_name) = fs::read_to_string(&dev_name_path) {
            let lower = dev_name.to_lowercase();
            if lower.contains("keyboard") || lower.contains("keypad") || lower.contains("kbd") {
                return true;
            }
        }
        false
    }

    pub fn new() -> Self {
        let mut evdev_fds = Vec::new();
        let mut tty_fds = Vec::new();
        let mut orig_termios = Vec::new();
        let mut has_keyboard_evdev = false;

        // 1. Probe and open evdev keyboard devices
        if let Ok(entries) = fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("event") {
                    let path = entry.path();
                    if let Ok(file) = OpenOptions::new()
                        .read(true)
                        .custom_flags(libc::O_NONBLOCK)
                        .open(&path)
                    {
                        let fd = file.as_raw_fd();
                        if Self::is_keyboard_device(fd, &path.to_string_lossy()) {
                            // EVIOCGRAB: exclusive grab prevents kernel from echoing keystrokes to /dev/tty* or stdin!
                            let _ = unsafe { libc::ioctl(fd, 0x40044590, 1) };
                            println!("leviathan-ui: Connected to Linux keyboard evdev device {}", path.display());
                            std::mem::forget(file);
                            evdev_fds.push(fd);
                            has_keyboard_evdev = true;
                        } else {
                            drop(file);
                        }
                    }
                }
            }
        }

        // 2. Open Virtual Terminal console for cursor suppression and fallback
        let vt_path = "/dev/tty1\0";
        let fd = unsafe {
            libc::open(
                vt_path.as_ptr() as *const libc::c_char,
                libc::O_RDWR | libc::O_NONBLOCK | libc::O_NOCTTY,
            )
        };
        if fd >= 0 {
            let mut orig: libc::termios = unsafe { std::mem::zeroed() };
            if unsafe { libc::tcgetattr(fd, &mut orig) } == 0 {
                let mut raw = orig;
                raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
                raw.c_iflag &= !(libc::IXON | libc::ICRNL);
                raw.c_cc[libc::VMIN] = 0;
                raw.c_cc[libc::VTIME] = 0;
                unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) };
                orig_termios.push((fd, orig));
            }

            let hide_cursor = b"\x1b[?25l";
            unsafe { libc::write(fd, hide_cursor.as_ptr() as *const libc::c_void, hide_cursor.len()) };

            if !has_keyboard_evdev {
                tty_fds.push(fd);
            } else {
                unsafe { libc::close(fd) };
            }
        }

        Self {
            evdev_fds,
            tty_fds,
            orig_termios,
            shift_pressed: Cell::new(false),
            caps_lock: Cell::new(false),
            ctrl_pressed: Cell::new(false),
            alt_pressed: Cell::new(false),
            super_pressed: Cell::new(false),
            has_keyboard_evdev,
        }
    }

    /// Polls all input sources non-blockingly without duplicating events
    pub fn poll_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();

        // 1. Read evdev input events (exclusive when keyboard evdev is present)
        if self.has_keyboard_evdev {
            let mut ev_buf = [InputEvent::default(); 32];
            for &fd in &self.evdev_fds {
                let buf_ptr = ev_buf.as_mut_ptr() as *mut libc::c_void;
                let bytes_to_read = std::mem::size_of_val(&ev_buf);
                let n = unsafe { libc::read(fd, buf_ptr, bytes_to_read) };
                if n > 0 {
                    let count = n as usize / std::mem::size_of::<InputEvent>();
                    for ev in &ev_buf[..count] {
                        if ev.type_ == EV_KEY {
                            let is_press = ev.value == 1;
                            let is_release = ev.value == 0;

                            if is_release {
                                match ev.code {
                                    42 | 54 => self.shift_pressed.set(false),
                                    29 | 97 => self.ctrl_pressed.set(false),
                                    56 | 100 => self.alt_pressed.set(false),
                                    125 | 126 => self.super_pressed.set(false),
                                    _ => {}
                                }
                                continue;
                            }

                            // Track modifier state changes on press
                            match ev.code {
                                42 | 54 => {
                                    self.shift_pressed.set(true);
                                    continue;
                                }
                                58 => {
                                    if is_press {
                                        self.caps_lock.set(!self.caps_lock.get());
                                    }
                                    continue;
                                }
                                29 | 97 => {
                                    self.ctrl_pressed.set(true);
                                    continue;
                                }
                                56 | 100 => {
                                    self.alt_pressed.set(true);
                                    continue;
                                }
                                125 | 126 => {
                                    self.super_pressed.set(true);
                                    if is_press {
                                        actions.push(Action::Super);
                                    }
                                    continue;
                                }
                                _ => {}
                            }

                            // Only process key press (value == 1) and autorepeat (value == 2) for navigation/editing
                            let allow_repeat = match ev.code {
                                14 | 111 | 103 | 108 | 105 | 106 => true, // Backspace, Delete, Arrows
                                _ => false,
                            };

                            if is_press || (ev.value == 2 && allow_repeat) {
                                if ev.code < 0x110 || ev.code > 0x117 {
                                    if let Some(action) = self.action_from_keycode(ev.code) {
                                        actions.push(action);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            return actions;
        }

        // 2. Fallback ONLY when NO evdev keyboard is detected
        let mut byte_buf = [0u8; 32];
        for &fd in &self.tty_fds {
            let n = unsafe {
                libc::read(fd, byte_buf.as_mut_ptr() as *mut libc::c_void, byte_buf.len())
            };
            if n > 0 {
                let slice = &byte_buf[..n as usize];
                let mut idx = 0;
                while idx < slice.len() {
                    let b = slice[idx];
                    if b == 27 {
                        if idx + 2 < slice.len() && slice[idx + 1] == b'[' {
                            match slice[idx + 2] {
                                b'A' => { actions.push(Action::ArrowUp); idx += 3; continue; }
                                b'B' => { actions.push(Action::ArrowDown); idx += 3; continue; }
                                b'C' => { actions.push(Action::ArrowRight); idx += 3; continue; }
                                b'D' => { actions.push(Action::ArrowLeft); idx += 3; continue; }
                                b'H' => { actions.push(Action::Home); idx += 3; continue; }
                                b'F' => { actions.push(Action::End); idx += 3; continue; }
                                _ => { idx += 3; continue; }
                            }
                        } else if idx + 1 < slice.len() && (slice[idx + 1] == b'[' || slice[idx + 1] == b'O') {
                            idx += 2;
                            continue;
                        } else {
                            actions.push(Action::Exit);
                            idx += 1;
                            continue;
                        }
                    }
                    if let Some(action) = Self::action_from_char(b) {
                        actions.push(action);
                    }
                    idx += 1;
                }
            }
        }

        actions
    }

    pub fn action_from_keycode(&self, code: u16) -> Option<Action> {
        Self::action_from_keycode_state(
            code,
            self.shift_pressed.get(),
            self.caps_lock.get(),
            self.ctrl_pressed.get(),
            self.alt_pressed.get(),
            self.super_pressed.get(),
        )
    }

    pub fn action_from_keycode_state(
        code: u16,
        shift: bool,
        caps_lock: bool,
        ctrl: bool,
        alt: bool,
        super_key: bool,
    ) -> Option<Action> {
        // 1. Alt-based window management shortcuts
        if alt {
            match code {
                15 => return Some(Action::AltTab),  // Alt+Tab -> Switch windows
                62 => return Some(Action::AltF4),   // Alt+F4 -> Close focused window
                _ => {}
            }
        }

        // 2. Super (Windows key) system shortcuts
        if super_key {
            match code {
                20 => return Some(Action::SuperT),     // Super+T -> Terminal
                18 => return Some(Action::SuperE),     // Super+E -> Files
                50 => return Some(Action::SuperM),     // Super+M -> Minimize
                103 => return Some(Action::SuperUp),   // Super+Up -> Maximize
                108 => return Some(Action::SuperDown), // Super+Down -> Restore/Minimize
                57 => return Some(Action::Super),      // Super+Space -> Launcher
                _ => {}
            }
        }

        // 3. Navigation and arrow keys
        match code {
            103 => return Some(Action::ArrowUp),
            108 => return Some(Action::ArrowDown),
            105 => return Some(Action::ArrowLeft),
            106 => return Some(Action::ArrowRight),
            104 => return Some(Action::PageUp),
            109 => return Some(Action::PageDown),
            102 => return Some(Action::Home),
            107 => return Some(Action::End),
            _ => {}
        }

        let is_upper = shift ^ caps_lock;

        // If Ctrl is held down with letters
        if ctrl {
            match code {
                16 => return Some(Action::Char(17)), // Ctrl+Q
                17 => return Some(Action::Char(23)), // Ctrl+W
                18 => return Some(Action::Char(5)),  // Ctrl+E
                19 => return Some(Action::Char(18)), // Ctrl+R
                20 => return Some(Action::Char(20)), // Ctrl+T
                21 => return Some(Action::Char(25)), // Ctrl+Y
                22 => return Some(Action::Char(21)), // Ctrl+U
                23 => return Some(Action::Char(9)),  // Ctrl+I
                24 => return Some(Action::Char(15)), // Ctrl+O
                25 => return Some(Action::Char(16)), // Ctrl+P
                30 => return Some(Action::Char(1)),  // Ctrl+A
                31 => return Some(Action::Char(19)), // Ctrl+S
                32 => return Some(Action::Char(4)),  // Ctrl+D
                33 => return Some(Action::Char(6)),  // Ctrl+F
                34 => return Some(Action::Char(7)),  // Ctrl+G
                35 => return Some(Action::Char(8)),  // Ctrl+H
                36 => return Some(Action::Char(10)), // Ctrl+J
                37 => return Some(Action::Char(11)), // Ctrl+K
                38 => return Some(Action::Char(12)), // Ctrl+L (Clear screen)
                44 => return Some(Action::Char(26)), // Ctrl+Z
                45 => return Some(Action::Char(24)), // Ctrl+X
                46 => return Some(Action::Char(3)),  // Ctrl+C (Interrupt)
                47 => return Some(Action::Char(22)), // Ctrl+V
                48 => return Some(Action::Char(2)),  // Ctrl+B
                49 => return Some(Action::Char(14)), // Ctrl+N
                50 => return Some(Action::Char(13)), // Ctrl+M
                _ => {}
            }
        }

        match code {
            KEY_ESC => Some(Action::Escape),
            KEY_ENTER | 96 => Some(Action::Enter), // Enter & Keypad Enter
            14 => Some(Action::Backspace),           // Backspace
            15 => Some(Action::Char(b'\t')),          // Tab
            57 => Some(Action::Char(b' ')),            // Space
            111 => Some(Action::Delete),               // Delete

            // Number row (1-0)
            2 => Some(Action::Char(if shift { b'!' } else { b'1' })),
            3 => Some(Action::Char(if shift { b'@' } else { b'2' })),
            4 => Some(Action::Char(if shift { b'#' } else { b'3' })),
            5 => Some(Action::Char(if shift { b'$' } else { b'4' })),
            6 => Some(Action::Char(if shift { b'%' } else { b'5' })),
            7 => Some(Action::Char(if shift { b'^' } else { b'6' })),
            8 => Some(Action::Char(if shift { b'&' } else { b'7' })),
            9 => Some(Action::Char(if shift { b'*' } else { b'8' })),
            10 => Some(Action::Char(if shift { b'(' } else { b'9' })),
            11 => Some(Action::Char(if shift { b')' } else { b'0' })),

            // Symbols & Punctuation
            12 => Some(Action::Char(if shift { b'_' } else { b'-' })),
            13 => Some(Action::Char(if shift { b'+' } else { b'=' })),
            26 => Some(Action::Char(if shift { b'{' } else { b'[' })),
            27 => Some(Action::Char(if shift { b'}' } else { b']' })),
            39 => Some(Action::Char(if shift { b':' } else { b';' })),
            40 => Some(Action::Char(if shift { b'"' } else { b'\'' })),
            41 => Some(Action::Char(if shift { b'~' } else { b'`' })),
            43 => Some(Action::Char(if shift { b'|' } else { b'\\' })),
            51 => Some(Action::Char(if shift { b'<' } else { b',' })),
            52 => Some(Action::Char(if shift { b'>' } else { b'.' })),
            53 => Some(Action::Char(if shift { b'?' } else { b'/' })),

            // Letters QWERTY row
            16 => Some(Action::Char(if is_upper { b'Q' } else { b'q' })),
            17 => Some(Action::Char(if is_upper { b'W' } else { b'w' })),
            18 => Some(Action::Char(if is_upper { b'E' } else { b'e' })),
            19 => Some(Action::Char(if is_upper { b'R' } else { b'r' })),
            20 => Some(Action::Char(if is_upper { b'T' } else { b't' })),
            21 => Some(Action::Char(if is_upper { b'Y' } else { b'y' })),
            22 => Some(Action::Char(if is_upper { b'U' } else { b'u' })),
            23 => Some(Action::Char(if is_upper { b'I' } else { b'i' })),
            24 => Some(Action::Char(if is_upper { b'O' } else { b'o' })),
            25 => Some(Action::Char(if is_upper { b'P' } else { b'p' })),

            // Letters ASDF row
            30 => Some(Action::Char(if is_upper { b'A' } else { b'a' })),
            31 => Some(Action::Char(if is_upper { b'S' } else { b's' })),
            32 => Some(Action::Char(if is_upper { b'D' } else { b'd' })),
            33 => Some(Action::Char(if is_upper { b'F' } else { b'f' })),
            34 => Some(Action::Char(if is_upper { b'G' } else { b'g' })),
            35 => Some(Action::Char(if is_upper { b'H' } else { b'h' })),
            36 => Some(Action::Char(if is_upper { b'J' } else { b'j' })),
            37 => Some(Action::Char(if is_upper { b'K' } else { b'k' })),
            38 => Some(Action::Char(if is_upper { b'L' } else { b'l' })),

            // Letters ZXCV row
            44 => Some(Action::Char(if is_upper { b'Z' } else { b'z' })),
            45 => Some(Action::Char(if is_upper { b'X' } else { b'x' })),
            46 => Some(Action::Char(if is_upper { b'C' } else { b'c' })),
            47 => Some(Action::Char(if is_upper { b'V' } else { b'v' })),
            48 => Some(Action::Char(if is_upper { b'B' } else { b'b' })),
            49 => Some(Action::Char(if is_upper { b'N' } else { b'n' })),
            50 => Some(Action::Char(if is_upper { b'M' } else { b'm' })),

            // Numeric Keypad
            82 => Some(Action::Char(b'0')),
            79 => Some(Action::Char(b'1')),
            80 => Some(Action::Char(b'2')),
            81 => Some(Action::Char(b'3')),
            75 => Some(Action::Char(b'4')),
            76 => Some(Action::Char(b'5')),
            77 => Some(Action::Char(b'6')),
            71 => Some(Action::Char(b'7')),
            72 => Some(Action::Char(b'8')),
            73 => Some(Action::Char(b'9')),
            83 => Some(Action::Char(b'.')),
            55 => Some(Action::Char(b'*')),
            74 => Some(Action::Char(b'-')),
            78 => Some(Action::Char(b'+')),
            98 => Some(Action::Char(b'/')),

            _ => None,
        }
    }

    fn action_from_char(ch: u8) -> Option<Action> {
        match ch {
            27 => Some(Action::Exit),
            b => Some(Action::Char(b)),
        }
    }
}

impl Drop for InputManager {
    fn drop(&mut self) {
        // Show cursor and restore termios on all tty fds
        let show_cursor = b"\x1b[?25h";
        for &(fd, ref orig) in &self.orig_termios {
            unsafe {
                libc::write(fd, show_cursor.as_ptr() as *const libc::c_void, show_cursor.len());
                libc::tcsetattr(fd, libc::TCSANOW, orig);
            }
        }

        for &fd in &self.tty_fds {
            if fd != libc::STDIN_FILENO && fd != libc::STDOUT_FILENO && fd != libc::STDERR_FILENO {
                unsafe { libc::close(fd) };
            }
        }

        for &fd in &self.evdev_fds {
            unsafe { libc::close(fd) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_event_size() {
        assert_eq!(std::mem::size_of::<InputEvent>(), 24);
    }

    #[test]
    fn test_action_mapping() {
        assert_eq!(InputManager::action_from_keycode_state(KEY_ESC, false, false, false, false, false), Some(Action::Escape));
        assert_eq!(InputManager::action_from_keycode_state(KEY_Q, false, false, false, false, false), Some(Action::Char(b'q')));
        assert_eq!(InputManager::action_from_keycode_state(KEY_Q, true, false, false, false, false), Some(Action::Char(b'Q')));
        assert_eq!(InputManager::action_from_keycode_state(2, false, false, false, false, false), Some(Action::Char(b'1')));
        assert_eq!(InputManager::action_from_keycode_state(2, true, false, false, false, false), Some(Action::Char(b'!')));
        assert_eq!(InputManager::action_from_keycode_state(KEY_C, false, false, true, false, false), Some(Action::Char(3)));
        assert_eq!(InputManager::action_from_keycode_state(15, false, false, false, true, false), Some(Action::AltTab));
        assert_eq!(InputManager::action_from_keycode_state(62, false, false, false, true, false), Some(Action::AltF4));
        assert_eq!(InputManager::action_from_keycode_state(20, false, false, false, false, true), Some(Action::SuperT));
        assert_eq!(InputManager::action_from_char(27), Some(Action::Exit));
        assert_eq!(InputManager::action_from_char(b'q'), Some(Action::Char(b'q')));
    }
}
