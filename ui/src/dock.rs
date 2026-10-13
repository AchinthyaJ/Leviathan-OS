use crate::color::Color;
use crate::graphics::Graphics;
use crate::icons::{draw_icon, IconId};
use crate::mouse::MouseState;
use crate::widget::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockAppId {
    AllApps,
    Terminal,
    Packages,
    Settings,
    Files,
    Abyss3D,
    LayerEditor,
}

pub struct DockItem {
    pub id: DockAppId,
    pub name: &'static str,
    pub icon: IconId,
    pub is_running: bool,
}

pub struct FloatingDock {
    pub rect: Rect,
    pub items: Vec<DockItem>,
    pub item_width: i32,
    pub is_all_apps_open: bool,
}

impl FloatingDock {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        let items = vec![
            DockItem {
                id: DockAppId::AllApps,
                name: "All Apps",
                icon: IconId::AllApps,
                is_running: false,
            },
            DockItem {
                id: DockAppId::Terminal,
                name: "Terminal",
                icon: IconId::Terminal,
                is_running: true,
            },
            DockItem {
                id: DockAppId::Packages,
                name: "Packages",
                icon: IconId::Packages,
                is_running: false,
            },
            DockItem {
                id: DockAppId::Settings,
                name: "Settings",
                icon: IconId::Settings,
                is_running: false,
            },
            DockItem {
                id: DockAppId::Files,
                name: "Files",
                icon: IconId::Files,
                is_running: false,
            },
            DockItem {
                id: DockAppId::Abyss3D,
                name: "3D Abyss",
                icon: IconId::Abyss3D,
                is_running: false,
            },
            DockItem {
                id: DockAppId::LayerEditor,
                name: "Layer Editor",
                icon: IconId::LayerEditor,
                is_running: false,
            },
        ];

        let item_width = 80;
        let padding = 12;
        let dock_w = (items.len() as i32) * item_width + padding * 2;
        let dock_h = 56;
        let dock_x = (screen_width as i32 - dock_w) / 2;
        let dock_y = screen_height as i32 - dock_h - 16; // 16px float from bottom

        Self {
            rect: Rect::new(dock_x, dock_y, dock_w, dock_h),
            items,
            item_width,
            is_all_apps_open: false,
        }
    }

    pub fn handle_click(&mut self, mouse: &MouseState) -> Option<DockAppId> {
        if !mouse.left_clicked || !self.rect.contains(mouse.x, mouse.y) {
            return None;
        }

        let rel_x = mouse.x - (self.rect.x + 12);
        if rel_x < 0 {
            return None;
        }

        let idx = (rel_x / self.item_width) as usize;
        if idx < self.items.len() {
            let id = self.items[idx].id;
            if id == DockAppId::AllApps {
                self.is_all_apps_open = !self.is_all_apps_open;
            }
            return Some(id);
        }

        None
    }

    pub fn set_layout(&mut self, screen_width: u32, screen_height: u32, scale: f32) {
        let base_item_w = 80.0 * scale;
        self.item_width = base_item_w.round() as i32;
        let padding = (12.0 * scale).round() as i32;
        let dock_w = (self.items.len() as i32) * self.item_width + padding * 2;
        let dock_h = (56.0 * scale).round() as i32;
        let dock_x = (screen_width as i32 - dock_w) / 2;
        let dock_y = screen_height as i32 - dock_h - (16.0 * scale).round() as i32;
        self.rect = Rect::new(dock_x, dock_y, dock_w, dock_h);
    }

    pub fn draw(&self, gfx: &mut Graphics, mouse: &MouseState) {
        // 1. Frosted glass drop shadow with squircle
        gfx.fill_squircle(self.rect.x + 3, self.rect.y + 4, self.rect.w, self.rect.h, 18, Color::new(15, 15, 15));

        // 2. Dock pill background with squircle
        let dock_bg = Color::new(45, 45, 48);
        let dock_border = Color::new(75, 75, 80);
        gfx.draw_squircle_card(self.rect.x, self.rect.y, self.rect.w, self.rect.h, 18, dock_bg, dock_border);

        // 3. Render dock app slots
        let mut start_x = self.rect.x + 12;
        for item in &self.items {
            let slot_rect = Rect::new(start_x, self.rect.y + 6, self.item_width, self.rect.h - 12);
            let is_hovered = slot_rect.contains(mouse.x, mouse.y);

            if is_hovered {
                // Highlight backing with squircle
                gfx.draw_squircle_card(slot_rect.x, slot_rect.y, slot_rect.w, slot_rect.h, 8, Color::new(65, 65, 70), Color::new(90, 90, 95));

                // Draw tooltip above dock
                let tip_w = (item.name.len() * 8 + 16) as i32;
                let tip_x = start_x + (self.item_width - tip_w) / 2;
                let tip_y = self.rect.y - 28;
                gfx.draw_squircle_card(tip_x, tip_y, tip_w, 22, 6, Color::new(45, 45, 48), Color::new(80, 80, 84));
                gfx.draw_string(
                    (tip_x + 8) as u32,
                    (tip_y + 5) as u32,
                    item.name,
                    Color::MACOS_TEXT,
                );
            }

            // Draw Icon
            let icon_size = 24;
            let icon_x = start_x + (self.item_width - icon_size) / 2;
            let icon_y = (self.rect.y + 6) + ((self.rect.h - 12) - icon_size) / 2 - 2;
            let icon_color = if item.id == DockAppId::AllApps && self.is_all_apps_open {
                Color::MACOS_ACCENT
            } else if is_hovered {
                Color::MACOS_ACCENT
            } else {
                Color::new(230, 235, 245)
            };
            draw_icon(gfx, item.icon, icon_x, icon_y, icon_size, icon_color);

            // Active running indicator dot underneath running apps
            if item.is_running {
                let dot_x = start_x + self.item_width / 2 - 2;
                let dot_y = self.rect.y + self.rect.h - 8;
                let dot_col = if is_hovered { Color::MACOS_ACCENT } else { Color::new(180, 190, 210) };
                gfx.fill_squircle(dot_x, dot_y, 5, 3, 1, dot_col);
            }

            start_x += self.item_width;
        }
    }
}
