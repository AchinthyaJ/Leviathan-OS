use crate::color::Color;
use crate::font::get_glyph;

use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;

const FBIOGET_FSCREENINFO: libc::c_int = 0x4602;
const FBIOGET_VSCREENINFO: libc::c_int = 0x4600;
const KDSETMODE: libc::c_int = 0x4B3A;
const KD_TEXT: libc::c_int = 0;
const KD_GRAPHICS: libc::c_int = 1;

#[repr(C)]
struct FbBitfield {
    offset: u32,
    length: u32,
    msb_right: u32,
}

#[repr(C)]
struct FbVarScreeninfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
    red: FbBitfield,
    green: FbBitfield,
    blue: FbBitfield,
    transp: FbBitfield,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

#[repr(C)]
struct FbFixScreeninfo {
    id: [u8; 16],
    smem_start: libc::c_ulong,
    smem_len: u32,
    fb_type: u32,
    type_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: libc::c_ulong,
    mmio_len: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 2],
}

#[allow(dead_code)]
pub struct Graphics {
    fb_file: File,
    mmap_ptr: *mut u8,
    smem_len: usize,
    width: u32,
    height: u32,
    pitch: u32,
    bpp: u32,
    bytes_per_pixel: u32,
    red: FbBitfield,
    green: FbBitfield,
    blue: FbBitfield,
    transp: FbBitfield,
    backbuffer: Vec<u8>,
    tty_fd: Option<RawFd>,
}

#[allow(dead_code)]
impl Graphics {
    pub fn new() -> Result<Self, String> {
        let fb_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/fb0")
            .map_err(|e| format!("Failed to open /dev/fb0: {}", e))?;

        let raw_fd = fb_file.as_raw_fd();

        let mut finfo: FbFixScreeninfo = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(raw_fd, FBIOGET_FSCREENINFO, &mut finfo) } < 0 {
            return Err("ioctl FBIOGET_FSCREENINFO failed".to_string());
        }

