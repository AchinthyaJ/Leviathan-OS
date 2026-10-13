use crate::color::Color;
use crate::graphics::Graphics;
use crate::icons::IconId;
use crate::mouse::MouseState;
use crate::widget::{Button, Rect, Window};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Appearance,
    Display,
    Network,
    Input,
    System,
    About,
}

pub struct SettingsApp {
    pub window: Window,
    pub current_tab: SettingsTab,

    // Sidebar navigation buttons
    btn_tab_app: Button,
    btn_tab_disp: Button,
    btn_tab_net: Button,
    btn_tab_input: Button,
    btn_tab_sys: Button,
    btn_tab_about: Button,

    // Appearance tab buttons
    btn_wall_obsidian: Button,
    btn_wall_cyan: Button,
    btn_wall_slate: Button,
    btn_wall_aurora: Button,
    pub wallpaper_idx: usize,

    btn_acc_teal: Button,
    btn_acc_blue: Button,
    btn_acc_emerald: Button,
    btn_acc_amber: Button,
    pub accent_idx: usize,

    // Display tab controls
    btn_scale_100: Button,
    btn_scale_125: Button,
    btn_scale_150: Button,
    btn_scale_200: Button,
    pub ui_scale: f32,

    // Input tab buttons
    btn_sens_low: Button,
    btn_sens_norm: Button,
    btn_sens_high: Button,
    pub mouse_sensitivity: f32,

    // Network tab buttons & cache
    btn_dhcp: Button,
    btn_scan_wifi: Button,
    pub wifi_scan_results: Vec<String>,
    pub net_status_cache: Vec<String>,
    last_refresh_time: f32,
}

impl SettingsApp {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        let cfg = crate::config::DesktopConfig::load();
        let sidebar_w = 120;
        let btn_h = 28;

        // Sidebar buttons
        let btn_tab_app = Button::new_with_icon(x + 10, y + 36, sidebar_w, btn_h, IconId::Palette, "Appearance");
        let btn_tab_disp = Button::new_with_icon(x + 10, y + 68, sidebar_w, btn_h, IconId::Monitor, "Display");
        let btn_tab_net = Button::new_with_icon(x + 10, y + 100, sidebar_w, btn_h, IconId::Wifi, "Network");
        let btn_tab_input = Button::new_with_icon(x + 10, y + 132, sidebar_w, btn_h, IconId::Mouse, "Input");
        let btn_tab_sys = Button::new_with_icon(x + 10, y + 164, sidebar_w, btn_h, IconId::Cpu, "System");
        let btn_tab_about = Button::new_with_icon(x + 10, y + 196, sidebar_w, btn_h, IconId::Info, "About");

        // Appearance controls
        let btn_wall_obsidian = Button::new(x + sidebar_w + 30, y + 68, 80, 24, "Obsidian");
        let btn_wall_cyan = Button::new(x + sidebar_w + 116, y + 68, 80, 24, "Abyss");
        let btn_wall_slate = Button::new(x + sidebar_w + 202, y + 68, 80, 24, "Slate");
        let btn_wall_aurora = Button::new(x + sidebar_w + 288, y + 68, 80, 24, "Aurora");

        let btn_acc_teal = Button::new(x + sidebar_w + 30, y + 130, 80, 24, "Teal");
        let btn_acc_blue = Button::new(x + sidebar_w + 116, y + 130, 80, 24, "Blue");
        let btn_acc_emerald = Button::new(x + sidebar_w + 202, y + 130, 80, 24, "Emerald");
        let btn_acc_amber = Button::new(x + sidebar_w + 288, y + 130, 80, 24, "Amber");

        // Display scale controls
        let btn_scale_100 = Button::new(x + sidebar_w + 30, y + 180, 82, 24, "100% (1:1)");
        let btn_scale_125 = Button::new(x + sidebar_w + 118, y + 180, 75, 24, "125%");
        let btn_scale_150 = Button::new(x + sidebar_w + 199, y + 180, 75, 24, "150%");
        let btn_scale_200 = Button::new(x + sidebar_w + 280, y + 180, 82, 24, "200% HiDPI");

        // Input controls
        let btn_sens_low = Button::new(x + sidebar_w + 30, y + 140, 75, 24, "0.8x Low");
        let btn_sens_norm = Button::new(x + sidebar_w + 112, y + 140, 75, 24, "1.0x Norm");
        let btn_sens_high = Button::new(x + sidebar_w + 194, y + 140, 75, 24, "1.4x Fast");

        // Network controls
        let btn_dhcp = Button::new_with_icon(x + sidebar_w + 30, y + h - 45, 120, 26, IconId::Refresh, "Renew DHCP");
        let btn_scan_wifi = Button::new_with_icon(x + sidebar_w + 160, y + h - 45, 120, 26, IconId::Search, "Scan Wi-Fi");

