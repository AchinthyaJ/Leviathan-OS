use crate::color::Color;
use crate::graphics::Graphics;

/// Identifier for any icon in the Leviathan icon system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconId {
    // Window Controls
    Close,
    Minimize,
    Maximize,
    Restore,

    // Navigation & Chevrons
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronDown,
    Back,
    Forward,
    Up,
    Down,

    // Core System & Dock Apps
    AllApps,
    Terminal,
    Packages,
    Settings,
    Files,
    File,
    Folder,
    FolderPlus,
    Abyss3D,
    LayerEditor,

    // Common Actions & Tools
    Search,
    Refresh,
    Home,
    HardDrive,
    Trash,
    Edit,
    Copy,
    Cut,
    Paste,
    Download,
    Plus,
    Minus,
    Check,

    // System Status & Hardware
    Palette,
    Monitor,
    Wifi,
    Cpu,
    Mouse,
    Info,
    Clock,
    Globe,
    Power,
    Lock,
    Unlock,
}

impl IconId {
    /// Convert an icon name or alias into an `IconId`.
    pub fn from_name(name: &str) -> Option<Self> {
        let n = name.trim().to_lowercase();
        match n.as_str() {
            "close" | "cancel" | "x" => Some(Self::Close),
            "minimize" | "min" => Some(Self::Minimize),
            "maximize" | "max" => Some(Self::Maximize),
            "restore" | "unmaximize" => Some(Self::Restore),

            "chevron-left" | "chevron_left" => Some(Self::ChevronLeft),
            "chevron-right" | "chevron_right" => Some(Self::ChevronRight),
            "chevron-up" | "chevron_up" => Some(Self::ChevronUp),
            "chevron-down" | "chevron_down" => Some(Self::ChevronDown),
            "back" | "arrow-left" | "left" | "<" => Some(Self::Back),
            "forward" | "arrow-right" | "right" | ">" => Some(Self::Forward),
            "up" | "arrow-up" | "^" => Some(Self::Up),
            "down" | "arrow-down" | "v" => Some(Self::Down),

            "allapps" | "all-apps" | "grid" | "apps" | "launcher" => Some(Self::AllApps),
            "terminal" | "console" | "shell" | "bash" | "xterm" => Some(Self::Terminal),
            "packages" | "package" | "box" | "pkg" | "software" | "apt" => Some(Self::Packages),
            "settings" | "preferences" | "config" | "sliders" => Some(Self::Settings),
            "files" | "filemanager" | "explorer" | "nautilus" => Some(Self::Files),
            "file" | "document" | "doc" | "text" => Some(Self::File),
            "folder" | "directory" | "dir" => Some(Self::Folder),
            "folder-plus" | "folder_plus" | "new-folder" => Some(Self::FolderPlus),
            "abyss3d" | "3d" | "cube" | "viewport" => Some(Self::Abyss3D),
            "layereditor" | "layer-editor" | "layers" => Some(Self::LayerEditor),

            "search" | "find" | "magnifier" => Some(Self::Search),
            "refresh" | "reload" | "rotate" | "update" => Some(Self::Refresh),
            "home" | "~" => Some(Self::Home),
            "harddrive" | "drive" | "disk" | "root" | "/" => Some(Self::HardDrive),
            "trash" | "delete" | "remove" | "rm" => Some(Self::Trash),
            "edit" | "rename" | "pencil" => Some(Self::Edit),
            "copy" | "duplicate" => Some(Self::Copy),
            "cut" | "scissors" => Some(Self::Cut),
            "paste" | "clipboard" => Some(Self::Paste),
            "download" | "install" => Some(Self::Download),
            "plus" | "add" | "+" => Some(Self::Plus),
            "minus" | "-" => Some(Self::Minus),
            "check" | "checkmark" | "ok" => Some(Self::Check),

            "palette" | "appearance" | "theme" => Some(Self::Palette),
            "monitor" | "display" | "screen" => Some(Self::Monitor),
            "wifi" | "network" | "net" | "online" | "wireless" => Some(Self::Wifi),
            "cpu" | "chip" | "memory" | "ram" | "processor" => Some(Self::Cpu),
            "mouse" | "input" | "cursor" | "pointer" => Some(Self::Mouse),
            "info" | "about" | "help" => Some(Self::Info),
            "clock" | "time" => Some(Self::Clock),
            "globe" | "browser" | "internet" | "web" | "firefox" => Some(Self::Globe),
            "power" | "shutdown" | "reboot" => Some(Self::Power),
            "lock" => Some(Self::Lock),
            "unlock" => Some(Self::Unlock),

            _ => None,
        }
    }