        let mut vinfo: FbVarScreeninfo = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(raw_fd, FBIOGET_VSCREENINFO, &mut vinfo) } < 0 {
            return Err("ioctl FBIOGET_VSCREENINFO failed".to_string());
        }

        let bpp = vinfo.bits_per_pixel;
        let bytes_per_pixel = bpp / 8;
        if bytes_per_pixel != 4 && bytes_per_pixel != 2 && bytes_per_pixel != 3 {
            return Err(format!("Unsupported bits per pixel: {}", bpp));
        }

        let smem_len = finfo.smem_len as usize;
        let mmap_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                smem_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                raw_fd,
                0,
            )
        };

        if mmap_ptr == libc::MAP_FAILED {
            return Err("mmap /dev/fb0 failed".to_string());
        }

        let backbuffer = vec![0u8; smem_len];

        // Silence Linux Virtual Terminal cursor & text output
        let mut tty_fd = None;
        for path in &["/dev/tty0", "/dev/tty1", "/dev/console"] {
            let fd = unsafe { libc::open(path.as_ptr() as *const libc::c_char, libc::O_RDWR) };
            if fd >= 0 {
                unsafe { libc::ioctl(fd, KDSETMODE, KD_GRAPHICS) };
                tty_fd = Some(fd);
                break;
            }
        }

        Ok(Self {
            fb_file,
            mmap_ptr: mmap_ptr as *mut u8,
            smem_len,
            width: vinfo.xres,
            height: vinfo.yres,
            pitch: finfo.line_length,
            bpp,
            bytes_per_pixel,
            red: vinfo.red,
            green: vinfo.green,
            blue: vinfo.blue,
            transp: vinfo.transp,
            backbuffer,
            tty_fd,
        })
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    pub fn pitch(&self) -> u32 { self.pitch }
    pub fn bpp(&self) -> u32 { self.bpp }
    pub fn buffer_size(&self) -> usize { self.smem_len }

    #[inline(always)]
    pub fn pack_color(&self, c: Color) -> u32 {
        let r = ((c.r >> (8 - self.red.length)) as u32) << self.red.offset;
        let g = ((c.g >> (8 - self.green.length)) as u32) << self.green.offset;
        let b = ((c.b >> (8 - self.blue.length)) as u32) << self.blue.offset;
        let a = if self.transp.length > 0 {
            ((c.a >> (8 - self.transp.length)) as u32) << self.transp.offset
        } else {
            0
        };
        r | g | b | a
    }

    #[inline(always)]
    pub fn put_pixel(&mut self, x: i32, y: i32, c: Color) {
        if x < 0 || (x as u32) >= self.width || y < 0 || (y as u32) >= self.height {
            return;
        }

        let offset = (y as usize) * (self.pitch as usize) + (x as usize) * (self.bytes_per_pixel as usize);
        if offset + (self.bytes_per_pixel as usize) > self.smem_len {
            return;
        }

        let pval = self.pack_color(c);
        let bpp = self.bytes_per_pixel;

        if bpp == 4 {
            let slice = &mut self.backbuffer[offset..offset + 4];
            slice.copy_from_slice(&pval.to_ne_bytes());
        } else if bpp == 2 {
            let slice = &mut self.backbuffer[offset..offset + 2];
            slice.copy_from_slice(&(pval as u16).to_ne_bytes());
        } else if bpp == 3 {
            self.backbuffer[offset] = c.b;
            self.backbuffer[offset + 1] = c.g;
            self.backbuffer[offset + 2] = c.r;
        }
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> Color {
        if x < 0 || (x as u32) >= self.width || y < 0 || (y as u32) >= self.height {
            return Color::BLACK;
        }

        let offset = (y as usize) * (self.pitch as usize) + (x as usize) * (self.bytes_per_pixel as usize);
        if offset + (self.bytes_per_pixel as usize) > self.smem_len {
            return Color::BLACK;
        }

        if self.bytes_per_pixel == 4 {
            let bytes: [u8; 4] = self.backbuffer[offset..offset + 4].try_into().unwrap_or([0; 4]);
            let pval = u32::from_ne_bytes(bytes);
            let r = (((pval >> self.red.offset) & ((1 << self.red.length) - 1)) << (8 - self.red.length)) as u8;
            let g = (((pval >> self.green.offset) & ((1 << self.green.length) - 1)) << (8 - self.green.length)) as u8;
            let b = (((pval >> self.blue.offset) & ((1 << self.blue.length) - 1)) << (8 - self.blue.length)) as u8;
            Color::rgba(r, g, b, 255)
        } else {
            Color::BLACK
        }
    }

    pub fn clear(&mut self, c: Color) {
        let pval = self.pack_color(c);
        let bytes = pval.to_ne_bytes();
        let bpp = self.bytes_per_pixel as usize;

        if bpp == 4 {
            for chunk in self.backbuffer.chunks_exact_mut(4) {
                chunk.copy_from_slice(&bytes);
            }
        } else {
            for y in 0..self.height {
                for x in 0..self.width {
                    self.put_pixel(x as i32, y as i32, c);
                }
            }
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        if w <= 0 || h <= 0 {
            return;
        }

        let x0 = x.clamp(0, self.width as i32);
        let y0 = y.clamp(0, self.height as i32);
        let x1 = (x + w).clamp(0, self.width as i32);
        let y1 = (y + h).clamp(0, self.height as i32);

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let pval = self.pack_color(c);
        let bytes = pval.to_ne_bytes();

        for cy in y0..y1 {
            let row_offset = (cy as usize) * (self.pitch as usize);
            for cx in x0..x1 {
                let offset = row_offset + (cx as usize) * (self.bytes_per_pixel as usize);
                if self.bytes_per_pixel == 4 {
                    self.backbuffer[offset..offset + 4].copy_from_slice(&bytes);
                } else {
                    self.put_pixel(cx, cy, c);
                }
            }
        }
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        if w <= 0 || h <= 0 {
            return;
        }
        self.draw_line(x, y, x + w - 1, y, c);
        self.draw_line(x, y + h - 1, x + w - 1, y + h - 1, c);
        self.draw_line(x, y, x, y + h - 1, c);
        self.draw_line(x + w - 1, y, x + w - 1, y + h - 1, c);
    }

    pub fn draw_line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, c: Color) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.put_pixel(x0, y0, c);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: i32, c: Color) {
        if radius < 0 {
            return;
        }
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.put_pixel(cx + x, cy + y, c);
            self.put_pixel(cx + y, cy + x, c);
            self.put_pixel(cx - y, cy + x, c);
            self.put_pixel(cx - x, cy + y, c);
            self.put_pixel(cx - x, cy - y, c);
            self.put_pixel(cx - y, cy - x, c);
            self.put_pixel(cx + y, cy - x, c);
            self.put_pixel(cx + x, cy - y, c);

            if err <= 0 {
                y += 1;
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, c: Color) {
        if radius < 0 {
            return;
        }
        let r2 = radius * radius;
        for dy in -radius..=radius {
            let mut dx_max = 0;
            while (dx_max + 1) * (dx_max + 1) + dy * dy <= r2 {
                dx_max += 1;
            }
            self.draw_line(cx - dx_max, cy + dy, cx + dx_max, cy + dy, c);
        }
    }

    pub fn draw_char(&mut self, x: i32, y: i32, ch: char, fg: Color, bg: Option<Color>) {
        let glyph = get_glyph(ch);
        for row in 0..8 {
            let byte = glyph[row];
            for col in 0..8 {
                if (byte & (0x80 >> col)) != 0 {
                    self.put_pixel(x + col as i32, y + row as i32, fg);
                } else if let Some(bg_col) = bg {
                    if bg_col.a > 0 {
                        self.put_pixel(x + col as i32, y + row as i32, bg_col);
                    }
                }
            }
        }
    }

    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, fg: Color, bg: Option<Color>) {
        let mut cur_x = x;
        let mut cur_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                cur_x = x;
                cur_y += 10;
            } else if ch == '\t' {
                cur_x += 32;
            } else {
                self.draw_char(cur_x, cur_y, ch, fg, bg);
                cur_x += 8;
            }
        }
    }

    /// Fast-blits the backbuffer to physical framebuffer in single burst
    pub fn swap_buffers(&mut self) {
        unsafe {
            ptr::copy_nonoverlapping(self.backbuffer.as_ptr(), self.mmap_ptr, self.smem_len);
        }
    }

    pub fn copy_to_backbuffer(&mut self, src: &[u8]) {
        let len = src.len().min(self.smem_len);
        self.backbuffer[..len].copy_from_slice(&src[..len]);
    }

    pub fn get_backbuffer(&self) -> &[u8] {
        &self.backbuffer
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        // Restore VT text mode
        if let Some(fd) = self.tty_fd {
            unsafe {
                libc::ioctl(fd, KDSETMODE, KD_TEXT);
                libc::close(fd);
            }
        }

        // Unmap framebuffer memory
        if !self.mmap_ptr.is_null() && self.mmap_ptr != libc::MAP_FAILED as *mut u8 {
            unsafe {
                libc::munmap(self.mmap_ptr as *mut libc::c_void, self.smem_len);
            }
        }
    }
}
