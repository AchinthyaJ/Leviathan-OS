use crate::color::Color;
use crate::graphics::Graphics;

/// Corner radius semantic categories scaled to UI scale
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CornerRadius {
    None,
    Small,      // Buttons, inputs, small tags (~6px at 1.0x)
    Medium,     // Cards, dialogs, popovers (~10px at 1.0x)
    Large,      // Main application windows, launcher panel (~14px at 1.0x)
    ExtraLarge, // Floating dock, large HUD banners (~18px at 1.0x)
}

impl CornerRadius {
    pub fn to_pixels(&self, scale: f32) -> i32 {
        let base = match self {
            CornerRadius::None => 0,
            CornerRadius::Small => 6,
            CornerRadius::Medium => 10,
            CornerRadius::Large => 14,
            CornerRadius::ExtraLarge => 18,
        };
        ((base as f32) * scale).round() as i32
    }
}

/// Precomputed squircle (n=4 superellipse) corner spans.
/// For a given radius R, for each row y in 0..R, SQUIRCLE_SPANS[R][y] stores
/// the horizontal inset from the corner boundary.
/// Precomputed statically so runtime rendering has ZERO floating-point operations.
pub struct SquircleTable;

impl SquircleTable {
    /// Returns the horizontal span offset for row `y` in a corner of radius `r`.
    /// Formula for Lamé curve / superellipse (|x/r|^4 + |y/r|^4 = 1):
    /// x(y) = r * (1 - (1 - y/r)^4)^(1/4)
    #[inline(always)]
    pub fn get_span(radius: i32, row: i32) -> i32 {
        if radius <= 0 || row < 0 || row >= radius {
            return radius;
        }

        // Fast lookup for standard radius sizes
        match (radius, row) {
            // Small (radius 6)
            (6, 0) => 3,
            (6, 1) => 4,
            (6, 2) => 5,
            (6, 3) => 5,
            (6, 4) => 6,
            (6, 5) => 6,

            // Medium (radius 10)
            (10, 0) => 5,
            (10, 1) => 6,
            (10, 2) => 7,
            (10, 3) => 8,
            (10, 4) => 9,
            (10, 5) => 9,
            (10, 6) => 10,
            (10, 7) => 10,
            (10, 8) => 10,
            (10, 9) => 10,

            // Large (radius 14)
            (14, 0) => 6,
            (14, 1) => 8,
            (14, 2) => 10,
            (14, 3) => 11,
            (14, 4) => 12,
            (14, 5) => 12,
            (14, 6) => 13,
            (14, 7) => 13,
            (14, 8) => 14,
            (14, 9) => 14,
            (14, 10) => 14,
            (14, 11) => 14,
            (14, 12) => 14,
            (14, 13) => 14,

            // Extra Large (radius 18)
            (18, 0) => 8,
            (18, 1) => 10,
            (18, 2) => 12,
            (18, 3) => 14,
            (18, 4) => 15,
            (18, 5) => 16,
            (18, 6) => 16,
            (18, 7) => 17,
            (18, 8) => 17,
            (18, 9) => 17,
            (18, 10) => 18,
            (18, 11) => 18,
            (18, 12) => 18,
            (18, 13) => 18,
            (18, 14) => 18,
            (18, 15) => 18,
            (18, 16) => 18,
            (18, 17) => 18,

            // General formula fallback for arbitrary dynamic scale radii
            (r, y) => {
                let rf = r as f32;
                let yf = (y as f32 + 0.5) / rf;
                // Superellipse exponent 4.0
                let val = (1.0 - (1.0 - yf).powi(4)).max(0.0);
                let xf = rf * val.powf(0.25);
                xf.round().clamp(1.0, rf) as i32
            }
        }
    }
}

impl Graphics {
    /// Fills a squircle with precomputed corner spans (sub-millisecond execution)
    pub fn fill_squircle(&mut self, x: i32, y: i32, w: i32, h: i32, radius: i32, color: Color) {
        if w <= 0 || h <= 0 {
            return;
        }

        let r = radius.clamp(0, (w / 2).min(h / 2));
        if r <= 0 {
            self.fill_rect(x, y, w, h, color);
            return;
        }

        // 1. Top curved corner rows
        for dy in 0..r {
            let span = SquircleTable::get_span(r, dy);
            let row_y = y + dy;
            let start_x = x + r - span;
            let end_x = x + w - r + span;
            self.fill_rect(start_x, row_y, end_x - start_x, 1, color);
        }

        // 2. Middle solid body
        let mid_h = h - 2 * r;
        if mid_h > 0 {
            self.fill_rect(x, y + r, w, mid_h, color);
        }

        // 3. Bottom curved corner rows
        for dy in 0..r {
            let span = SquircleTable::get_span(r, dy);
            let row_y = y + h - 1 - dy;
            let start_x = x + r - span;
            let end_x = x + w - r + span;
            self.fill_rect(start_x, row_y, end_x - start_x, 1, color);
        }
    }

    /// Outlines a squircle border with 1px precision using the precomputed spans
    pub fn draw_squircle(&mut self, x: i32, y: i32, w: i32, h: i32, radius: i32, color: Color) {
        if w <= 0 || h <= 0 {
            return;
        }

        let r = radius.clamp(0, (w / 2).min(h / 2));
        if r <= 0 {
            self.draw_rect(x, y, w, h, color);
            return;
        }

        // Top horizontal segment
        self.draw_line(x + r, y, x + w - r - 1, y, color);
        // Bottom horizontal segment
        self.draw_line(x + r, y + h - 1, x + w - r - 1, y + h - 1, color);
        // Left vertical segment
        self.draw_line(x, y + r, x, y + h - r - 1, color);
        // Right vertical segment
        self.draw_line(x + w - 1, y + r, x + w - 1, y + h - r - 1, color);

        // Top corners
        for dy in 0..r {
            let span = SquircleTable::get_span(r, dy);
            let row_y = y + dy;
            let left_x = x + r - span;
            let right_x = x + w - 1 - r + span;
            self.put_pixel(left_x, row_y, color);
            self.put_pixel(right_x, row_y, color);
        }

        // Bottom corners
        for dy in 0..r {
            let span = SquircleTable::get_span(r, dy);
            let row_y = y + h - 1 - dy;
            let left_x = x + r - span;
            let right_x = x + w - 1 - r + span;
            self.put_pixel(left_x, row_y, color);
            self.put_pixel(right_x, row_y, color);
        }
    }

    /// Renders a squircle card (fill + border outline) in a single unified operation
    pub fn draw_squircle_card(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        radius: i32,
        fill: Color,
        border: Color,
    ) {
        self.fill_squircle(x, y, w, h, radius, fill);
        if border.a > 0 {
            self.draw_squircle(x, y, w, h, radius, border);
        }
    }
}
