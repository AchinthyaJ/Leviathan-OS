use crate::color::Color;
use crate::graphics::Graphics;
use crate::icons::{draw_icon, IconId};
use crate::mouse::MouseState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    pub fn draw_fill(&self, gfx: &mut Graphics, color: Color) {
        if self.w > 0 && self.h > 0 {
            gfx.fill_rect(self.x, self.y, self.w, self.h, color);
        }
    }

    pub fn draw_outline(&self, gfx: &mut Graphics, color: Color) {
        if self.w > 0 && self.h > 0 {
            gfx.draw_rect(self.x, self.y, self.w, self.h, color);
        }
    }

    pub fn draw_card(&self, gfx: &mut Graphics, fill: Color, border: Color) {
        self.draw_fill(gfx, fill);
        self.draw_outline(gfx, border);
    }

    pub fn draw_squircle_card(&self, gfx: &mut Graphics, radius: i32, fill: Color, border: Color) {
        gfx.draw_squircle_card(self.x, self.y, self.w, self.h, radius, fill, border);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    TextOnly,
    IconOnly,
    IconLeft,
    IconRight,
}

pub struct Button {
    pub rect: Rect,
    pub label: String,
    pub icon: Option<IconId>,
    pub icon_size: i32,
    pub icon_spacing: i32,
    pub style: ButtonStyle,
    pub bg_color: Color,
    pub hover_color: Color,
    pub border_color: Color,
    pub text_color: Color,
    pub icon_color: Color,
    pub corner_radius: i32,
}

impl Button {
    pub fn new(x: i32, y: i32, w: i32, h: i32, label: &str) -> Self {
        Self {
            rect: Rect::new(x, y, w, h),
            label: label.to_string(),
            icon: None,
            icon_size: 14,
            icon_spacing: 6,
            style: ButtonStyle::TextOnly,
            bg_color: Color::MACOS_HOVER,
            hover_color: Color::MACOS_ACCENT,
            border_color: Color::MACOS_BORDER,
            text_color: Color::MACOS_TEXT,
            icon_color: Color::MACOS_TEXT,
            corner_radius: 6,
        }
    }

    pub fn new_icon(x: i32, y: i32, w: i32, h: i32, icon: IconId) -> Self {
        Self {
            rect: Rect::new(x, y, w, h),
            label: String::new(),
            icon: Some(icon),
            icon_size: (h - 8).max(12).min(20),
            icon_spacing: 0,
            style: ButtonStyle::IconOnly,
            bg_color: Color::MACOS_HOVER,
            hover_color: Color::MACOS_ACCENT,
            border_color: Color::MACOS_BORDER,
            text_color: Color::MACOS_TEXT,
            icon_color: Color::MACOS_TEXT,
            corner_radius: 6,
        }
    }

    pub fn new_with_icon(x: i32, y: i32, w: i32, h: i32, icon: IconId, label: &str) -> Self {
        Self {
            rect: Rect::new(x, y, w, h),
            label: label.to_string(),
            icon: Some(icon),
            icon_size: 14,
            icon_spacing: 6,
            style: ButtonStyle::IconLeft,
            bg_color: Color::MACOS_HOVER,
            hover_color: Color::MACOS_ACCENT,
            border_color: Color::MACOS_BORDER,
            text_color: Color::MACOS_TEXT,
            icon_color: Color::MACOS_TEXT,
            corner_radius: 6,
        }
    }

    pub fn is_clicked(&self, mouse: &MouseState) -> bool {
        mouse.left_clicked && self.rect.contains(mouse.x, mouse.y)
    }

