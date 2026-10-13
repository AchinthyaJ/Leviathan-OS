use crate::color::Color;
use crate::graphics::Graphics;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};

pub const EV_KEY: u16 = 0x01;
pub const EV_REL: u16 = 0x02;
pub const EV_ABS: u16 = 0x03;

pub const REL_X: u16 = 0x00;
pub const REL_Y: u16 = 0x01;

pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;

pub const BTN_LEFT: u16 = 0x110;
pub const BTN_RIGHT: u16 = 0x111;
pub const BTN_MIDDLE: u16 = 0x112;
pub const BTN_TOUCH: u16 = 0x14a;

// Linux ioctl: EVIOCGABS(0) and EVIOCGABS(1)
const EVIOCGABS_X: libc::c_int = 0x80184540u32 as libc::c_int;
const EVIOCGABS_Y: libc::c_int = 0x80184541u32 as libc::c_int;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct EvdevEvent {
    time_sec: usize,
    time_usec: usize,
    type_: u16,
    code: u16,
    value: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct InputAbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

struct EvdevDev {
    fd: RawFd,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    has_abs: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub middle_pressed: bool,
    pub left_clicked: bool,
    pub right_clicked: bool,
    pub middle_clicked: bool,
    pub left_released: bool,
    pub right_released: bool,
    pub middle_released: bool,
    pub scroll_delta: i32,
}

pub struct Mouse {
    mice_fd: Option<RawFd>,
    evdev_devs: Vec<EvdevDev>,
    pub x: i32,
    pub y: i32,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub middle_pressed: bool,
    was_left_pressed: bool,
    was_right_pressed: bool,
    was_middle_pressed: bool,
    scroll_delta: i32,
    has_left_click: bool,
    has_right_click: bool,
    has_middle_click: bool,
    has_left_release: bool,
    has_right_release: bool,
    has_middle_release: bool,
    screen_width: i32,
    screen_height: i32,
    pub sensitivity: f32,
    pub prev_draw_x: i32,
    pub prev_draw_y: i32,
    saved_bg: Vec<u8>,
    has_saved_bg: bool,
    pub cursor_img: Option<crate::image::Image>,
    pub cursor_pressed_img: Option<crate::image::Image>,
}

impl Mouse {
    pub fn new(width: u32, height: u32) -> Self {
        let mut mice_fd = None;
        for path in &["/dev/input/mice", "/dev/input/mouse0"] {
            if let Ok(file) = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(path)
            {
                let raw = file.as_raw_fd();
                std::mem::forget(file);
                mice_fd = Some(raw);
                println!("leviathan-ui: Connected to mouse device {}", path);
                break;
            }
        }

        let mut evdev_devs = Vec::new();
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
                        let raw = file.as_raw_fd();
                        std::mem::forget(file);

                        // Check device name from /sys/class/input/<event>/device/name to ignore keyboards
                        let sysfs_name_path = format!("/sys/class/input/{}/device/name", name_str);
                        let dev_name = fs::read_to_string(&sysfs_name_path)
                            .unwrap_or_default()
                            .to_lowercase();

                        if dev_name.contains("keyboard") || dev_name.contains("keypad") ||
                           dev_name.contains("power button") || dev_name.contains("sleep button") ||
                           dev_name.contains("lid switch") || dev_name.contains("vmmouse") {
                            continue;
                        }

                        let mut abs_x = InputAbsInfo::default();
                        let mut abs_y = InputAbsInfo::default();
                        let mut has_abs = false;

                        unsafe {
                            if libc::ioctl(raw, EVIOCGABS_X as _, &mut abs_x) == 0
                                && libc::ioctl(raw, EVIOCGABS_Y as _, &mut abs_y) == 0
                                && abs_x.maximum > abs_x.minimum
                            {
                                has_abs = true;
                            }
                        }

                        evdev_devs.push(EvdevDev {
                            fd: raw,
                            min_x: if has_abs { abs_x.minimum } else { 0 },
                            max_x: if has_abs { abs_x.maximum } else { 32767 },
                            min_y: if has_abs { abs_y.minimum } else { 0 },
                            max_y: if has_abs { abs_y.maximum } else { 32767 },
                            has_abs,
                        });
                        println!(
                            "leviathan-ui: Mouse connected to evdev {} (name='{}', abs={})",
                            path.display(),
                            dev_name.trim(),
                            has_abs
                        );
                    }
                }
            }
        }

        let candidate_dirs = [
            "cinematic-scene-assets",
            "/usr/share/cinematic-scene-assets",
            "/cinematic-scene-assets",
            "../cinematic-scene-assets",
            "/home/inchara/My Death/Leviathan/cinematic-scene-assets",
        ];

        let mut cursor_img = None;
        let mut cursor_pressed_img = None;
        for dir in &candidate_dirs {
            let p_norm = std::path::Path::new(dir).join("cursor_paw.png");
            let p_press = std::path::Path::new(dir).join("cursor_paw_pressed.png");
            if p_norm.exists() {
                cursor_img = crate::image::Image::load_png(&p_norm).ok();
                cursor_pressed_img = crate::image::Image::load_png(&p_press).ok();
                if cursor_img.is_some() {
                    break;
                }
            }
            let p_orig = std::path::Path::new(dir).join("cursor.png");
            if p_orig.exists() {
                cursor_img = crate::image::Image::load_png(&p_orig).ok();
                if cursor_img.is_some() {
                    break;
                }
            }
        }

        Self {
            mice_fd,
            evdev_devs,
            x: (width / 2) as i32,
            y: (height / 2) as i32,
            left_pressed: false,
            right_pressed: false,
            middle_pressed: false,
            was_left_pressed: false,
            was_right_pressed: false,
            was_middle_pressed: false,
            scroll_delta: 0,
            has_left_click: false,
            has_right_click: false,
            has_middle_click: false,
            has_left_release: false,
            has_right_release: false,
            has_middle_release: false,
            screen_width: width as i32,
            screen_height: height as i32,
            sensitivity: 1.0,
            prev_draw_x: (width / 2) as i32,
            prev_draw_y: (height / 2) as i32,
            saved_bg: vec![0u8; 48 * 48 * 4],
            has_saved_bg: false,
            cursor_img,
            cursor_pressed_img,
        }
    }

    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.screen_width = width as i32;
        self.screen_height = height as i32;
        self.x = self.x.clamp(0, self.screen_width - 1);
        self.y = self.y.clamp(0, self.screen_height - 1);
    }

    pub fn update(&mut self) -> MouseState {
        let has_abs_dev = self.evdev_devs.iter().any(|d| d.has_abs);

        // 1. Process evdev devices (Supports absolute VirtIO/USB tablets, touchpads, and evdev mice)
        let mut ev_buf = [EvdevEvent::default(); 64];
        for dev in &self.evdev_devs {
            let n = unsafe {
                libc::read(
                    dev.fd,
                    ev_buf.as_mut_ptr() as *mut libc::c_void,
                    ev_buf.len() * std::mem::size_of::<EvdevEvent>(),
                )
            };
            if n > 0 {
                let count = n as usize / std::mem::size_of::<EvdevEvent>();
                for ev in &ev_buf[..count] {
                    match ev.type_ {
                        EV_ABS => {
                            if ev.code == ABS_X {
                                let span = (dev.max_x - dev.min_x).max(1);
                                self.x = ((ev.value - dev.min_x) as i64 * self.screen_width as i64 / span as i64) as i32;
                                self.x = self.x.clamp(0, self.screen_width - 1);
                            } else if ev.code == ABS_Y {
                                let span = (dev.max_y - dev.min_y).max(1);
                                self.y = ((ev.value - dev.min_y) as i64 * self.screen_height as i64 / span as i64) as i32;
                                self.y = self.y.clamp(0, self.screen_height - 1);
                            }
                        }
                        EV_REL => {
                            // If an absolute tablet device is active, avoid relative drift
                            if !has_abs_dev {
                                if ev.code == REL_X {
                                    self.x += ((ev.value as f32) * self.sensitivity).round() as i32;
                                } else if ev.code == REL_Y {
                                    self.y += ((ev.value as f32) * self.sensitivity).round() as i32;
                                }
                            }
                            if ev.code == 0x08 /* REL_WHEEL */ || ev.code == 0x0b /* REL_WHEEL_HI_RES */ {
                                self.scroll_delta += ev.value;
                            }
                        }
                        EV_KEY => {
                            if ev.code == BTN_LEFT || ev.code == BTN_TOUCH {
                                if ev.value != 0 {
                                    self.left_pressed = true;
                                    self.has_left_click = true;
                                } else {
                                    self.left_pressed = false;
                                    self.has_left_release = true;
                                }
                            } else if ev.code == BTN_RIGHT {
                                if ev.value != 0 {
                                    self.right_pressed = true;
                                    self.has_right_click = true;
                                } else {
                                    self.right_pressed = false;
                                    self.has_right_release = true;
                                }
                            } else if ev.code == BTN_MIDDLE {
                                if ev.value != 0 {
                                    self.middle_pressed = true;
                                    self.has_middle_click = true;
                                } else {
                                    self.middle_pressed = false;
                                    self.has_middle_release = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // 2. Process legacy PS/2 stream (/dev/input/mice) ONLY if no evdev devices exist
        if self.evdev_devs.is_empty() {
            let mut buf = [0u8; 96];
            if let Some(fd) = self.mice_fd {
                let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
                if n > 0 {
                    let chunks = n as usize / 3;
                    for i in 0..chunks {
                        let b0 = buf[i * 3];
                        let b1 = buf[i * 3 + 1];
                        let b2 = buf[i * 3 + 2];

                        let btn_left = (b0 & 0x01) != 0;
                        let btn_right = (b0 & 0x02) != 0;
                        let btn_middle = (b0 & 0x04) != 0;

                        let dx = b1 as i8 as i32;
                        let dy = b2 as i8 as i32;

                        self.x += ((dx as f32) * self.sensitivity).round() as i32;
                        self.y -= ((dy as f32) * self.sensitivity).round() as i32;

                        if btn_left && !self.left_pressed {
                            self.has_left_click = true;
                        } else if !btn_left && self.left_pressed {
                            self.has_left_release = true;
                        }
                        self.left_pressed = btn_left;

                        if btn_right && !self.right_pressed {
                            self.has_right_click = true;
                        } else if !btn_right && self.right_pressed {
                            self.has_right_release = true;
                        }
                        self.right_pressed = btn_right;

                        if btn_middle && !self.middle_pressed {
                            self.has_middle_click = true;
                        } else if !btn_middle && self.middle_pressed {
                            self.has_middle_release = true;
                        }
                        self.middle_pressed = btn_middle;
                    }
                }
            }
        }

        self.x = self.x.clamp(0, self.screen_width - 1);
        self.y = self.y.clamp(0, self.screen_height - 1);

        let left_clicked = self.has_left_click || (self.left_pressed && !self.was_left_pressed);
        let right_clicked = self.has_right_click || (self.right_pressed && !self.was_right_pressed);
        let middle_clicked = self.has_middle_click || (self.middle_pressed && !self.was_middle_pressed);
        let left_released = self.has_left_release || (!self.left_pressed && self.was_left_pressed);
        let right_released = self.has_right_release || (!self.right_pressed && self.was_right_pressed);
        let middle_released = self.has_middle_release || (!self.middle_pressed && self.was_middle_pressed);

        self.has_left_click = false;
        self.has_right_click = false;
        self.has_middle_click = false;
        self.has_left_release = false;
        self.has_right_release = false;
        self.has_middle_release = false;

        let scroll_delta = self.scroll_delta;
        self.scroll_delta = 0;

        self.was_left_pressed = self.left_pressed;
        self.was_right_pressed = self.right_pressed;
        self.was_middle_pressed = self.middle_pressed;

        MouseState {
            x: self.x,
            y: self.y,
            left_pressed: self.left_pressed,
            right_pressed: self.right_pressed,
            middle_pressed: self.middle_pressed,
            left_clicked,
            right_clicked,
            middle_clicked,
            left_released,
            right_released,
            middle_released,
            scroll_delta,
        }
    }

    /// Inject mouse state from Wayland pointer events
    pub fn inject_wayland_event(
        &mut self,
        x: i32,
        y: i32,
        left_down: bool,
        right_down: bool,
        left_clicked: bool,
        left_released: bool,
        right_clicked: bool,
        scroll_delta: i32,
    ) {
        self.x = x.clamp(0, (self.screen_width - 1).max(0));
        self.y = y.clamp(0, (self.screen_height - 1).max(0));
        self.left_pressed = left_down;
        self.right_pressed = right_down;
        if left_clicked {
            self.has_left_click = true;
        }
        if left_released {
            self.has_left_release = true;
        }
        if right_clicked {
            self.has_right_click = true;
        }
        self.scroll_delta += scroll_delta;
    }

    /// Restores the previously saved background rectangle before cursor moves
    pub fn restore_cursor_background(&mut self, gfx: &mut Graphics) {
        if !self.has_saved_bg {
            return;
        }
        let bpp = gfx.bytes_per_pixel() as usize;
        let pitch = gfx.pitch() as usize;
        let buf = gfx.backbuffer_mut();

        for row in 0..16 {
            let py = self.prev_draw_y + row as i32;
            if py < 0 || py >= self.screen_height {
                continue;
            }
            let screen_row_offset = (py as usize) * pitch;
            let src_row_offset = row * 16 * bpp;

            for col in 0..16 {
                let px = self.prev_draw_x + col as i32;
                if px < 0 || px >= self.screen_width {
                    continue;
                }
                let dst_off = screen_row_offset + (px as usize) * bpp;
                let src_off = src_row_offset + col * bpp;
                if dst_off + bpp <= buf.len() && src_off + bpp <= self.saved_bg.len() {
                    buf[dst_off..dst_off + bpp].copy_from_slice(&self.saved_bg[src_off..src_off + bpp]);
                }
            }
        }
        self.has_saved_bg = false;
    }

    /// Draws the cursor sprite onto the framebuffer
    pub fn draw_cursor(&mut self, gfx: &mut Graphics) {
        let mx = self.x;
        let my = self.y;

        // 1. If custom paw cursor image is loaded, draw it directly
        let custom_cursor = if self.left_pressed {
            self.cursor_pressed_img.as_ref().or(self.cursor_img.as_ref())
        } else {
            self.cursor_img.as_ref()
        };

        if let Some(img) = custom_cursor {
            // Hotspot at top tip of paw
            let hot_x = (img.width as i32 * 18) / 36;
            let hot_y = (img.height as i32 * 5) / 36;
            let draw_x = mx - hot_x;
            let draw_y = my - hot_y;

            let pitch = gfx.pitch();
            let width = gfx.width();
            let height = gfx.height();
            let buf = gfx.backbuffer_mut();

            img.draw_to_buffer(buf, width, height, pitch, draw_x, draw_y, 1.0);

            self.prev_draw_x = mx;
            self.prev_draw_y = my;
            return;
        }

        const CURSOR_PATTERN: [&str; 15] = [
            "1...............",
            "11..............",
            "121.............",
            "1221............",
            "12221...........",
            "122221..........",
            "1222221.........",
            "12222221........",
            "122222221.......",
            "122221111.......",
            "122121..........",
            "121..121........",
            "11....121.......",
            "1......11.......",
            "................",
        ];

        let col_shadow = Color::new(5, 12, 20);
        let col_fill = if self.left_pressed {
            Color::new(0, 255, 220)
        } else {
            Color::new(240, 250, 255)
        };

        // Direct backbuffer access for maximum speed
        let bpp = gfx.bytes_per_pixel();
        let pitch = gfx.pitch();
        let buf = gfx.backbuffer_mut();
        let buf_len = buf.len();

        for (row_idx, row_str) in CURSOR_PATTERN.iter().enumerate() {
            let py = my + row_idx as i32;
            if py < 0 || py >= self.screen_height {
                continue;
            }

            let row_offset = (py as usize) * (pitch as usize);

            for (col_idx, ch) in row_str.chars().enumerate() {
                let px = mx + col_idx as i32;
                if px < 0 || px >= self.screen_width {
                    continue;
                }

                let pixel_offset = row_offset + (px as usize) * (bpp as usize);

                if pixel_offset + 3 < buf_len {
                    let color = if ch == '1' {
                        col_shadow
                    } else if ch == '2' {
                        col_fill
                    } else {
                        continue;
                    };

                    // Direct BGRA write (assuming 32bpp framebuffer)
                    buf[pixel_offset] = color.b;
                    buf[pixel_offset + 1] = color.g;
                    buf[pixel_offset + 2] = color.r;
                    // buf[pixel_offset + 3] = 255; // alpha - skip for speed
                }
            }
        }

        self.prev_draw_x = mx;
        self.prev_draw_y = my;
    }

    /// Fast cursor update that only flushes the 16x16 cursor region directly to hardware
    pub fn update_cursor_fast_swap(&mut self, gfx: &mut Graphics) {
        let old_x = self.prev_draw_x;
        let old_y = self.prev_draw_y;
        self.restore_cursor_background(gfx);
        self.draw_cursor(gfx);
        gfx.swap_rect(old_x, old_y, 16, 16);
        gfx.swap_rect(self.x, self.y, 16, 16);
    }
}

impl Drop for Mouse {
    fn drop(&mut self) {
        if let Some(fd) = self.mice_fd {
            unsafe { libc::close(fd) };
        }
        for dev in &self.evdev_devs {
            unsafe { libc::close(dev.fd) };
        }
    }
}