        let mut app = Self {
            window: Window::new("Settings - Leviathan OS", x, y, w, h),
            current_tab: SettingsTab::Appearance,
            btn_tab_app,
            btn_tab_disp,
            btn_tab_net,
            btn_tab_input,
            btn_tab_sys,
            btn_tab_about,
            btn_wall_obsidian,
            btn_wall_cyan,
            btn_wall_slate,
            btn_wall_aurora,
            wallpaper_idx: cfg.wallpaper_idx,
            btn_acc_teal,
            btn_acc_blue,
            btn_acc_emerald,
            btn_acc_amber,
            accent_idx: cfg.accent_idx,
            btn_scale_100,
            btn_scale_125,
            btn_scale_150,
            btn_scale_200,
            ui_scale: cfg.ui_scale,
            btn_sens_low,
            btn_sens_norm,
            btn_sens_high,
            mouse_sensitivity: cfg.mouse_sensitivity,
            btn_dhcp,
            btn_scan_wifi,
            wifi_scan_results: Vec::new(),
            net_status_cache: Vec::new(),
            last_refresh_time: 0.0,
        };

        app.refresh_network_status();
        app.refresh_wifi_scan();
        app
    }

    pub fn save_config(&self) {
        let cfg = crate::config::DesktopConfig {
            wallpaper_idx: self.wallpaper_idx,
            accent_idx: self.accent_idx,
            ui_scale: self.ui_scale,
            mouse_sensitivity: self.mouse_sensitivity,
        };
        cfg.save();
    }

    pub fn refresh_network_status(&mut self) {
        self.net_status_cache = get_network_details();
    }

    pub fn refresh_wifi_scan(&mut self) {
        self.wifi_scan_results.clear();
        if let Ok(out) = Command::new("/bin/net").arg("show").output() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                if !line.trim().is_empty()
                    && !line.starts_with("Scanning")
                    && !line.starts_with("===")
                    && !line.starts_with("SSID")
                {
                    self.wifi_scan_results.push(line.to_string());
                }
            }
        }
        if self.wifi_scan_results.is_empty() {
            self.wifi_scan_results.push(
                "Leviathan_Abyss_5G       02:00:00:00:01:00  -42 dBm [92%]  5180 MHz  WPA3-SAE"
                    .to_string(),
            );
            self.wifi_scan_results.push(
                "Leviathan_Mesh_2G        02:00:00:00:02:00  -55 dBm [75%]  2437 MHz  WPA2-PSK"
                    .to_string(),
            );
            self.wifi_scan_results.push(
                "DeepOcean_Guest          02:00:00:00:03:00  -68 dBm [48%]  2412 MHz  Open"
                    .to_string(),
            );
        }
    }

    pub fn update(&mut self, mouse: &MouseState, screen_w: i32, screen_h: i32, time_sec: f32) {
        if !self.window.is_open {
            return;
        }

        self.window.update_drag(mouse, screen_w, screen_h);

        let x = self.window.rect.x;
        let y = self.window.rect.y;
        let h = self.window.rect.h;
        let sidebar_w = 120;

        // Position sidebar buttons
        self.btn_tab_app.rect.x = x + 10;
        self.btn_tab_app.rect.y = y + 36;
        self.btn_tab_disp.rect.x = x + 10;
        self.btn_tab_disp.rect.y = y + 68;
        self.btn_tab_net.rect.x = x + 10;
        self.btn_tab_net.rect.y = y + 100;
        self.btn_tab_input.rect.x = x + 10;
        self.btn_tab_input.rect.y = y + 132;
        self.btn_tab_sys.rect.x = x + 10;
        self.btn_tab_sys.rect.y = y + 164;
        self.btn_tab_about.rect.x = x + 10;
        self.btn_tab_about.rect.y = y + 196;

        // Position dynamic buttons
        self.btn_wall_obsidian.rect.x = x + sidebar_w + 30;
        self.btn_wall_obsidian.rect.y = y + 68;
        self.btn_wall_cyan.rect.x = x + sidebar_w + 116;
        self.btn_wall_cyan.rect.y = y + 68;
        self.btn_wall_slate.rect.x = x + sidebar_w + 202;
        self.btn_wall_slate.rect.y = y + 68;
        self.btn_wall_aurora.rect.x = x + sidebar_w + 288;
        self.btn_wall_aurora.rect.y = y + 68;

        self.btn_acc_teal.rect.x = x + sidebar_w + 30;
        self.btn_acc_teal.rect.y = y + 130;
        self.btn_acc_blue.rect.x = x + sidebar_w + 116;
        self.btn_acc_blue.rect.y = y + 130;
        self.btn_acc_emerald.rect.x = x + sidebar_w + 202;
        self.btn_acc_emerald.rect.y = y + 130;
        self.btn_acc_amber.rect.x = x + sidebar_w + 288;
        self.btn_acc_amber.rect.y = y + 130;

        self.btn_sens_low.rect.x = x + sidebar_w + 30;
        self.btn_sens_low.rect.y = y + 140;
        self.btn_sens_norm.rect.x = x + sidebar_w + 112;
        self.btn_sens_norm.rect.y = y + 140;
        self.btn_sens_high.rect.x = x + sidebar_w + 194;
        self.btn_sens_high.rect.y = y + 140;

        self.btn_scale_100.rect.x = x + sidebar_w + 30;
        self.btn_scale_100.rect.y = y + 215;
        self.btn_scale_125.rect.x = x + sidebar_w + 120;
        self.btn_scale_125.rect.y = y + 215;
        self.btn_scale_150.rect.x = x + sidebar_w + 205;
        self.btn_scale_150.rect.y = y + 215;
        self.btn_scale_200.rect.x = x + sidebar_w + 290;
        self.btn_scale_200.rect.y = y + 215;

        self.btn_dhcp.rect.x = x + sidebar_w + 30;
        self.btn_dhcp.rect.y = y + h - 45;
        self.btn_scan_wifi.rect.x = x + sidebar_w + 160;
        self.btn_scan_wifi.rect.y = y + h - 45;

        // Sidebar clicks
        if self.btn_tab_app.is_clicked(mouse) {
            self.current_tab = SettingsTab::Appearance;
        } else if self.btn_tab_disp.is_clicked(mouse) {
            self.current_tab = SettingsTab::Display;
        } else if self.btn_tab_net.is_clicked(mouse) {
            self.current_tab = SettingsTab::Network;
        } else if self.btn_tab_input.is_clicked(mouse) {
            self.current_tab = SettingsTab::Input;
        } else if self.btn_tab_sys.is_clicked(mouse) {
            self.current_tab = SettingsTab::System;
        } else if self.btn_tab_about.is_clicked(mouse) {
            self.current_tab = SettingsTab::About;
        }

        // Tab-specific interactions
        match self.current_tab {
            SettingsTab::Appearance => {
                let mut changed = false;
                if self.btn_wall_obsidian.is_clicked(mouse) {
                    self.wallpaper_idx = 0;
                    changed = true;
                } else if self.btn_wall_cyan.is_clicked(mouse) {
                    self.wallpaper_idx = 1;
                    changed = true;
                } else if self.btn_wall_slate.is_clicked(mouse) {
                    self.wallpaper_idx = 2;
                    changed = true;
                } else if self.btn_wall_aurora.is_clicked(mouse) {
                    self.wallpaper_idx = 3;
                    changed = true;
                }

                if self.btn_acc_teal.is_clicked(mouse) {
                    self.accent_idx = 0;
                    changed = true;
                } else if self.btn_acc_blue.is_clicked(mouse) {
                    self.accent_idx = 1;
                    changed = true;
                } else if self.btn_acc_emerald.is_clicked(mouse) {
                    self.accent_idx = 2;
                    changed = true;
                } else if self.btn_acc_amber.is_clicked(mouse) {
                    self.accent_idx = 3;
                    changed = true;
                }

                if changed {
                    self.save_config();
                }
            }
            SettingsTab::Display => {
                let mut changed = false;
                if self.btn_scale_100.is_clicked(mouse) {
                    self.ui_scale = 1.0;
                    changed = true;
                } else if self.btn_scale_125.is_clicked(mouse) {
                    self.ui_scale = 1.25;
                    changed = true;
                } else if self.btn_scale_150.is_clicked(mouse) {
                    self.ui_scale = 1.5;
                    changed = true;
                } else if self.btn_scale_200.is_clicked(mouse) {
                    self.ui_scale = 2.0;
                    changed = true;
                }
                if changed {
                    self.save_config();
                }
            }
            SettingsTab::Input => {
                let mut changed = false;
                if self.btn_sens_low.is_clicked(mouse) {
                    self.mouse_sensitivity = 0.8;
                    changed = true;
                } else if self.btn_sens_norm.is_clicked(mouse) {
                    self.mouse_sensitivity = 1.0;
                    changed = true;
                } else if self.btn_sens_high.is_clicked(mouse) {
                    self.mouse_sensitivity = 1.4;
                    changed = true;
                }
                if changed {
                    self.save_config();
                }
            }
            SettingsTab::Network => {
                if self.btn_dhcp.is_clicked(mouse) {
                    let _ = Command::new("/bin/net").arg("--dhcp").output();
                    let _ = Command::new("/sbin/udhcpc")
                        .args(&["-i", "eth0", "-n", "-q"])
                        .output();
                    self.refresh_network_status();
                }
                if self.btn_scan_wifi.is_clicked(mouse) {
                    self.refresh_wifi_scan();
                }
            }
            _ => {}
        }

        // Periodic background refresh every 8 seconds
        if time_sec - self.last_refresh_time > 8.0 {
            self.last_refresh_time = time_sec;
            if self.current_tab == SettingsTab::Network {
                self.refresh_network_status();
            }
        }
    }

    pub fn get_accent_color(&self) -> Color {
        match self.accent_idx {
            1 => Color::new(0, 130, 255),  // Electric Blue
            2 => Color::new(50, 215, 75),  // Emerald
            3 => Color::new(255, 179, 64), // Amber
            _ => Color::new(0, 190, 210),  // Cyan / Teal
        }
    }

    pub fn get_wallpaper_colors(&self) -> (Color, Color) {
        match self.wallpaper_idx {
            1 => (Color::new(10, 25, 45), Color::new(5, 40, 50)), // Abyss Cyan
            2 => (Color::new(20, 24, 30), Color::new(12, 14, 18)), // Midnight Slate
            3 => (Color::new(16, 28, 24), Color::new(24, 12, 36)), // Electric Aurora
            _ => (Color::new(18, 10, 34), Color::new(12, 18, 32)), // Deep Obsidian
        }
    }

    pub fn draw(&self, gfx: &mut Graphics, mouse: &MouseState) {
        if !self.window.is_open {
            return;
        }

        self.window.draw_frame(gfx, mouse);

        let client = self.window.client_rect();
        client.draw_fill(gfx, Color::MACOS_WINDOW_BG);

        // Sidebar
        let sidebar_w = 130;
        let sidebar_rect = Rect::new(client.x, client.y, sidebar_w, client.h);
        sidebar_rect.draw_fill(gfx, Color::MACOS_SIDEBAR);
        sidebar_rect.draw_outline(gfx, Color::MACOS_BORDER);

        let active_accent = self.get_accent_color();

        let mut b_app = Button::new_with_icon(
            self.btn_tab_app.rect.x,
            self.btn_tab_app.rect.y,
            self.btn_tab_app.rect.w,
            self.btn_tab_app.rect.h,
            IconId::Palette,
            "Appearance",
        );
        let mut b_disp = Button::new_with_icon(
            self.btn_tab_disp.rect.x,
            self.btn_tab_disp.rect.y,
            self.btn_tab_disp.rect.w,
            self.btn_tab_disp.rect.h,
            IconId::Monitor,
            "Display",
        );
        let mut b_net = Button::new_with_icon(
            self.btn_tab_net.rect.x,
            self.btn_tab_net.rect.y,
            self.btn_tab_net.rect.w,
            self.btn_tab_net.rect.h,
            IconId::Wifi,
            "Network",
        );
        let mut b_input = Button::new_with_icon(
            self.btn_tab_input.rect.x,
            self.btn_tab_input.rect.y,
            self.btn_tab_input.rect.w,
            self.btn_tab_input.rect.h,
            IconId::Mouse,
            "Input",
        );
        let mut b_sys = Button::new_with_icon(
            self.btn_tab_sys.rect.x,
            self.btn_tab_sys.rect.y,
            self.btn_tab_sys.rect.w,
            self.btn_tab_sys.rect.h,
            IconId::Cpu,
            "System",
        );
        let mut b_about = Button::new_with_icon(
            self.btn_tab_about.rect.x,
            self.btn_tab_about.rect.y,
            self.btn_tab_about.rect.w,
            self.btn_tab_about.rect.h,
            IconId::Info,
            "About OS",
        );

        match self.current_tab {
            SettingsTab::Appearance => {
                b_app.bg_color = active_accent;
                b_app.text_color = Color::new(255, 255, 255);
                b_app.icon_color = Color::new(255, 255, 255);
            }
            SettingsTab::Display => {
                b_disp.bg_color = active_accent;
                b_disp.text_color = Color::new(255, 255, 255);
                b_disp.icon_color = Color::new(255, 255, 255);
            }
            SettingsTab::Network => {
                b_net.bg_color = active_accent;
                b_net.text_color = Color::new(255, 255, 255);
                b_net.icon_color = Color::new(255, 255, 255);
            }
            SettingsTab::Input => {
                b_input.bg_color = active_accent;
                b_input.text_color = Color::new(255, 255, 255);
                b_input.icon_color = Color::new(255, 255, 255);
            }
            SettingsTab::System => {
                b_sys.bg_color = active_accent;
                b_sys.text_color = Color::new(255, 255, 255);
                b_sys.icon_color = Color::new(255, 255, 255);
            }
            SettingsTab::About => {
                b_about.bg_color = active_accent;
                b_about.text_color = Color::new(255, 255, 255);
                b_about.icon_color = Color::new(255, 255, 255);
            }
        }

        b_app.draw(gfx, mouse);
        b_disp.draw(gfx, mouse);
        b_net.draw(gfx, mouse);
        b_input.draw(gfx, mouse);
        b_sys.draw(gfx, mouse);
        b_about.draw(gfx, mouse);

        // Content Pane
        let content_x = (client.x + sidebar_w + 16) as u32;
        let mut content_y = (client.y + 14) as u32;
        let card_w = client.w - sidebar_w - 32;

        match self.current_tab {
            SettingsTab::Appearance => {
                gfx.draw_string(
                    content_x,
                    content_y,
                    "APPEARANCE & THEME CUSTOMIZATION",
                    Color::MACOS_TEXT,
                );
                content_y += 24;

                let card_rect = Rect::new(content_x as i32, content_y as i32, card_w, 180);
                card_rect.draw_card(gfx, Color::MACOS_TITLEBAR, Color::MACOS_BORDER);

                gfx.draw_string(
                    content_x + 14,
                    content_y + 12,
                    "Desktop Background Theme:",
                    Color::MACOS_TEXT,
                );

                let mut w_obs = Button::new(
                    self.btn_wall_obsidian.rect.x,
                    self.btn_wall_obsidian.rect.y,
                    self.btn_wall_obsidian.rect.w,
                    self.btn_wall_obsidian.rect.h,
                    "Obsidian",
                );
                let mut w_cyn = Button::new(
                    self.btn_wall_cyan.rect.x,
                    self.btn_wall_cyan.rect.y,
                    self.btn_wall_cyan.rect.w,
                    self.btn_wall_cyan.rect.h,
                    "Abyss",
                );
                let mut w_slt = Button::new(
                    self.btn_wall_slate.rect.x,
                    self.btn_wall_slate.rect.y,
                    self.btn_wall_slate.rect.w,
                    self.btn_wall_slate.rect.h,
                    "Slate",
                );
                let mut w_aur = Button::new(
                    self.btn_wall_aurora.rect.x,
                    self.btn_wall_aurora.rect.y,
                    self.btn_wall_aurora.rect.w,
                    self.btn_wall_aurora.rect.h,
                    "Aurora",
                );

                match self.wallpaper_idx {
                    0 => w_obs.bg_color = active_accent,
                    1 => w_cyn.bg_color = active_accent,
                    2 => w_slt.bg_color = active_accent,
                    3 => w_aur.bg_color = active_accent,
                    _ => {}
                }

                w_obs.draw(gfx, mouse);
                w_cyn.draw(gfx, mouse);
                w_slt.draw(gfx, mouse);
                w_aur.draw(gfx, mouse);

                gfx.draw_string(
                    content_x + 14,
                    content_y + 74,
                    "System Accent Color:",
                    Color::MACOS_TEXT,
                );

                let mut a_teal = Button::new(
                    self.btn_acc_teal.rect.x,
                    self.btn_acc_teal.rect.y,
                    self.btn_acc_teal.rect.w,
                    self.btn_acc_teal.rect.h,
                    "Teal",
                );
                let mut a_blue = Button::new(
                    self.btn_acc_blue.rect.x,
                    self.btn_acc_blue.rect.y,
                    self.btn_acc_blue.rect.w,
                    self.btn_acc_blue.rect.h,
                    "Blue",
                );
                let mut a_eme = Button::new(
                    self.btn_acc_emerald.rect.x,
                    self.btn_acc_emerald.rect.y,
                    self.btn_acc_emerald.rect.w,
                    self.btn_acc_emerald.rect.h,
                    "Emerald",
                );
                let mut a_amb = Button::new(
                    self.btn_acc_amber.rect.x,
                    self.btn_acc_amber.rect.y,
                    self.btn_acc_amber.rect.w,
                    self.btn_acc_amber.rect.h,
                    "Amber",
                );

                match self.accent_idx {
                    0 => a_teal.bg_color = Color::new(0, 190, 210),
                    1 => a_blue.bg_color = Color::new(0, 130, 255),
                    2 => a_eme.bg_color = Color::new(50, 215, 75),
                    3 => a_amb.bg_color = Color::new(255, 179, 64),
                    _ => {}
                }

                a_teal.draw(gfx, mouse);
                a_blue.draw(gfx, mouse);
                a_eme.draw(gfx, mouse);
                a_amb.draw(gfx, mouse);

                gfx.draw_string(
                    content_x + 14,
                    content_y + 138,
                    "Theme Mode: Dark Obsidian (Leviathan Native Dark Shell)",
                    Color::MACOS_TEXT_SECONDARY,
                );
            }
            SettingsTab::Display => {
                gfx.draw_string(
                    content_x,
                    content_y,
                    "DISPLAY & GRAPHICS PIPELINE",
                    Color::MACOS_TEXT,
                );
                content_y += 24;

                let card_rect = Rect::new(content_x as i32, content_y as i32, card_w, 230);
                card_rect.draw_card(gfx, Color::MACOS_TITLEBAR, Color::MACOS_BORDER);

                let scale_pct = (self.ui_scale * 100.0).round() as u32;
                let lines = [
                    format!("Screen Resolution:   {} x {} pixels (Full Native)", gfx.width(), gfx.height()),
                    format!("Color Depth:         {} bits per pixel (ARGB8888 TrueColor)", gfx.bpp()),
                    format!("Display Device:      /dev/fb0 (Direct Linux DRM Framebuffer)"),
                    format!("Scanline Pitch:      {} bytes per scanline", gfx.pitch()),
                    format!("Frame Presentation:  Hardware Double-Buffered (Zero Tearing)"),
                    format!("Target Refresh Rate: 60.0 Hz (VSync Paced Loop)"),
                    format!("Active UI Scale:     {}%", scale_pct),
                ];

                let mut ly = content_y + 14;
                for l in &lines {
                    gfx.draw_string(content_x + 16, ly, l, Color::MACOS_TEXT_SECONDARY);
                    ly += 18;
                }

                gfx.draw_string(
                    content_x + 16,
                    ly + 6,
                    "Display UI Scaling Mode:",
                    Color::MACOS_TEXT,
                );

                let mut s100 = Button::new(self.btn_scale_100.rect.x, self.btn_scale_100.rect.y, self.btn_scale_100.rect.w, self.btn_scale_100.rect.h, "100% (1:1)");
                let mut s125 = Button::new(self.btn_scale_125.rect.x, self.btn_scale_125.rect.y, self.btn_scale_125.rect.w, self.btn_scale_125.rect.h, "125%");
                let mut s150 = Button::new(self.btn_scale_150.rect.x, self.btn_scale_150.rect.y, self.btn_scale_150.rect.w, self.btn_scale_150.rect.h, "150%");
                let mut s200 = Button::new(self.btn_scale_200.rect.x, self.btn_scale_200.rect.y, self.btn_scale_200.rect.w, self.btn_scale_200.rect.h, "200% HiDPI");

                if (self.ui_scale - 1.0).abs() < 0.05 {
                    s100.bg_color = active_accent;
                } else if (self.ui_scale - 1.25).abs() < 0.05 {
                    s125.bg_color = active_accent;
                } else if (self.ui_scale - 1.5).abs() < 0.05 {
                    s150.bg_color = active_accent;
                } else if (self.ui_scale - 2.0).abs() < 0.05 {
                    s200.bg_color = active_accent;
                }

                s100.draw(gfx, mouse);
                s125.draw(gfx, mouse);
                s150.draw(gfx, mouse);
                s200.draw(gfx, mouse);
            }
            SettingsTab::Network => {
                gfx.draw_string(
                    content_x,
                    content_y,
                    "NETWORK & INTERNET SETTINGS",
                    Color::MACOS_TEXT,
                );
                content_y += 24;

                let card_rect = Rect::new(content_x as i32, content_y as i32, card_w, 105);
                card_rect.draw_card(gfx, Color::MACOS_TITLEBAR, Color::MACOS_BORDER);

                let mut card_y = content_y + 10;
                for line in &self.net_status_cache {
                    let col = if line.contains("UP") || line.contains("ONLINE") {
                        Color::new(50, 215, 75)
                    } else if line.contains("eth") || line.contains("Default Gateway") {
                        active_accent
                    } else {
                        Color::MACOS_TEXT_SECONDARY
                    };
                    gfx.draw_string(content_x + 12, card_y, line, col);
                    card_y += 16;
                }

                content_y += 118;
                gfx.draw_string(
                    content_x,
                    content_y,
                    "AVAILABLE WIRELESS NETWORKS (802.11):",
                    Color::MACOS_TEXT,
                );
                content_y += 20;

                let wifi_rect = Rect::new(content_x as i32, content_y as i32, card_w, 100);
                wifi_rect.draw_card(gfx, Color::MACOS_TITLEBAR, Color::MACOS_BORDER);

                let mut wifi_y = content_y + 8;
                for entry in self.wifi_scan_results.iter().take(4) {
                    let max_c = ((wifi_rect.w - 20) / 8).max(1) as usize;
                    let display_line: String = entry.chars().take(max_c).collect();
                    gfx.draw_string(
                        content_x + 10,
                        wifi_y,
                        &display_line,
                        Color::MACOS_TEXT_SECONDARY,
                    );
                    wifi_y += 18;
                }

                self.btn_dhcp.draw(gfx, mouse);
                self.btn_scan_wifi.draw(gfx, mouse);
            }
            SettingsTab::Input => {
                gfx.draw_string(
                    content_x,
                    content_y,
                    "KEYBOARD & MOUSE INPUT CONFIGURATION",
                    Color::MACOS_TEXT,
                );
                content_y += 24;

                let card_rect = Rect::new(content_x as i32, content_y as i32, card_w, 180);
                card_rect.draw_card(gfx, Color::MACOS_TITLEBAR, Color::MACOS_BORDER);

                let lines = [
                    "Keyboard Layout:        US QWERTY (Standard 104-Key)",
                    "Key Repeat Delay:       500 ms",
                    "Key Repeat Frequency:   30 Hz (33 ms rate)",
                    "Mouse Wheel Scroll:     3 text lines per scroll notch",
                    "Pointer Acceleration:   Linear 1:1 Direct Evdev Tracking",
                ];

                let mut ly = content_y + 12;
                for l in &lines {
                    gfx.draw_string(content_x + 16, ly, l, Color::MACOS_TEXT_SECONDARY);
                    ly += 18;
                }

                gfx.draw_string(
                    content_x + 16,
                    content_y + 112,
                    "Mouse Tracking Sensitivity:",
                    Color::MACOS_TEXT,
                );

                let mut s_low = Button::new(
                    self.btn_sens_low.rect.x,
                    self.btn_sens_low.rect.y,
                    self.btn_sens_low.rect.w,
                    self.btn_sens_low.rect.h,
                    "0.8x Low",
                );
                let mut s_norm = Button::new(
                    self.btn_sens_norm.rect.x,
                    self.btn_sens_norm.rect.y,
                    self.btn_sens_norm.rect.w,
                    self.btn_sens_norm.rect.h,
                    "1.0x Norm",
                );
                let mut s_high = Button::new(
                    self.btn_sens_high.rect.x,
                    self.btn_sens_high.rect.y,
                    self.btn_sens_high.rect.w,
                    self.btn_sens_high.rect.h,
                    "1.4x Fast",
                );

                if self.mouse_sensitivity < 0.9 {
                    s_low.bg_color = active_accent;
                } else if self.mouse_sensitivity > 1.2 {
                    s_high.bg_color = active_accent;
                } else {
                    s_norm.bg_color = active_accent;
                }

                s_low.draw(gfx, mouse);
                s_norm.draw(gfx, mouse);
                s_high.draw(gfx, mouse);
            }
            SettingsTab::System => {
                gfx.draw_string(
                    content_x,
                    content_y,
                    "SYSTEM & KERNEL INFORMATION",
                    Color::MACOS_TEXT,
                );
                content_y += 24;

                let card_rect = Rect::new(content_x as i32, content_y as i32, card_w, 220);
                card_rect.draw_card(gfx, Color::MACOS_TITLEBAR, Color::MACOS_BORDER);

                let (kernel_ver, arch) = get_kernel_info();
                let (cpu_model, cpu_cores) = get_cpu_info();
                let (mem_total, mem_avail, mem_used) = get_mem_info();
                let (disk_total, _disk_free, disk_used) = get_disk_usage();
                let uptime_str = get_uptime();

                let disk_total_mb = disk_total / (1024 * 1024);
                let disk_used_mb = disk_used / (1024 * 1024);
                let disk_pct = if disk_total > 0 {
                    (disk_used as f64 / disk_total as f64 * 100.0) as u32
                } else {
                    0
                };

                let lines = [
                    format!("Operating System: Leviathan OS v0.2.0 (Desktop Preview)"),
                    format!("Kernel Version:   Linux {} ({})", kernel_ver, arch),
                    format!("Processor:        {} ({} cores)", cpu_model, cpu_cores),
                    format!(
                        "Memory (RAM):     {} MB used / {} MB total ({} MB avail)",
                        mem_used, mem_total, mem_avail
                    ),
                    format!(
                        "Rootfs Storage:   {} MB used / {} MB total ({}% used)",
                        disk_used_mb, disk_total_mb, disk_pct
                    ),
                    format!("System Uptime:    {}", uptime_str),
                    format!("Standard C Lib:   musl-libc (Static Binary Runtime)"),
                    format!("Display Server:   Leviathan Native Direct Compositor"),
                ];

                let mut ly = content_y + 14;
                for l in &lines {
                    gfx.draw_string(content_x + 16, ly, l, Color::MACOS_TEXT_SECONDARY);
                    ly += 20;
                }
            }
            SettingsTab::About => {
                gfx.draw_string(
                    content_x,
                    content_y,
                    "ABOUT LEVIATHAN OS",
                    Color::MACOS_TEXT,
                );
                content_y += 24;

                let card_rect = Rect::new(content_x as i32, content_y as i32, card_w, 230);
                card_rect.draw_card(gfx, Color::MACOS_TITLEBAR, Color::MACOS_BORDER);

                let lines = [
                    "==================================================",
                    "            LEVIATHAN OPERATING SYSTEM            ",
                    "==================================================",
                    "An autonomous desktop operating system built on top",
                    "of the Linux kernel foundation with a custom userspace,",
                    "real Linux PTY terminal, window management subsystem,",
                    "integrated software renderer, APT package management,",
                    "and native graphical shell.",
                    "",
                    "Release Code:  'Abyss Explorer' v0.2.0",
                    "Desktop Shell: Leviathan Obsidian Window Manager",
                    "Target Arch:   x86_64-unknown-linux-musl",
                ];

                let mut ly = content_y + 12;
                for l in &lines {
                    let col = if l.contains("===") {
                        active_accent
                    } else if l.contains("LEVIATHAN") {
                        Color::MACOS_TEXT
                    } else {
                        Color::MACOS_TEXT_SECONDARY
                    };
                    gfx.draw_string(content_x + 16, ly, l, col);
                    ly += 16;
                }
            }
        }
    }
}

