#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[allow(dead_code)]
impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
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
}