    /// Canonical string identifier for this icon.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Minimize => "minimize",
            Self::Maximize => "maximize",
            Self::Restore => "restore",
            Self::ChevronLeft => "chevron-left",
            Self::ChevronRight => "chevron-right",
            Self::ChevronUp => "chevron-up",
            Self::ChevronDown => "chevron-down",
            Self::Back => "back",
            Self::Forward => "forward",
            Self::Up => "up",
            Self::Down => "down",
            Self::AllApps => "all-apps",
            Self::Terminal => "terminal",
            Self::Packages => "packages",
            Self::Settings => "settings",
            Self::Files => "files",
            Self::File => "file",
            Self::Folder => "folder",
            Self::FolderPlus => "folder-plus",
            Self::Abyss3D => "abyss-3d",
            Self::LayerEditor => "layer-editor",
            Self::Search => "search",
            Self::Refresh => "refresh",
            Self::Home => "home",
            Self::HardDrive => "hard-drive",
            Self::Trash => "trash",
            Self::Edit => "edit",
            Self::Copy => "copy",
            Self::Cut => "cut",
            Self::Paste => "paste",
            Self::Download => "download",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Check => "check",
            Self::Palette => "palette",
            Self::Monitor => "monitor",
            Self::Wifi => "wifi",
            Self::Cpu => "cpu",
            Self::Mouse => "mouse",
            Self::Info => "info",
            Self::Clock => "clock",
            Self::Globe => "globe",
            Self::Power => "power",
            Self::Lock => "lock",
            Self::Unlock => "unlock",
        }
    }
}

/// Helper function to retrieve an icon by string identifier.
pub fn get_icon(name: &str) -> Option<IconId> {
    IconId::from_name(name)
}

// ---------------------------------------------------------------------------
// Precision Vector Drawing Helpers for 24x24 Icon Grid
// ---------------------------------------------------------------------------

struct IconRenderer<'a> {
    gfx: &'a mut Graphics,
    ox: i32,
    oy: i32,
    size: i32,
    stroke_w: i32,
    color: Color,
}

impl<'a> IconRenderer<'a> {
    fn new(gfx: &'a mut Graphics, x: i32, y: i32, size: i32, color: Color) -> Self {
        let stroke_w = if size <= 16 {
            1
        } else if size <= 28 {
            2
        } else {
            (size / 12).max(2)
        };

        Self {
            gfx,
            ox: x,
            oy: y,
            size,
            stroke_w,
            color,
        }
    }

    #[inline(always)]
    fn sx(&self, vx: f32) -> i32 {
        self.ox + ((vx * self.size as f32) / 24.0).round() as i32
    }

    #[inline(always)]
    fn sy(&self, vy: f32) -> i32 {
        self.oy + ((vy * self.size as f32) / 24.0).round() as i32
    }

    #[inline(always)]
    fn sdim(&self, v: f32) -> i32 {
        ((v * self.size as f32) / 24.0).round().max(1.0) as i32
    }

