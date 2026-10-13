use crate::color::Color;
use crate::graphics::Graphics;
use crate::icons::{draw_icon, IconId};
use crate::mouse::MouseState;
use crate::package::{PackageInfo, PackageManager};
use crate::widget::{Button, Rect, Window};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PackageTab {
    All,
    Applications,
    Tools,
    Internet,
    Installed,
    Updates,
}

#[derive(Debug, Clone)]
pub enum OpStatus {
    Idle,
    Working(String),
    Success(String),
    Error(String),
}

pub struct PackagesApp {
    pub window: Window,
    pub current_tab: PackageTab,
    pub search_query: String,
    pub search_focused: bool,

    // Category buttons
    btn_tab_all: Button,
    btn_tab_apps: Button,
    btn_tab_tools: Button,
    btn_tab_net: Button,
    btn_tab_inst: Button,
    btn_tab_upd: Button,

    // Action buttons
    btn_update_all: Button,
    btn_refresh: Button,
    btn_toggle_details: Button,
    btn_prev: Button,
    btn_next: Button,

    // Package list & cache
    packages: Vec<PackageInfo>,
    scroll_offset: usize,
    pub packages_changed: bool,

    // Asynchronous operation channels
    op_status: Arc<Mutex<OpStatus>>,
    op_log: Arc<Mutex<Vec<String>>>,
    show_details: bool,

    // Channel for background job triggers
    job_tx: Sender<JobRequest>,
    result_rx: Receiver<JobResult>,
}

enum JobRequest {
    Install(String),
    Remove(String),
    UpdateLists,
    UpgradeAll,
}

enum JobResult {
    Finished(bool, String),
    LogLine(String),
}

impl PackagesApp {
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        let btn_tab_all = Button::new_with_icon(x + 16, y + 68, 64, 24, IconId::AllApps, "All");
        let btn_tab_apps = Button::new_with_icon(x + 86, y + 68, 86, 24, IconId::Monitor, "Apps");
        let btn_tab_tools = Button::new_with_icon(x + 178, y + 68, 86, 24, IconId::Terminal, "Tools");
        let btn_tab_net = Button::new_with_icon(x + 270, y + 68, 96, 24, IconId::Globe, "Internet");
        let btn_tab_inst = Button::new_with_icon(x + 372, y + 68, 100, 24, IconId::Check, "Installed");
        let btn_tab_upd = Button::new_with_icon(x + 478, y + 68, 96, 24, IconId::Download, "Updates");

        let btn_update_all = Button::new_with_icon(x + (w as i32) - 130, y + 68, 110, 24, IconId::Download, "Update All");
        let btn_refresh = Button::new_with_icon(x + (w as i32) - 95, y + 34, 78, 26, IconId::Refresh, "Refresh");
        let btn_toggle_details = Button::new_with_icon(x + (w as i32) - 95, y + (h as i32) - 34, 80, 22, IconId::Info, "Details");
        let btn_prev = Button::new_icon(x + (w as i32) - 170, y + (h as i32) - 34, 32, 22, IconId::ChevronLeft);
        let btn_next = Button::new_icon(x + (w as i32) - 134, y + (h as i32) - 34, 32, 22, IconId::ChevronRight);

        let (job_tx, job_rx) = channel::<JobRequest>();
        let (result_tx, result_rx) = channel::<JobResult>();

        let op_status = Arc::new(Mutex::new(OpStatus::Idle));
        let op_log = Arc::new(Mutex::new(Vec::new()));