    pub fn draw(&self, gfx: &mut Graphics, mouse: &MouseState) {
        let is_hovered = self.rect.contains(mouse.x, mouse.y);
        let fill = if is_hovered {
            self.hover_color
        } else {
            self.bg_color
        };
        let border = if is_hovered {
            Color::MACOS_BORDER_ACTIVE
        } else {
            self.border_color
        };
        let text_col = if is_hovered {
            Color::new(255, 255, 255)
        } else {
            self.text_color
        };
        let icon_col = if is_hovered {
            Color::new(255, 255, 255)
        } else {
            self.icon_color
        };

        gfx.draw_squircle_card(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            self.corner_radius,
            fill,
            border,
        );

        match self.style {
            ButtonStyle::IconOnly => {
                if let Some(icon) = self.icon {
                    let ix = self.rect.x + (self.rect.w - self.icon_size) / 2;
                    let iy = self.rect.y + (self.rect.h - self.icon_size) / 2;
                    draw_icon(gfx, icon, ix, iy, self.icon_size, icon_col);
                }
            }
            ButtonStyle::IconLeft => {
                let text_w = (self.label.len() * 8) as i32;
                let total_content_w = if self.icon.is_some() {
                    self.icon_size + self.icon_spacing + text_w
                } else {
                    text_w
                };
                let start_x = self.rect.x + (self.rect.w - total_content_w) / 2;

                let mut current_x = start_x;
                if let Some(icon) = self.icon {
                    let iy = self.rect.y + (self.rect.h - self.icon_size) / 2;
                    draw_icon(gfx, icon, current_x, iy, self.icon_size, icon_col);
                    current_x += self.icon_size + self.icon_spacing;
                }

                let text_y = self.rect.y + (self.rect.h - 12) / 2;
                if current_x >= 0 && text_y >= 0 && !self.label.is_empty() {
                    gfx.draw_string(current_x as u32, text_y as u32, &self.label, text_col);
                }
            }
            ButtonStyle::IconRight => {
                let text_w = (self.label.len() * 8) as i32;
                let total_content_w = if self.icon.is_some() {
                    self.icon_size + self.icon_spacing + text_w
                } else {
                    text_w
                };
                let start_x = self.rect.x + (self.rect.w - total_content_w) / 2;

                let text_y = self.rect.y + (self.rect.h - 12) / 2;
                if start_x >= 0 && text_y >= 0 && !self.label.is_empty() {
                    gfx.draw_string(start_x as u32, text_y as u32, &self.label, text_col);
                }

                if let Some(icon) = self.icon {
                    let ix = start_x + text_w + self.icon_spacing;
                    let iy = self.rect.y + (self.rect.h - self.icon_size) / 2;
                    draw_icon(gfx, icon, ix, iy, self.icon_size, icon_col);
                }
            }
            ButtonStyle::TextOnly => {
                let text_w = (self.label.len() * 8) as i32;
                let text_x = self.rect.x + (self.rect.w - text_w) / 2;
                let text_y = self.rect.y + (self.rect.h - 12) / 2;

                if text_x >= 0 && text_y >= 0 && !self.label.is_empty() {
                    gfx.draw_string(text_x as u32, text_y as u32, &self.label, text_col);
                }
            }
        }
    }
}

pub struct Window {
    pub title: String,
    pub rect: Rect,
    pub normal_rect: Rect,
    pub is_open: bool,
    pub is_focused: bool,
    pub is_maximized: bool,
    pub is_minimized: bool,
    pub is_dragging: bool,
    drag_offset_x: i32,
    drag_offset_y: i32,
    pub title_height: i32,
}

impl Window {
    pub fn new(title: &str, x: i32, y: i32, w: i32, h: i32) -> Self {
        let title_height = 28;
        let rect = Rect::new(x, y, w, h);

        Self {
            title: title.to_string(),
            rect,
            normal_rect: rect,
            is_open: true,
            is_focused: true,
            is_maximized: false,
            is_minimized: false,
            is_dragging: false,
            drag_offset_x: 0,
            drag_offset_y: 0,
            title_height,
        }
    }

    pub fn client_rect(&self) -> Rect {
        Rect::new(
            self.rect.x + 1,
            self.rect.y + self.title_height,
            (self.rect.w - 2).max(10),
            (self.rect.h - self.title_height - 1).max(10),
        )
    }

    pub fn close_button_rect(&self) -> Rect {
        Rect::new(self.rect.x + self.rect.w - 24, self.rect.y + 4, 20, 20)
    }

    pub fn maximize_button_rect(&self) -> Rect {
        Rect::new(self.rect.x + self.rect.w - 48, self.rect.y + 4, 20, 20)
    }

    pub fn minimize_button_rect(&self) -> Rect {
        Rect::new(self.rect.x + self.rect.w - 72, self.rect.y + 4, 20, 20)
    }

    pub fn toggle_maximize(&mut self, screen_w: i32, screen_h: i32) {
        if self.is_maximized {
            self.rect = self.normal_rect;
            self.is_maximized = false;
        } else {
            self.normal_rect = self.rect;
            let max_x = 0;
            let max_y = 28;
            let max_w = (screen_w - 2).max(200);
            let max_h = (screen_h - 28 - 72 - 2).max(150);
            self.rect = Rect::new(max_x, max_y, max_w, max_h);
            self.is_maximized = true;
        }
    }