    fn draw_line_raw(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        if self.stroke_w <= 1 {
            self.gfx.draw_line(x0, y0, x1, y1, self.color);
        } else {
            // Draw with stroke width
            let half = self.stroke_w / 2;
            let rem = self.stroke_w - half;
            let dx = (x1 - x0).abs();
            let dy = (y1 - y0).abs();

            if dx >= dy {
                for offset in -half..rem {
                    self.gfx.draw_line(x0, y0 + offset, x1, y1 + offset, self.color);
                }
            } else {
                for offset in -half..rem {
                    self.gfx.draw_line(x0 + offset, y0, x1 + offset, y1, self.color);
                }
            }
        }
    }

    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.draw_line_raw(self.sx(x0), self.sy(y0), self.sx(x1), self.sy(y1));
    }

    fn rect_outline(&mut self, x: f32, y: f32, w: f32, h: f32, radius: i32) {
        let rx = self.sx(x);
        let ry = self.sy(y);
        let rw = self.sdim(w);
        let rh = self.sdim(h);
        let rad = if radius > 0 { self.sdim(radius as f32).min(rw / 2).min(rh / 2) } else { 0 };

        if rad > 1 {
            self.gfx.draw_squircle(rx, ry, rw, rh, rad, self.color);
            if self.stroke_w > 1 && rw > 4 && rh > 4 {
                self.gfx.draw_squircle(rx + 1, ry + 1, rw - 2, rh - 2, (rad - 1).max(1), self.color);
            }
        } else {
            for i in 0..self.stroke_w {
                if rw > i * 2 && rh > i * 2 {
                    self.gfx.draw_rect(rx + i, ry + i, rw - i * 2, rh - i * 2, self.color);
                }
            }
        }
    }

    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: i32) {
        let rx = self.sx(x);
        let ry = self.sy(y);
        let rw = self.sdim(w);
        let rh = self.sdim(h);
        let rad = if radius > 0 { self.sdim(radius as f32).min(rw / 2).min(rh / 2) } else { 0 };

        if rad > 1 {
            self.gfx.fill_squircle(rx, ry, rw, rh, rad, self.color);
        } else {
            self.gfx.fill_rect(rx, ry, rw, rh, self.color);
        }
    }

    fn circle_outline(&mut self, cx: f32, cy: f32, r: f32) {
        let rcx = self.sx(cx);
        let rcy = self.sy(cy);
        let rr = self.sdim(r);

        for i in 0..self.stroke_w {
            if rr - i >= 0 {
                self.gfx.draw_circle(rcx, rcy, rr - i, self.color);
            }
        }
    }

    fn fill_circle(&mut self, cx: f32, cy: f32, r: f32) {
        let rcx = self.sx(cx);
        let rcy = self.sy(cy);
        let rr = self.sdim(r);
        self.gfx.fill_circle(rcx, rcy, rr, self.color);
    }

    fn arc(&mut self, cx: f32, cy: f32, r: f32, start_deg: f32, end_deg: f32) {
        let rcx = self.sx(cx);
        let rcy = self.sy(cy);
        let rr = self.sdim(r) as f32;
        let step_deg = (360.0 / (rr * 6.28).max(12.0)).min(15.0);

        let mut a = start_deg;
        let mut prev_pt: Option<(i32, i32)> = None;

        while a <= end_deg {
            let rad = a.to_radians();
            let px = rcx + (rr * rad.cos()).round() as i32;
            let py = rcy + (rr * rad.sin()).round() as i32;

            if let Some((px0, py0)) = prev_pt {
                self.draw_line_raw(px0, py0, px, py);
            }
            prev_pt = Some((px, py));
            a += step_deg;
        }
    }
}

