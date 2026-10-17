#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[allow(dead_code)]
impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self::rgb(r, g, b)
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn new_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);

    // macOS-inspired palette
    pub const MACOS_BG_LIGHT: Color = Color::rgb(30, 30, 30);         // Dark mode desktop bg
    pub const MACOS_WINDOW_BG: Color = Color::rgb(38, 38, 40);        // Window body
    pub const MACOS_TITLEBAR: Color = Color::rgb(56, 56, 58);         // Window title bar  
    pub const MACOS_TITLEBAR_INACTIVE: Color = Color::rgb(46, 46, 48);
    pub const MACOS_SIDEBAR: Color = Color::rgb(44, 44, 46);          // Sidebar bg
    pub const MACOS_BORDER: Color = Color::rgb(72, 72, 74);           // Subtle borders
    pub const MACOS_BORDER_ACTIVE: Color = Color::rgb(100, 100, 105);  
    pub const MACOS_TEXT: Color = Color::rgb(235, 235, 240);           // Primary text
    pub const MACOS_TEXT_SECONDARY: Color = Color::rgb(152, 152, 157);// Secondary text
    pub const MACOS_ACCENT: Color = Color::rgb(10, 132, 255);         // Blue accent
    pub const MACOS_CLOSE: Color = Color::rgb(255, 95, 87);           // Traffic light red
    pub const MACOS_MINIMIZE: Color = Color::rgb(255, 189, 46);       // Traffic light yellow
    pub const MACOS_MAXIMIZE: Color = Color::rgb(40, 200, 64);        // Traffic light green
    pub const MACOS_DOCK_BG: Color = Color::rgba(50, 50, 52, 200);    // Dock translucent bg
    pub const MACOS_MENUBAR: Color = Color::rgb(44, 44, 46);          // Menu bar
    pub const MACOS_SELECTION: Color = Color::rgb(10, 132, 255);      // Selection blue
    pub const MACOS_TERMINAL_BG: Color = Color::rgb(30, 30, 30);      // Terminal background
    pub const MACOS_HOVER: Color = Color::rgb(62, 62, 64);            // Hover state

    pub fn blend(base: Color, add: Color, alpha: f32) -> Color {
        if alpha <= 0.0 {
            return base;
        }
        if alpha >= 1.0 {
            return add;
        }
        let inv = 1.0 - alpha;
        Color {
            r: (base.r as f32 * inv + add.r as f32 * alpha) as u8,
            g: (base.g as f32 * inv + add.g as f32 * alpha) as u8,
            b: (base.b as f32 * inv + add.b as f32 * alpha) as u8,
            a: 255,
        }
    }

    #[inline(always)]
    pub fn scale(&self, factor: f32) -> Color {
        let f = factor.clamp(0.0, 2.0);
        Color {
            r: ((self.r as f32 * f).min(255.0)) as u8,
            g: ((self.g as f32 * f).min(255.0)) as u8,
            b: ((self.b as f32 * f).min(255.0)) as u8,
            a: self.a,
        }
    }

    #[inline(always)]
    pub fn add_clamped(&self, other: Color) -> Color {
        Color {
            r: (self.r as u16 + other.r as u16).min(255) as u8,
            g: (self.g as u16 + other.g as u16).min(255) as u8,
            b: (self.b as u16 + other.b as u16).min(255) as u8,
            a: self.a,
        }
    }

    /// Returns this color with a new alpha value
    #[inline(always)]
    pub fn with_alpha(self, a: u8) -> Color {
        Color { r: self.r, g: self.g, b: self.b, a }
    }

    // Leviathan / Incy OS brand gold
    pub const INCY_GOLD: Color = Color::rgb(255, 190, 40);
pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
}