        // Worker thread for non-blocking APT/dpkg executions
        let worker_status = Arc::clone(&op_status);
        let worker_log = Arc::clone(&op_log);
        thread::spawn(move || {
            while let Ok(job) = job_rx.recv() {
                match job {
                    JobRequest::Install(pkg_name) => {
                        *worker_status.lock().unwrap() =
                            OpStatus::Working(format!("Installing {}...", pkg_name));
                        let rtx = result_tx.clone();
                        let wlog = Arc::clone(&worker_log);

                        let res = PackageManager::install(&pkg_name, |line| {
                            wlog.lock().unwrap().push(line.to_string());
                            let _ = rtx.send(JobResult::LogLine(line.to_string()));
                        });

                        match res {
                            Ok(()) => {
                                *worker_status.lock().unwrap() =
                                    OpStatus::Success(format!("{} installed successfully.", pkg_name));
                                let _ = result_tx.send(JobResult::Finished(true, pkg_name));
                            }
                            Err(e) => {
                                *worker_status.lock().unwrap() =
                                    OpStatus::Error(format!("Install failed: {}", e));
                                let _ = result_tx.send(JobResult::Finished(false, e));
                            }
                        }
                    }
                    JobRequest::Remove(pkg_name) => {
                        *worker_status.lock().unwrap() =
                            OpStatus::Working(format!("Removing {}...", pkg_name));
                        let rtx = result_tx.clone();
                        let wlog = Arc::clone(&worker_log);

                        let res = PackageManager::remove(&pkg_name, |line| {
                            wlog.lock().unwrap().push(line.to_string());
                            let _ = rtx.send(JobResult::LogLine(line.to_string()));
                        });

                        match res {
                            Ok(()) => {
                                *worker_status.lock().unwrap() =
                                    OpStatus::Success(format!("{} removed successfully.", pkg_name));
                                let _ = result_tx.send(JobResult::Finished(true, pkg_name));
                            }
                            Err(e) => {
                                *worker_status.lock().unwrap() =
                                    OpStatus::Error(format!("Remove failed: {}", e));
                                let _ = result_tx.send(JobResult::Finished(false, e));
                            }
                        }
                    }
                    JobRequest::UpdateLists => {
                        *worker_status.lock().unwrap() =
                            OpStatus::Working("Synchronizing package lists...".into());
                        let rtx = result_tx.clone();
                        let wlog = Arc::clone(&worker_log);

                        let res = PackageManager::update(|line| {
                            wlog.lock().unwrap().push(line.to_string());
                            let _ = rtx.send(JobResult::LogLine(line.to_string()));
                        });

                        match res {
                            Ok(()) => {
                                *worker_status.lock().unwrap() =
                                    OpStatus::Success("Repositories up to date.".into());
                                let _ = result_tx.send(JobResult::Finished(true, "update".into()));
                            }
                            Err(e) => {
                                *worker_status.lock().unwrap() =
                                    OpStatus::Error(format!("Update failed: {}", e));
                                let _ = result_tx.send(JobResult::Finished(false, e));
                            }
                        }
                    }
                    JobRequest::UpgradeAll => {
                        *worker_status.lock().unwrap() =
                            OpStatus::Working("Upgrading packages...".into());
                        let rtx = result_tx.clone();
                        let wlog = Arc::clone(&worker_log);

                        let res = PackageManager::upgrade(|line| {
                            wlog.lock().unwrap().push(line.to_string());
                            let _ = rtx.send(JobResult::LogLine(line.to_string()));
                        });

                        match res {
                            Ok(()) => {
                                *worker_status.lock().unwrap() =
                                    OpStatus::Success("System upgraded.".into());
                                let _ = result_tx.send(JobResult::Finished(true, "upgrade".into()));
                            }
                            Err(e) => {
                                *worker_status.lock().unwrap() =
                                    OpStatus::Error(format!("Upgrade failed: {}", e));
                                let _ = result_tx.send(JobResult::Finished(false, e));
                            }
                        }
                    }
                }
            }
        });