/// Draw an icon cleanly at (x, y) with the specified bounding square size and color.
pub fn draw_icon(gfx: &mut Graphics, icon: IconId, x: i32, y: i32, size: i32, color: Color) {
    let mut r = IconRenderer::new(gfx, x, y, size, color);

    match icon {
        // Window Controls
        IconId::Close => {
            // Clean, balanced 45-degree cross
            r.line(6.0, 6.0, 18.0, 18.0);
            r.line(18.0, 6.0, 6.0, 18.0);
        }
        IconId::Minimize => {
            // Crisp horizontal bar
            r.line(5.0, 12.0, 19.0, 12.0);
        }
        IconId::Maximize => {
            // Clean square frame with subtle rounded corners
            r.rect_outline(5.0, 5.0, 14.0, 14.0, 2);
        }
        IconId::Restore => {
            // Overlapping window frames
            r.rect_outline(8.0, 4.0, 12.0, 12.0, 2);
            r.rect_outline(4.0, 8.0, 12.0, 12.0, 2);
        }

        // Navigation & Chevrons
        IconId::ChevronLeft | IconId::Back => {
            r.line(15.0, 5.0, 8.0, 12.0);
            r.line(8.0, 12.0, 15.0, 19.0);
        }
        IconId::ChevronRight | IconId::Forward => {
            r.line(9.0, 5.0, 16.0, 12.0);
            r.line(16.0, 12.0, 9.0, 19.0);
        }
        IconId::ChevronUp | IconId::Up => {
            r.line(5.0, 15.0, 12.0, 8.0);
            r.line(12.0, 8.0, 19.0, 15.0);
        }
        IconId::ChevronDown | IconId::Down => {
            r.line(5.0, 9.0, 12.0, 16.0);
            r.line(12.0, 16.0, 19.0, 9.0);
        }

        // Core Apps & System
        IconId::AllApps => {
            // Balanced 3x3 dot grid
            let dot_r = 1.6;
            for row in 0..3 {
                for col in 0..3 {
                    let cx = 6.0 + col as f32 * 6.0;
                    let cy = 6.0 + row as f32 * 6.0;
                    r.fill_circle(cx, cy, dot_r);
                }
            }
        }
        IconId::Terminal => {
            // Refined terminal prompt `>` + cursor `_`
            r.line(5.0, 7.0, 11.0, 12.0);
            r.line(11.0, 12.0, 5.0, 17.0);
            r.line(13.0, 17.0, 20.0, 17.0);
        }
        IconId::Packages => {
            // Minimalist isometric box with center fold
            r.line(12.0, 3.0, 21.0, 7.5);
            r.line(21.0, 7.5, 12.0, 12.0);
            r.line(12.0, 12.0, 3.0, 7.5);
            r.line(3.0, 7.5, 12.0, 3.0);

            r.line(3.0, 7.5, 3.0, 16.5);
            r.line(12.0, 12.0, 12.0, 21.0);
            r.line(21.0, 7.5, 21.0, 16.5);

            r.line(3.0, 16.5, 12.0, 21.0);
            r.line(12.0, 21.0, 21.0, 16.5);

            r.line(12.0, 3.0, 12.0, 12.0);
        }
        IconId::Settings => {
            // Refined modern sliders/sliders symbol or cog
            r.line(4.0, 7.0, 20.0, 7.0);
            r.line(4.0, 17.0, 20.0, 17.0);
            r.fill_rect(7.0, 4.5, 4.0, 5.0, 1);
            r.fill_rect(13.0, 14.5, 4.0, 5.0, 1);
        }
        IconId::Files | IconId::Folder => {
            // Clean tabbed folder outline
            r.line(3.0, 7.0, 3.0, 19.0);
            r.line(3.0, 19.0, 21.0, 19.0);
            r.line(21.0, 19.0, 21.0, 9.0);
            r.line(21.0, 9.0, 11.5, 9.0);
            r.line(11.5, 9.0, 9.5, 6.0);
            r.line(9.5, 6.0, 4.0, 6.0);
            r.line(4.0, 6.0, 3.0, 7.0);
        }
        IconId::FolderPlus => {
            // Folder with centered plus
            r.line(3.0, 7.0, 3.0, 19.0);
            r.line(3.0, 19.0, 21.0, 19.0);
            r.line(21.0, 19.0, 21.0, 9.0);
            r.line(21.0, 9.0, 11.5, 9.0);
            r.line(11.5, 9.0, 9.5, 6.0);
            r.line(9.5, 6.0, 4.0, 6.0);
            r.line(4.0, 6.0, 3.0, 7.0);

            r.line(12.0, 11.5, 12.0, 16.5);
            r.line(9.5, 14.0, 14.5, 14.0);
        }
        IconId::File => {
            // Clean document sheet with top-right corner fold
            r.line(5.0, 4.0, 14.0, 4.0);
            r.line(14.0, 4.0, 19.0, 9.0);
            r.line(19.0, 9.0, 19.0, 20.0);
            r.line(19.0, 20.0, 5.0, 20.0);
            r.line(5.0, 20.0, 5.0, 4.0);

            r.line(14.0, 4.0, 14.0, 9.0);
            r.line(14.0, 9.0, 19.0, 9.0);
        }
        IconId::Abyss3D => {
            // Wireframe 3D Cube
            r.rect_outline(4.0, 7.0, 11.0, 11.0, 1);
            r.line(4.0, 7.0, 9.0, 4.0);
            r.line(15.0, 7.0, 20.0, 4.0);
            r.line(15.0, 18.0, 20.0, 15.0);
            r.line(9.0, 4.0, 20.0, 4.0);
            r.line(20.0, 4.0, 20.0, 15.0);
        }
        IconId::LayerEditor => {
            // Stacked layers / planes
            r.line(12.0, 3.0, 21.0, 7.5);
            r.line(21.0, 7.5, 12.0, 12.0);
            r.line(12.0, 12.0, 3.0, 7.5);
            r.line(3.0, 7.5, 12.0, 3.0);

            r.line(3.0, 12.0, 12.0, 16.5);
            r.line(12.0, 16.5, 21.0, 12.0);

            r.line(3.0, 16.5, 12.0, 21.0);
            r.line(12.0, 21.0, 21.0, 16.5);
        }

        // Actions & Tools
        IconId::Search => {
            // Magnifying glass at 45 degrees
            r.circle_outline(10.0, 10.0, 5.5);
            r.line(14.0, 14.0, 20.0, 20.0);
        }
        IconId::Refresh => {
            // Circular reload arrow
            r.arc(12.0, 12.0, 7.0, -140.0, 130.0);
            r.line(19.0, 10.0, 19.0, 15.0);
            r.line(19.0, 15.0, 14.0, 15.0);
        }
        IconId::Home => {
            // Minimalist house
            r.line(3.0, 10.5, 12.0, 3.5);
            r.line(12.0, 3.5, 21.0, 10.5);
            r.line(5.5, 9.5, 5.5, 20.0);
            r.line(5.5, 20.0, 18.5, 20.0);
            r.line(18.5, 20.0, 18.5, 9.5);
            r.rect_outline(10.0, 14.0, 4.0, 6.0, 0);
        }
        IconId::HardDrive => {
            // Clean hard drive chassis
            r.rect_outline(3.5, 6.0, 17.0, 12.0, 2);
            r.line(3.5, 14.0, 20.5, 14.0);
            r.fill_circle(17.0, 16.0, 1.0);
        }
        IconId::Trash => {
            // Trash bin
            r.line(4.0, 6.0, 20.0, 6.0);
            r.line(9.0, 6.0, 9.0, 3.5);
            r.line(9.0, 3.5, 15.0, 3.5);
            r.line(15.0, 3.5, 15.0, 6.0);

            r.line(6.0, 6.0, 7.0, 20.0);
            r.line(7.0, 20.0, 17.0, 20.0);
            r.line(17.0, 20.0, 18.0, 6.0);

            r.line(10.0, 10.0, 10.0, 16.0);
            r.line(14.0, 10.0, 14.0, 16.0);
        }
        IconId::Edit => {
            // Pencil at 45 degrees
            r.line(4.0, 20.0, 8.5, 19.5);
            r.line(4.0, 20.0, 4.5, 15.5);
            r.line(4.5, 15.5, 16.0, 4.0);
            r.line(16.0, 4.0, 20.0, 8.0);
            r.line(20.0, 8.0, 8.5, 19.5);
            r.line(4.5, 15.5, 8.5, 19.5);
        }
        IconId::Copy => {
            // Two overlapping rectangular cards
            r.rect_outline(8.0, 8.0, 12.0, 12.0, 2);
            r.line(16.0, 8.0, 16.0, 4.0);
            r.line(16.0, 4.0, 4.0, 4.0);
            r.line(4.0, 4.0, 4.0, 16.0);
            r.line(4.0, 16.0, 8.0, 16.0);
        }
        IconId::Cut => {
            // Scissors
            r.circle_outline(6.0, 16.0, 2.8);
            r.circle_outline(18.0, 16.0, 2.8);
            r.line(7.5, 13.5, 17.0, 4.0);
            r.line(16.5, 13.5, 7.0, 4.0);
        }
        IconId::Paste => {
            // Clipboard
            r.rect_outline(5.0, 6.0, 14.0, 15.0, 2);
            r.rect_outline(9.0, 3.0, 6.0, 4.0, 1);
            r.line(8.5, 11.0, 15.5, 11.0);
            r.line(8.5, 15.0, 13.5, 15.0);
        }
        IconId::Download => {
            // Arrow down into tray
            r.line(12.0, 3.5, 12.0, 15.0);
            r.line(7.5, 10.5, 12.0, 15.0);
            r.line(16.5, 10.5, 12.0, 15.0);
            r.line(4.0, 16.0, 4.0, 20.0);
            r.line(4.0, 20.0, 20.0, 20.0);
            r.line(20.0, 20.0, 20.0, 16.0);
        }
        IconId::Plus => {
            r.line(12.0, 5.0, 12.0, 19.0);
            r.line(5.0, 12.0, 19.0, 12.0);
        }
        IconId::Minus => {
            r.line(5.0, 12.0, 19.0, 12.0);
        }
        IconId::Check => {
            r.line(4.5, 12.5, 9.5, 17.5);
            r.line(9.5, 17.5, 19.5, 6.5);
        }

        // System Status & Hardware
        IconId::Palette => {
            // Artist palette
            r.circle_outline(12.0, 12.0, 8.5);
            r.fill_circle(9.0, 9.0, 1.4);
            r.fill_circle(14.0, 8.5, 1.4);
            r.fill_circle(16.0, 13.0, 1.4);
            r.fill_circle(12.0, 16.0, 1.4);
        }
        IconId::Monitor => {
            // Display screen + stand
            r.rect_outline(3.0, 4.5, 18.0, 12.0, 2);
            r.line(12.0, 16.5, 12.0, 20.0);
            r.line(8.0, 20.0, 16.0, 20.0);
        }
        IconId::Wifi => {
            // Radiating wifi arcs
            r.arc(12.0, 18.0, 13.0, -145.0, -35.0);
            r.arc(12.0, 18.0, 8.5, -145.0, -35.0);
            r.arc(12.0, 18.0, 4.0, -145.0, -35.0);
            r.fill_circle(12.0, 18.0, 1.2);
        }
        IconId::Cpu => {
            // CPU Chip with edge pins
            r.rect_outline(6.0, 6.0, 12.0, 12.0, 2);
            r.rect_outline(9.0, 9.0, 6.0, 6.0, 1);
            // Pins top/bottom
            r.line(9.0, 2.5, 9.0, 6.0);
            r.line(15.0, 2.5, 15.0, 6.0);
            r.line(9.0, 18.0, 9.0, 21.5);
            r.line(15.0, 18.0, 15.0, 21.5);
            // Pins left/right
            r.line(2.5, 9.0, 6.0, 9.0);
            r.line(2.5, 15.0, 6.0, 15.0);
            r.line(18.0, 9.0, 21.5, 9.0);
            r.line(18.0, 15.0, 21.5, 15.0);
        }
        IconId::Mouse => {
            // Mouse with center line
            r.rect_outline(6.5, 4.0, 11.0, 16.0, 5);
            r.line(12.0, 5.0, 12.0, 10.0);
        }
        IconId::Info => {
            // Circle with 'i'
            r.circle_outline(12.0, 12.0, 8.5);
            r.fill_circle(12.0, 8.0, 1.2);
            r.line(12.0, 11.0, 12.0, 16.0);
        }
        IconId::Clock => {
            // Circle with clock hands
            r.circle_outline(12.0, 12.0, 8.5);
            r.line(12.0, 7.5, 12.0, 12.0);
            r.line(12.0, 12.0, 16.0, 12.0);
        }
        IconId::Globe => {
            // Globe circle with equator and meridian
            r.circle_outline(12.0, 12.0, 8.5);
            r.line(3.5, 12.0, 20.5, 12.0);
            r.arc(12.0, 12.0, 5.5, -90.0, 90.0);
            r.arc(12.0, 12.0, 5.5, 90.0, 270.0);
        }
        IconId::Power => {
            // Power circle with top notch & line
            r.arc(12.0, 13.0, 7.5, -55.0, 235.0);
            r.line(12.0, 4.0, 12.0, 12.0);
        }
        IconId::Lock => {
            // Shackle + body
            r.rect_outline(5.0, 10.0, 14.0, 10.5, 2);
            r.arc(12.0, 10.0, 4.5, 180.0, 360.0);
            r.line(7.5, 10.0, 7.5, 10.5);
            r.line(16.5, 10.0, 16.5, 10.5);
        }
        IconId::Unlock => {
            // Open shackle + body
            r.rect_outline(5.0, 10.0, 14.0, 10.5, 2);
            r.arc(14.0, 7.0, 4.5, 180.0, 360.0);
            r.line(9.5, 7.0, 9.5, 10.0);
        }
    }
}