    pub fn update_drag(&mut self, mouse: &MouseState, screen_w: i32, screen_h: i32) {
        let title_bar = Rect::new(self.rect.x, self.rect.y, self.rect.w, self.title_height);
        let btn_close = self.close_button_rect();
        let btn_max = self.maximize_button_rect();
        let btn_min = self.minimize_button_rect();

        // Check close button click
        if mouse.left_clicked && btn_close.contains(mouse.x, mouse.y) {
            self.is_open = false;
            self.is_dragging = false;
            return;
        }

        // Check maximize button click
        if mouse.left_clicked && btn_max.contains(mouse.x, mouse.y) {
            self.toggle_maximize(screen_w, screen_h);
            return;
        }

        // Check minimize button click
        if mouse.left_clicked && btn_min.contains(mouse.x, mouse.y) {
            self.is_minimized = true;
            self.is_focused = false;
            self.is_dragging = false;
            return;
        }

        // Start drag if clicking on title bar (not on control buttons)
        if mouse.left_clicked
            && title_bar.contains(mouse.x, mouse.y)
            && !btn_close.contains(mouse.x, mouse.y)
            && !btn_max.contains(mouse.x, mouse.y)
            && !btn_min.contains(mouse.x, mouse.y)
        {
            if !self.is_maximized {
                self.is_dragging = true;
                self.drag_offset_x = mouse.x - self.rect.x;
                self.drag_offset_y = mouse.y - self.rect.y;
            }
            self.is_focused = true;
        }

        if mouse.left_released {
            self.is_dragging = false;
        }

        if self.is_dragging && mouse.left_pressed && !self.is_maximized {
            let new_x = mouse.x - self.drag_offset_x;
            let new_y = mouse.y - self.drag_offset_y;

            self.rect.x = new_x.clamp(-self.rect.w + 60, screen_w - 60);
            self.rect.y = new_y.clamp(28, screen_h - 40);
        }
    }

    pub fn draw_frame(&self, gfx: &mut Graphics, mouse: &MouseState) {
        if !self.is_open || self.is_minimized {
            return;
        }

        // 1. Subtle drop shadow
        gfx.fill_squircle(self.rect.x + 3, self.rect.y + 4, self.rect.w, self.rect.h, 14, Color::new(8, 10, 16));

        // 2. Window body background
        gfx.fill_squircle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, 14, Color::new(20, 24, 34));

        // 3. Titlebar
        let title_bg = if self.is_focused {
            Color::new(28, 35, 50)
        } else {
            Color::new(18, 22, 30)
        };
        gfx.fill_squircle(self.rect.x, self.rect.y, self.rect.w, self.title_height + 8, 14, title_bg);
        gfx.fill_rect(self.rect.x, self.rect.y + 14, self.rect.w, self.title_height - 14, title_bg);

        // Subtle bottom border on title bar
        gfx.draw_line(
            self.rect.x,
            self.rect.y + self.title_height,
            self.rect.x + self.rect.w,
            self.rect.y + self.title_height,
            Color::new(45, 55, 75),
        );

        // 4. Window border
        let border_color = if self.is_focused {
            Color::new(0, 190, 210) // Leviathan Cyan
        } else {
            Color::new(45, 55, 75)
        };
        gfx.draw_squircle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, 14, border_color);

        // 5. Title text
        let title_color = if self.is_focused {
            Color::new(245, 248, 255)
        } else {
            Color::new(140, 150, 165)
        };
        gfx.draw_string((self.rect.x + 12) as u32, (self.rect.y + 7) as u32, &self.title, title_color);

        // 6. Right-side Control Buttons (Minimize, Maximize, Close) with squircles
        let btn_min = self.minimize_button_rect();
        let hover_min = btn_min.contains(mouse.x, mouse.y);
        let min_bg = if hover_min { Color::new(65, 75, 95) } else { Color::new(45, 52, 68) };
        let min_fg = if hover_min { Color::new(255, 255, 255) } else { Color::new(200, 205, 215) };
        gfx.fill_squircle(btn_min.x, btn_min.y, btn_min.w, btn_min.h, 6, min_bg);
        draw_icon(gfx, IconId::Minimize, btn_min.x + (btn_min.w - 12) / 2, btn_min.y + (btn_min.h - 12) / 2, 12, min_fg);

        let btn_max = self.maximize_button_rect();
        let hover_max = btn_max.contains(mouse.x, mouse.y);
        let max_bg = if hover_max { Color::new(50, 140, 170) } else { Color::new(45, 52, 68) };
        let max_fg = if hover_max { Color::new(255, 255, 255) } else { Color::new(200, 205, 215) };
        let max_icon = if self.is_maximized { IconId::Restore } else { IconId::Maximize };
        gfx.fill_squircle(btn_max.x, btn_max.y, btn_max.w, btn_max.h, 6, max_bg);
        draw_icon(gfx, max_icon, btn_max.x + (btn_max.w - 12) / 2, btn_max.y + (btn_max.h - 12) / 2, 12, max_fg);

        let btn_close = self.close_button_rect();
        let hover_close = btn_close.contains(mouse.x, mouse.y);
        let close_bg = if hover_close { Color::new(220, 53, 69) } else { Color::new(170, 40, 50) };
        let close_fg = if hover_close { Color::new(255, 255, 255) } else { Color::new(240, 240, 240) };
        gfx.fill_squircle(btn_close.x, btn_close.y, btn_close.w, btn_close.h, 6, close_bg);
        draw_icon(gfx, IconId::Close, btn_close.x + (btn_close.w - 12) / 2, btn_close.y + (btn_close.h - 12) / 2, 12, close_fg);
    }
}