        let mut app = Self {
            window: Window::new("Packages - Leviathan OS", x, y, w as i32, h as i32),
            current_tab: PackageTab::All,
            search_query: String::new(),
            search_focused: false,
            btn_tab_all,
            btn_tab_apps,
            btn_tab_tools,
            btn_tab_net,
            btn_tab_inst,
            btn_tab_upd,
            btn_update_all,
            btn_refresh,
            btn_toggle_details,
            btn_prev,
            btn_next,
            packages: Vec::new(),
            packages_changed: false,
            scroll_offset: 0,
            op_status,
            op_log,
            show_details: false,
            job_tx,
            result_rx,
        };

        app.reload_packages();
        app
    }

    pub fn reload_packages(&mut self) {
        self.packages = PackageManager::search(&self.search_query);
    }

    pub fn handle_key(&mut self, ch: char) {
        if !self.window.is_open || !self.window.is_focused {
            return;
        }

        if ch == '\x08' {
            // Backspace
            self.search_query.pop();
            self.scroll_offset = 0;
            self.reload_packages();
        } else if ch == '\n' || ch == '\r' {
            self.scroll_offset = 0;
            self.reload_packages();
        } else if ch == '\x1b' {
            // Escape clears search
            self.search_query.clear();
            self.scroll_offset = 0;
            self.reload_packages();
        } else if !ch.is_control() {
            self.search_query.push(ch);
            self.scroll_offset = 0;
            self.reload_packages();
        }
    }

    pub fn update(&mut self, mouse: &MouseState, screen_w: i32, screen_h: i32) {
        self.window.update_drag(mouse, screen_w, screen_h);
        if !self.window.is_open {
            return;
        }

        // Check background results
        while let Ok(res) = self.result_rx.try_recv() {
            if let JobResult::Finished(true, _) = res {
                self.reload_packages();
                self.packages_changed = true;
            }
        }

        let wx = self.window.rect.x;
        let wy = self.window.rect.y;
        let ww = self.window.rect.w;
        let wh = self.window.rect.h;

        // Position tabs dynamically relative to window
        self.btn_tab_all.rect.x = wx + 16;
        self.btn_tab_all.rect.y = wy + 68;
        self.btn_tab_apps.rect.x = wx + 86;
        self.btn_tab_apps.rect.y = wy + 68;
        self.btn_tab_tools.rect.x = wx + 178;
        self.btn_tab_tools.rect.y = wy + 68;
        self.btn_tab_net.rect.x = wx + 270;
        self.btn_tab_net.rect.y = wy + 68;
        self.btn_tab_inst.rect.x = wx + 372;
        self.btn_tab_inst.rect.y = wy + 68;
        self.btn_tab_upd.rect.x = wx + 478;
        self.btn_tab_upd.rect.y = wy + 68;

        self.btn_refresh.rect.x = wx + ww - 95;
        self.btn_refresh.rect.y = wy + 34;

        self.btn_update_all.rect.x = wx + ww - 130;
        self.btn_update_all.rect.y = wy + 68;

        self.btn_toggle_details.rect.x = wx + ww - 95;
        self.btn_toggle_details.rect.y = wy + wh - 30;

        self.btn_next.rect.x = wx + ww - 134;
        self.btn_next.rect.y = wy + wh - 30;

        self.btn_prev.rect.x = wx + ww - 170;
        self.btn_prev.rect.y = wy + wh - 30;

        // Search bar click focus
        let search_rect = Rect::new(wx + 16, wy + 34, ww - 120, 26);
        if mouse.left_clicked {
            self.search_focused = search_rect.contains(mouse.x, mouse.y);
        }

        // Tab selection
        let old_tab = self.current_tab;
        if self.btn_tab_all.is_clicked(mouse) {
            self.current_tab = PackageTab::All;
        } else if self.btn_tab_apps.is_clicked(mouse) {
            self.current_tab = PackageTab::Applications;
        } else if self.btn_tab_tools.is_clicked(mouse) {
            self.current_tab = PackageTab::Tools;
        } else if self.btn_tab_net.is_clicked(mouse) {
            self.current_tab = PackageTab::Internet;
        } else if self.btn_tab_inst.is_clicked(mouse) {
            self.current_tab = PackageTab::Installed;
        } else if self.btn_tab_upd.is_clicked(mouse) {
            self.current_tab = PackageTab::Updates;
        }
        if self.current_tab != old_tab {
            self.scroll_offset = 0;
        }

        if self.btn_refresh.is_clicked(mouse) {
            let _ = self.job_tx.send(JobRequest::UpdateLists);
        }

        if self.btn_update_all.is_clicked(mouse) {
            let _ = self.job_tx.send(JobRequest::UpgradeAll);
        }

        if self.btn_toggle_details.is_clicked(mouse) {
            self.show_details = !self.show_details;
        }

        // Package cards interaction
        let card_w = ww - 32;
        let card_h = 56;
        let start_y = wy + 104;

        let filtered_indices: Vec<usize> = self
            .packages
            .iter()
            .enumerate()
            .filter(|(_, p)| match self.current_tab {
                PackageTab::All => true,
                PackageTab::Applications => p.category == "Applications",
                PackageTab::Tools => p.category == "Tools" || p.category == "Development",
                PackageTab::Internet => p.category == "Internet",
                PackageTab::Installed => p.is_installed,
                PackageTab::Updates => p.is_upgradable,
            })
            .map(|(i, _)| i)
            .collect();

        let total_count = filtered_indices.len();
        if self.scroll_offset >= total_count && total_count > 0 {
            self.scroll_offset = (total_count - 1) / 5 * 5;
        }

        if self.btn_prev.is_clicked(mouse) {
            self.scroll_offset = self.scroll_offset.saturating_sub(5);
        }
        if self.btn_next.is_clicked(mouse) {
            if self.scroll_offset + 5 < total_count {
                self.scroll_offset += 5;
            }
        }

        for (slot, &idx) in filtered_indices.iter().skip(self.scroll_offset).enumerate().take(5) {
            let cy = start_y + (slot as i32) * (card_h + 8);
            let action_btn_rect = Rect::new(wx + card_w - 90, cy + 14, 80, 26);

            if mouse.left_clicked && action_btn_rect.contains(mouse.x, mouse.y) {
                let pkg = &self.packages[idx];
                if pkg.is_installed {
                    // Remove
                    let _ = self.job_tx.send(JobRequest::Remove(pkg.name.clone()));
                } else {
                    // Install
                    let _ = self.job_tx.send(JobRequest::Install(pkg.name.clone()));
                }
            }
        }
    }

    pub fn draw(&mut self, gfx: &mut Graphics, mouse: &MouseState) {
        if !self.window.is_open {
            return;
        }

        self.window.draw_frame(gfx, mouse);

        let wx = self.window.rect.x;
        let wy = self.window.rect.y;
        let ww = self.window.rect.w;
        let wh = self.window.rect.h;

        // 1. Search Box
        let search_rect = Rect::new(wx + 16, wy + 34, ww - 120, 26);
        let search_bg = if self.search_focused {
            Color::MACOS_WINDOW_BG
        } else {
            Color::MACOS_TITLEBAR
        };
        let search_border = if self.search_focused {
            Color::MACOS_BORDER_ACTIVE
        } else {
            Color::MACOS_BORDER
        };
        search_rect.draw_fill(gfx, search_bg);
        search_rect.draw_outline(gfx, search_border);

        draw_icon(gfx, IconId::Search, wx + 24, wy + 40, 14, Color::MACOS_TEXT_SECONDARY);

        let search_text = if self.search_query.is_empty() {
            if self.search_focused {
                "_"
            } else {
                "Search packages (e.g. firefox, htop)..."
            }
        } else {
            &self.search_query
        };
        let search_col = if self.search_query.is_empty() && !self.search_focused {
            Color::MACOS_TEXT_SECONDARY
        } else {
            Color::MACOS_TEXT
        };
        gfx.draw_string((wx + 44) as u32, (wy + 42) as u32, search_text, search_col);

        self.btn_refresh.draw(gfx, mouse);

        // 2. Category Tabs
        let tabs = [
            (&mut self.btn_tab_all, self.current_tab == PackageTab::All),
            (&mut self.btn_tab_apps, self.current_tab == PackageTab::Applications),
            (&mut self.btn_tab_tools, self.current_tab == PackageTab::Tools),
            (&mut self.btn_tab_net, self.current_tab == PackageTab::Internet),
            (&mut self.btn_tab_inst, self.current_tab == PackageTab::Installed),
            (&mut self.btn_tab_upd, self.current_tab == PackageTab::Updates),
        ];

        for (btn, is_active) in tabs {
            if is_active {
                btn.bg_color = Color::MACOS_ACCENT;
                btn.border_color = Color::MACOS_BORDER_ACTIVE;
                btn.text_color = Color::new(255, 255, 255);
                btn.icon_color = Color::new(255, 255, 255);
            } else {
                btn.bg_color = Color::MACOS_HOVER;
                btn.border_color = Color::MACOS_BORDER;
                btn.text_color = Color::MACOS_TEXT;
                btn.icon_color = Color::MACOS_TEXT;
            }
            btn.draw(gfx, mouse);
        }

        if self.current_tab == PackageTab::Updates {
            self.btn_update_all.draw(gfx, mouse);
        }

        // 3. Package Cards List
        let card_w = ww - 32;
        let card_h = 56;
        let start_y = wy + 104;

        let filtered_pkgs: Vec<&PackageInfo> = self
            .packages
            .iter()
            .filter(|p| match self.current_tab {
                PackageTab::All => true,
                PackageTab::Applications => p.category == "Applications",
                PackageTab::Tools => p.category == "Tools" || p.category == "Development",
                PackageTab::Internet => p.category == "Internet",
                PackageTab::Installed => p.is_installed,
                PackageTab::Updates => p.is_upgradable,
            })
            .collect();

        let total_pkgs = filtered_pkgs.len();

        if filtered_pkgs.is_empty() {
            let empty_rect = Rect::new(wx + 16, start_y, card_w, 180);
            empty_rect.draw_fill(gfx, Color::MACOS_WINDOW_BG);
            empty_rect.draw_outline(gfx, Color::MACOS_BORDER);
            gfx.draw_string(
                (wx + (card_w / 2) - 100) as u32,
                (start_y + 80) as u32,
                "No packages found in this view",
                Color::MACOS_TEXT_SECONDARY,
            );
        } else {
            for (slot, pkg) in filtered_pkgs.iter().skip(self.scroll_offset).enumerate().take(5) {
                let cy = start_y + (slot as i32) * (card_h + 8);
                let card_rect = Rect::new(wx + 16, cy, card_w, card_h);
                let is_hover = card_rect.contains(mouse.x, mouse.y);

                let bg = if is_hover {
                    Color::MACOS_HOVER
                } else {
                    Color::MACOS_WINDOW_BG
                };
                let border = if is_hover {
                    Color::MACOS_BORDER_ACTIVE
                } else {
                    Color::MACOS_BORDER
                };
                card_rect.draw_fill(gfx, bg);
                card_rect.draw_outline(gfx, border);

                // Package Icon, Title & Version
                let pkg_icon = match pkg.category.as_str() {
                    "Applications" => IconId::Monitor,
                    "Tools" | "Development" => IconId::Terminal,
                    "Internet" => IconId::Globe,
                    _ => IconId::Packages,
                };
                draw_icon(gfx, pkg_icon, wx + 26, cy + 17, 22, Color::MACOS_ACCENT);

                gfx.draw_string((wx + 54) as u32, (cy + 10) as u32, &pkg.name, Color::MACOS_TEXT);
                let ver_str = format!("v{}", pkg.version);
                gfx.draw_string((wx + 54 + (pkg.name.len() as i32 * 8) + 12) as u32, (cy + 10) as u32, &ver_str, Color::MACOS_TEXT_SECONDARY);

                // Category pill tag
                let cat_tag = format!("[{}]", pkg.category);
                gfx.draw_string((wx + card_w - 180) as u32, (cy + 10) as u32, &cat_tag, Color::MACOS_ACCENT);

                // Description
                let desc = if pkg.description.len() > 38 {
                    format!("{}...", &pkg.description[..36])
                } else {
                    pkg.description.clone()
                };
                gfx.draw_string((wx + 54) as u32, (cy + 32) as u32, &desc, Color::MACOS_TEXT_SECONDARY);

                // Action Button (Install / Installed / Remove)
                let btn_rect = Rect::new(wx + card_w - 90, cy + 14, 80, 26);
                let is_btn_hover = btn_rect.contains(mouse.x, mouse.y);

                if pkg.is_installed {
                    btn_rect.draw_fill(gfx, if is_btn_hover { Color::new(255, 95, 87) } else { Color::new(200, 70, 60) });
                    btn_rect.draw_outline(gfx, Color::MACOS_CLOSE);
                    gfx.draw_string((btn_rect.x + 14) as u32, (btn_rect.y + 7) as u32, "Remove", Color::new(255, 255, 255));
                } else {
                    btn_rect.draw_fill(gfx, if is_btn_hover { Color::MACOS_ACCENT } else { Color::new(0, 100, 200) });
                    btn_rect.draw_outline(gfx, Color::MACOS_ACCENT);
                    gfx.draw_string((btn_rect.x + 12) as u32, (btn_rect.y + 7) as u32, "Install", Color::new(255, 255, 255));
                }
            }
        }

        // 4. Status Bar & Details Collapsible Panel
        let status_guard = self.op_status.lock().unwrap();
        let idle_text = if total_pkgs > 0 {
            let start_n = self.scroll_offset + 1;
            let end_n = (self.scroll_offset + 5).min(total_pkgs);
            format!("{}-{} of {} packages", start_n, end_n, total_pkgs)
        } else {
            "Ready. APT/dpkg backend connected.".to_string()
        };
        let status_text = match &*status_guard {
            OpStatus::Idle => &idle_text,
            OpStatus::Working(msg) => msg,
            OpStatus::Success(msg) => msg,
            OpStatus::Error(msg) => msg,
        };
        let status_col = match &*status_guard {
            OpStatus::Idle => Color::MACOS_TEXT_SECONDARY,
            OpStatus::Working(_) => Color::MACOS_ACCENT,
            OpStatus::Success(_) => Color::new(50, 215, 75),
            OpStatus::Error(_) => Color::new(255, 69, 58),
        };
        gfx.draw_string((wx + 16) as u32, (wy + wh - 22) as u32, status_text, status_col);

        self.btn_prev.draw(gfx, mouse);
        self.btn_next.draw(gfx, mouse);
        self.btn_toggle_details.draw(gfx, mouse);

        // If details expanded, draw log overlay
        if self.show_details {
            let log_rect = Rect::new(wx + 16, wy + wh - 140, ww - 32, 110);
            log_rect.draw_fill(gfx, Color::MACOS_WINDOW_BG);
            log_rect.draw_outline(gfx, Color::MACOS_BORDER);

            let log_guard = self.op_log.lock().unwrap();
            let total = log_guard.len();
            let start = if total > 5 { total - 5 } else { 0 };
            for (i, line) in log_guard[start..].iter().enumerate() {
                let trimmed = if line.len() > 65 {
                    format!("{}...", &line[..62])
                } else {
                    line.clone()
                };
                gfx.draw_string((wx + 24) as u32, (wy + wh - 132 + (i as i32 * 18)) as u32, &trimmed, Color::MACOS_TEXT_SECONDARY);
            }
        }
    }
}