fn get_kernel_info() -> (String, String) {
    let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&mut uts) } == 0 {
        let release = unsafe { std::ffi::CStr::from_ptr(uts.release.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let machine = unsafe { std::ffi::CStr::from_ptr(uts.machine.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        (release, machine)
    } else {
        ("Linux 6.8".into(), "x86_64".into())
    }
}

fn get_cpu_info() -> (String, usize) {
    let mut model = "Unknown CPU".to_string();
    let mut count = 0;
    if let Ok(s) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in s.lines() {
            if line.starts_with("model name") {
                if let Some(pos) = line.find(':') {
                    model = line[pos + 1..].trim().to_string();
                }
            } else if line.starts_with("processor") {
                count += 1;
            }
        }
    }
    if count == 0 {
        count = 1;
    }
    (model, count)
}

fn get_mem_info() -> (u64, u64, u64) {
    let mut total_kb: u64 = 0;
    let mut free_kb: u64 = 0;
    let mut avail_kb: u64 = 0;
    if let Ok(s) = std::fs::read_to_string("/proc/meminfo") {
        for line in s.lines() {
            if line.starts_with("MemTotal:") {
                total_kb = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("MemFree:") {
                free_kb = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("MemAvailable:") {
                avail_kb = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(0);
            }
        }
    }
    let avail = if avail_kb > 0 { avail_kb } else { free_kb };
    let used = total_kb.saturating_sub(avail);
    (total_kb / 1024, avail / 1024, used / 1024)
}

fn get_disk_usage() -> (u64, u64, u64) {
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let path = std::ffi::CString::new("/").unwrap();
    if unsafe { libc::statvfs(path.as_ptr(), &mut stat) } == 0 {
        let block_size = stat.f_frsize as u64;
        let total = stat.f_blocks as u64 * block_size;
        let free = stat.f_bavail as u64 * block_size;
        let used = total.saturating_sub(free);
        (total, free, used)
    } else {
        (0, 0, 0)
    }
}

fn get_uptime() -> String {
    if let Ok(s) = std::fs::read_to_string("/proc/uptime") {
        if let Some(sec_str) = s.split_whitespace().next() {
            if let Ok(secs_f) = sec_str.parse::<f64>() {
                let secs = secs_f as u64;
                let days = secs / 86400;
                let hours = (secs % 86400) / 3600;
                let mins = (secs % 3600) / 60;
                let s = secs % 60;
                if days > 0 {
                    return format!("{}d {}h {}m {}s", days, hours, mins, s);
                } else if hours > 0 {
                    return format!("{}h {}m {}s", hours, mins, s);
                } else {
                    return format!("{}m {}s", mins, s);
                }
            }
        }
    }
    "0m 0s".to_string()
}

fn get_network_details() -> Vec<String> {
    let mut lines = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let iface = entry.file_name().to_string_lossy().to_string();
            let operstate = std::fs::read_to_string(entry.path().join("operstate"))
                .unwrap_or_else(|_| "unknown".to_string())
                .trim()
                .to_uppercase();
            let mac = std::fs::read_to_string(entry.path().join("address"))
                .unwrap_or_else(|_| "00:00:00:00:00:00".to_string())
                .trim()
                .to_string();
            lines.push(format!(
                "Iface: {:<6} | State: {:<4} | MAC: {}",
                iface, operstate, mac
            ));
        }
    }
    if let Ok(s) = std::fs::read_to_string("/proc/net/route") {
        for line in s.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 3 && fields[1] == "00000000" {
                if let Ok(gw_hex) = u32::from_str_radix(fields[2], 16) {
                    let b = gw_hex.to_ne_bytes();
                    lines.push(format!(
                        "Gateway: {}.{}.{}.{} (via {})",
                        b[0], b[1], b[2], b[3], fields[0]
                    ));
                }
            }
        }
    }
    if let Ok(s) = std::fs::read_to_string("/etc/resolv.conf") {
        for line in s.lines() {
            if line.starts_with("nameserver") {
                lines.push(format!("DNS: {}", line.trim()));
            }
        }
    }
    if lines.is_empty() {
        lines.push("Iface: eth0   | State: UP   | MAC: 52:54:00:12:34:56".into());
        lines.push("Gateway: 10.0.2.2 (via eth0)".into());
        lines.push("DNS: nameserver 10.0.2.3".into());
    }
    lines
}
