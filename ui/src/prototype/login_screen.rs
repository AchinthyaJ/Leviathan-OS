use crate::color::Color;
use crate::font::FontWeight;
use crate::graphics::Graphics;
use crate::prototype::abyss_scene::AbyssScene;
use crate::prototype::login_config::LoginConfig;
use sha_crypt::sha512_check;

#[derive(Debug, Clone)]
pub struct DemoUser {
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub initial: char,
}

#[derive(Debug, Clone)]
pub struct DesktopEnvironment {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginExitAction {
    Authenticate,
    Shutdown,
    Reboot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedElement {
    PasswordField,
    UserDropdown,
    DesktopEnvDropdown,
    PowerMenu(usize), // 0: Sleep, 1: Reboot, 2: Exit / Shutdown
}

/// System hardware status cached for the top-right status capsule
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub wifi_connected: bool,
    pub battery_percent: u8,
    pub battery_charging: bool,
    pub volume_percent: u8,
}

impl SystemStatus {
    pub fn query() -> Self {
        // 1. Wi-Fi / Network connectivity
        let mut wifi_connected = false;
        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str == "lo" {
                    continue;
                }
                let operstate_path = entry.path().join("operstate");
                if let Ok(state) = std::fs::read_to_string(operstate_path) {
                    let s = state.trim();
                    if s == "up" || s == "unknown" {
                        wifi_connected = true;
                        break;
                    }
                }
            }
        } else {
            wifi_connected = true;
        }

        // 2. Battery percentage & charging state
        let mut battery_percent = 92;
        let mut battery_charging = false;
        let mut found_battery = false;
        if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("BAT") {
                    let cap_path = entry.path().join("capacity");
                    if let Ok(cap_str) = std::fs::read_to_string(cap_path) {
                        if let Ok(val) = cap_str.trim().parse::<u8>() {
                            battery_percent = val.clamp(0, 100);
                            found_battery = true;
                        }
                    }
                    let stat_path = entry.path().join("status");
                    if let Ok(stat_str) = std::fs::read_to_string(stat_path) {
                        let s = stat_str.trim();
                        if s.eq_ignore_ascii_case("Charging") {
                            battery_charging = true;
                        }
                    }
                    if found_battery {
                        break;
                    }
                }
            }
        }
        if !found_battery {
            battery_percent = 98;
            battery_charging = true;
        }

        // 3. Volume percentage
        let mut volume_percent = 75;
        if let Ok(out) = std::process::Command::new("amixer").arg("get").arg("Master").output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                if let Some(idx) = s.find('%') {
                    let bytes = s.as_bytes();
                    let mut start = idx;
                    while start > 0 && bytes[start - 1].is_ascii_digit() {
                        start -= 1;
                    }
                    if let Ok(v) = s[start..idx].parse::<u8>() {
                        volume_percent = v.clamp(0, 100);
                    }
                }
            }
        }

        Self {
            wifi_connected,
            battery_percent,
            battery_charging,
            volume_percent,
        }
    }
}

pub struct LoginScreen {
    pub time: f32,
    pub users: Vec<DemoUser>,
    pub selected_user_index: usize,
    pub dropdown_open: bool,

    pub environments: Vec<DesktopEnvironment>,
    pub selected_env_index: usize,
    pub env_dropdown_open: bool,

    pub password_input: String,
    pub focus: FocusedElement,
    pub status_message: Option<(String, f32)>, // message, remaining seconds
    pub cursor_blink_timer: f32,
    pub auth_pending: bool,       // brief visual flash before exiting on success
    pub auth_pending_timer: f32,
    pub exit_action: Option<LoginExitAction>,
    pub reduced_motion: bool,
    pub requested_replay: bool,
    pub requested_exit: bool,

    // Mouse tracking for hover & pressed states
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_down: bool,

    // Clock minute rollover transition
    pub prev_minute_str: String,
    pub minute_transition_t: f32,

    // System hardware status for top-right capsule
    pub system_status: SystemStatus,
    pub status_refresh_timer: f32,

    pub config: LoginConfig,
}

impl LoginScreen {
    pub fn new() -> Self {
        Self::with_config(LoginConfig::load())
    }

    pub fn get_current_time_and_date() -> (String, String) {
        let mut now: libc::time_t = 0;
        unsafe { libc::time(&mut now); }
        if now > 0 {
            let mut tm: libc::tm = unsafe { std::mem::zeroed() };
            unsafe { libc::localtime_r(&now, &mut tm); }

            let hours = tm.tm_hour;
            let mins = tm.tm_min;
            let time_str = format!("{:02}:{:02}", hours, mins);

            let days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
            let months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
            let wday = (tm.tm_wday as usize).clamp(0, 6);
            let mon = (tm.tm_mon as usize).clamp(0, 11);
            let mday = tm.tm_mday;
            let date_str = format!("{}, {} {}", days[wday], months[mon], mday);

            (time_str, date_str)
        } else {
            ("22:45".to_string(), "Tuesday, October 14".to_string())
        }
    }

    pub fn handle_mouse_move(&mut self, mouse_x: i32, mouse_y: i32, mouse_down: bool) {
        self.mouse_x = mouse_x;
        self.mouse_y = mouse_y;
        self.mouse_down = mouse_down;
    }

    pub fn with_config(config: LoginConfig) -> Self {
        let users = vec![
            DemoUser {
                username: "leviathan".to_string(),
                display_name: "Leviathan".to_string(),
                role: "System Architect".to_string(),
                initial: 'L',
            },
            DemoUser {
                username: "operator".to_string(),
                display_name: "Oceanic Operator".to_string(),
                role: "Station Control".to_string(),
                initial: 'O',
            },
            DemoUser {
                username: "explorer".to_string(),
                display_name: "Deepsea Explorer".to_string(),
                role: "Guest Research Node".to_string(),
                initial: 'E',
            },
        ];

        let environments = vec![
            DesktopEnvironment {
                id: "hyprland".to_string(),
                name: "Hyprland".to_string(),
            },
            DesktopEnvironment {
                id: "noctalia".to_string(),
                name: "Noctalia".to_string(),
            },
            DesktopEnvironment {
                id: "sway".to_string(),
                name: "Sway".to_string(),
            },
            DesktopEnvironment {
                id: "leviathan".to_string(),
                name: "Leviathan UI".to_string(),
            },
        ];

        Self {
            time: 0.0,
            users,
            selected_user_index: 0,
            dropdown_open: false,
            environments,
            selected_env_index: 0,
            env_dropdown_open: false,
            password_input: String::new(),
            focus: FocusedElement::PasswordField,
            status_message: None,
            cursor_blink_timer: 0.0,
            auth_pending: false,
            auth_pending_timer: 0.0,
            exit_action: None,
            reduced_motion: false,
            requested_replay: false,
            requested_exit: false,
            mouse_x: -1,
            mouse_y: -1,
            mouse_down: false,
            prev_minute_str: String::new(),
            minute_transition_t: 1.0,
            system_status: SystemStatus::query(),
            status_refresh_timer: 0.0,
            config,
        }
    }

    pub fn selected_user(&self) -> &DemoUser {
        &self.users[self.selected_user_index]
    }

    pub fn select_user(&mut self, index: usize) {
        if index < self.users.len() {
            self.selected_user_index = index;
            self.dropdown_open = false;
            self.focus = FocusedElement::PasswordField;
            self.status_message = None;
        }
    }

    pub fn toggle_dropdown(&mut self) {
        self.dropdown_open = !self.dropdown_open;
        if self.dropdown_open {
            self.env_dropdown_open = false;
            self.focus = FocusedElement::UserDropdown;
        } else {
            self.focus = FocusedElement::PasswordField;
        }
    }

    pub fn selected_environment(&self) -> &DesktopEnvironment {
        &self.environments[self.selected_env_index]
    }

    pub fn select_environment(&mut self, index: usize) {
        if index < self.environments.len() {
            self.selected_env_index = index;
            self.env_dropdown_open = false;
            self.focus = FocusedElement::DesktopEnvDropdown;
        }
    }

    pub fn toggle_env_dropdown(&mut self) {
        self.env_dropdown_open = !self.env_dropdown_open;
        if self.env_dropdown_open {
            self.dropdown_open = false;
            self.focus = FocusedElement::DesktopEnvDropdown;
        } else {
            self.focus = FocusedElement::DesktopEnvDropdown;
        }
    }

    pub fn cycle_environment_next(&mut self) {
        if self.environments.is_empty() { return; }
        self.selected_env_index = (self.selected_env_index + 1) % self.environments.len();
    }

    pub fn cycle_environment_prev(&mut self) {
        if self.environments.is_empty() { return; }
        if self.selected_env_index > 0 {
            self.selected_env_index -= 1;
        } else {
            self.selected_env_index = self.environments.len() - 1;
        }
    }

    pub fn handle_char(&mut self, ch: char) {
        if self.auth_pending {
            return;
        }

        if self.focus == FocusedElement::PasswordField {
            if !ch.is_control() && self.password_input.len() < 32 {
                self.password_input.push(ch);
                self.cursor_blink_timer = 0.0;
            }
        }
    }

    pub fn handle_backspace(&mut self) {
        if self.auth_pending {
            return;
        }

        if self.focus == FocusedElement::PasswordField {
            self.password_input.pop();
            self.cursor_blink_timer = 0.0;
        }
    }

    /// Reads /etc/shadow and checks `password` against the stored SHA-512 crypt hash
    /// for `username`. Returns Ok(true) on match, Ok(false) on mismatch, Err on I/O.
    fn verify_password(username: &str, password: &str) -> Result<bool, String> {
        // At runtime inside the rootfs the path is just /etc/shadow.
        // We also accept the rootfs-relative path for development runs.
        let candidates = ["/etc/shadow", "rootfs/etc/shadow"];
        let content = candidates
            .iter()
            .find_map(|p| std::fs::read_to_string(p).ok())
            .ok_or_else(|| "Cannot open /etc/shadow".to_string())?;

        for line in content.lines() {
            let mut fields = line.splitn(9, ':');
            let user = fields.next().unwrap_or("");
            let hash = fields.next().unwrap_or("");
            if user != username {
                continue;
            }
            // Locked / no password
            if hash.is_empty() || hash == "*" || hash.starts_with('!') {
                return Ok(false);
            }
            // sha-crypt supports $6$ (SHA-512) hashes
            return match sha512_check(password, hash) {
                Ok(()) => Ok(true),
                Err(_) => Ok(false),
            };
        }
        Err(format!("User '{}' not found in shadow", username))
    }

    pub fn handle_enter(&mut self) {
        if self.dropdown_open {
            self.dropdown_open = false;
            self.focus = FocusedElement::PasswordField;
            return;
        }
        if self.env_dropdown_open {
            self.env_dropdown_open = false;
            self.focus = FocusedElement::DesktopEnvDropdown;
            return;
        }

        match self.focus {
            FocusedElement::PasswordField => {
                if self.password_input.is_empty() {
                    self.status_message = Some((
                        "Password required".to_string(),
                        3.0,
                    ));
                } else {
                    let username = self.users
                        .get(self.selected_user_index)
                        .map(|u| u.username.clone())
                        .unwrap_or_else(|| "leviathan".to_string());

                    match Self::verify_password(&username, &self.password_input) {
                        Ok(true) => {
                            // Brief success flash then exit
                            self.status_message = Some((
                                "Authenticated. Unlocking...".to_string(),
                                2.0,
                            ));
                            self.exit_action = Some(LoginExitAction::Authenticate);
                            self.auth_pending = true;
                            self.auth_pending_timer = 0.0;
                        }
                        Ok(false) => {
                            self.status_message = Some((
                                "Incorrect password".to_string(),
                                3.0,
                            ));
                            self.password_input.clear();
                        }
                        Err(e) => {
                            self.status_message = Some((
                                format!("Auth error: {}", e),
                                4.0,
                            ));
                            self.password_input.clear();
                        }
                    }
                }
            }
            FocusedElement::UserDropdown => {
                self.toggle_dropdown();
            }
            FocusedElement::DesktopEnvDropdown => {
                self.cycle_environment_next();
            }
            FocusedElement::PowerMenu(idx) => {
                match idx {
                    0 => {
                        self.status_message = Some((
                            "Suspending...".to_string(),
                            2.0,
                        ));
                        let _ = std::process::Command::new("/usr/sbin/poweroff")
                            .arg("--suspend")
                            .spawn();
                    }
                    1 => {
                        self.status_message = Some((
                            "Rebooting...".to_string(),
                            2.0,
                        ));
                        self.exit_action = Some(LoginExitAction::Reboot);
                        self.requested_exit = true;
                    }
                    2 => {
                        self.status_message = Some((
                            "Shutting down...".to_string(),
                            2.0,
                        ));
                        self.exit_action = Some(LoginExitAction::Shutdown);
                        self.requested_exit = true;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn handle_up(&mut self) {
        if self.dropdown_open {
            if self.selected_user_index > 0 {
                self.selected_user_index -= 1;
            } else {
                self.selected_user_index = self.users.len() - 1;
            }
            return;
        }
        if self.env_dropdown_open {
            self.cycle_environment_prev();
            return;
        }

        match self.focus {
            FocusedElement::PasswordField => {
                self.focus = FocusedElement::UserDropdown;
                self.toggle_dropdown();
            }
            FocusedElement::DesktopEnvDropdown => {
                self.focus = FocusedElement::PasswordField;
            }
            FocusedElement::PowerMenu(_) => {
                self.focus = FocusedElement::DesktopEnvDropdown;
            }
            FocusedElement::UserDropdown => {
                self.toggle_dropdown();
            }
        }
    }

    pub fn handle_down(&mut self) {
        if self.dropdown_open {
            if self.selected_user_index + 1 < self.users.len() {
                self.selected_user_index += 1;
            } else {
                self.selected_user_index = 0;
            }
            return;
        }
        if self.env_dropdown_open {
            self.cycle_environment_next();
            return;
        }

        match self.focus {
            FocusedElement::UserDropdown => {
                self.focus = FocusedElement::PasswordField;
            }
            FocusedElement::PasswordField => {
                self.focus = FocusedElement::DesktopEnvDropdown;
            }
            FocusedElement::DesktopEnvDropdown => {
                if self.config.show_power_menu {
                    self.focus = FocusedElement::PowerMenu(0);
                } else {
                    self.focus = FocusedElement::UserDropdown;
                }
            }
            FocusedElement::PowerMenu(_) => {
                self.focus = FocusedElement::UserDropdown;
            }
        }
    }

    pub fn handle_left(&mut self) {
        match self.focus {
            FocusedElement::DesktopEnvDropdown => {
                self.cycle_environment_prev();
            }
            FocusedElement::PowerMenu(idx) => {
                let next = if idx > 0 { idx - 1 } else { 2 };
                self.focus = FocusedElement::PowerMenu(next);
            }
            _ => {}
        }
    }

    pub fn handle_right(&mut self) {
        match self.focus {
            FocusedElement::DesktopEnvDropdown => {
                self.cycle_environment_next();
            }
            FocusedElement::PowerMenu(idx) => {
                let next = if idx < 2 { idx + 1 } else { 0 };
                self.focus = FocusedElement::PowerMenu(next);
            }
            _ => {}
        }
    }

    pub fn handle_tab(&mut self) {
        if self.dropdown_open {
            self.dropdown_open = false;
            self.focus = FocusedElement::PasswordField;
            return;
        }
        if self.env_dropdown_open {
            self.env_dropdown_open = false;
            self.focus = FocusedElement::DesktopEnvDropdown;
            return;
        }

        self.focus = match self.focus {
            FocusedElement::UserDropdown => FocusedElement::PasswordField,
            FocusedElement::PasswordField => FocusedElement::DesktopEnvDropdown,
            FocusedElement::DesktopEnvDropdown => {
                if self.config.show_power_menu {
                    FocusedElement::PowerMenu(0)
                } else {
                    FocusedElement::UserDropdown
                }
            }
            FocusedElement::PowerMenu(0) => FocusedElement::PowerMenu(1),
            FocusedElement::PowerMenu(1) => FocusedElement::PowerMenu(2),
            FocusedElement::PowerMenu(2) => FocusedElement::UserDropdown,
            FocusedElement::PowerMenu(_) => FocusedElement::PasswordField,
        };
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.cursor_blink_timer += dt;
        self.minute_transition_t += dt;

        let (cur_time, _) = Self::get_current_time_and_date();
        if self.prev_minute_str.is_empty() {
            self.prev_minute_str = cur_time;
        } else if self.prev_minute_str != cur_time {
            self.prev_minute_str = cur_time;
            self.minute_transition_t = 0.0;
        }

        self.status_refresh_timer += dt;
        if self.status_refresh_timer >= 3.0 {
            self.status_refresh_timer = 0.0;
            self.system_status = SystemStatus::query();
        }

        if let Some((_, ref mut remaining)) = self.status_message {
            *remaining -= dt;
            if *remaining <= 0.0 {
                self.status_message = None;
            }
        }

        if self.auth_pending {
            self.auth_pending_timer += dt;
            // After ~0.4 s of "Authenticated. Unlocking..." flash, exit cleanly
            if self.auth_pending_timer > 0.4 {
                self.auth_pending = false;
                self.password_input.clear();
                self.exit_action = Some(LoginExitAction::Authenticate);
                self.requested_exit = true;
            }
        }
    }

    /// Handles mouse pointer clicks on UI elements.
    pub fn handle_click(&mut self, mouse_x: i32, mouse_y: i32, screen_w: u32, screen_h: u32) {
        let (time_str, date_str) = Self::get_current_time_and_date();
        let coords = self.config.resolve(screen_w, screen_h, &time_str, &date_str);

        // 1. If user dropdown is open, check dropdown items (UPWARD)
        if self.dropdown_open {
            let item_h = 44;
            let menu_h = (self.users.len() as i32) * item_h + 12;
            let menu_w = coords.user_w;
            let menu_x = coords.user_x;
            let menu_y = coords.user_y - menu_h - 8;

            if mouse_x >= menu_x && mouse_x <= menu_x + menu_w && mouse_y >= menu_y && mouse_y <= menu_y + menu_h {
                for i in 0..self.users.len() {
                    let item_y = menu_y + 6 + (i as i32) * item_h;
                    if mouse_y >= item_y && mouse_y <= item_y + item_h {
                        self.select_user(i);
                        return;
                    }
                }
            } else {
                self.dropdown_open = false;
            }
        }

        // 2. Check User Switcher button
        let ux = coords.user_x;
        let uy = coords.user_y;
        let uw = coords.user_w;
        let uh = coords.user_h;
        if mouse_x >= ux && mouse_x <= ux + uw && mouse_y >= uy && mouse_y <= uy + uh {
            self.toggle_dropdown();
            return;
        }

        // 3. Check Password Input Box
        let px = coords.password_x;
        let py = coords.password_y;
        let pw = coords.password_w;
        let ph = coords.password_h;
        if mouse_x >= px && mouse_x <= px + pw && mouse_y >= py && mouse_y <= py + ph {
            let btn_cx = px + pw - 28;
            let btn_cy = py + ph / 2;
            let dist_sq = (mouse_x - btn_cx) * (mouse_x - btn_cx) + (mouse_y - btn_cy) * (mouse_y - btn_cy);
            if dist_sq <= 18 * 18 {
                // Clicked submit arrow
                self.focus = FocusedElement::PasswordField;
                self.handle_enter();
            } else {
                self.focus = FocusedElement::PasswordField;
            }
            return;
        }

        // 4. Check Desktop Environment Selector
        let dx = coords.desktop_env_x;
        let dy = coords.desktop_env_y;
        let dw = coords.desktop_env_w;
        let dh = coords.desktop_env_h;
        if mouse_x >= dx && mouse_x <= dx + dw && mouse_y >= dy && mouse_y <= dy + dh {
            self.focus = FocusedElement::DesktopEnvDropdown;
            self.cycle_environment_next();
            return;
        }

        // 5. Check Restrained Power Menu Buttons (40px diameter, 14px spacing)
        if coords.show_power_menu {
            let btn_d = 40;
            let btn_r = btn_d / 2;
            let spacing = 14;
            let start_x = coords.power_x;
            let cy = coords.power_y;

            for idx in 0..3 {
                let cx = start_x + (idx as i32) * (btn_d + spacing) + btn_r;
                let dist_sq = (mouse_x - cx) * (mouse_x - cx) + (mouse_y - cy) * (mouse_y - cy);
                if dist_sq <= 22 * 22 {
                    self.focus = FocusedElement::PowerMenu(idx as usize);
                    self.handle_enter();
                    return;
                }
            }
        }

        // 6. Check Top-Right Status Capsule
        if coords.show_status_capsule {
            let cx = coords.status_capsule_x;
            let cy = coords.status_capsule_y;
            let cw = coords.status_capsule_w;
            let ch = coords.status_capsule_h;
            if mouse_x >= cx && mouse_x <= cx + cw && mouse_y >= cy && mouse_y <= cy + ch {
                self.system_status = SystemStatus::query();
                let net_desc = if self.system_status.wifi_connected { "Online" } else { "Offline" };
                let chg_desc = if self.system_status.battery_charging { "Charging" } else { "Discharging" };
                self.status_message = Some((
                    format!("Network: {} | Battery: {}% ({}) | Vol: {}%", net_desc, self.system_status.battery_percent, chg_desc, self.system_status.volume_percent),
                    3.5,
                ));
                return;
            }
        }
    }

    pub fn render(&self, scene: &AbyssScene, gfx: &mut Graphics, blend: f32) {
        let opacity = blend.clamp(0.0, 1.0);
        if opacity <= 0.0 {
            return;
        }

        let _w = scene.width as i32;
        let _h = scene.height as i32;

        // 1. Abyss Background & Volumetric Light
        if self.config.show_volumetric_light {
            scene.render_background(gfx, self.time, opacity);
        } else {
            gfx.clear(Color::new(5, 12, 28));
        }

        // 2. Subtle distant ambient Leviathan in deep background
        if self.config.show_ambient_leviathan {
            let ambient_progress = if self.reduced_motion {
                0.5
            } else {
                ((self.time * 0.02) % 1.6) - 0.3
            };
            scene.render_leviathan(gfx, self.time * 0.4, 0.28 * opacity, ambient_progress);
        }

        // 3. Floating Marine Snow & Bioluminescent motes
        if self.config.show_particles {
            scene.render_particles(gfx, self.time, opacity);
        }

        // Dynamic strings from system clock
        let (cur_time, cur_date) = Self::get_current_time_and_date();
        let time_str = &cur_time;
        let date_str = &cur_date;

        // Resolve pixel coordinates through configuration (.conf file)
        let coords = self.config.resolve(scene.width, scene.height, time_str, date_str);

        // 4. Middle of screen: Clean Digital Clock and Date
        self.render_clock(gfx, &coords, time_str, date_str, opacity);

        // 5. Top Left: Leviathan Brand Logo (above account switcher)
        self.render_logo(scene, gfx, &coords, opacity);

        // 6. Full Left Column: Account Switcher, Password Box, Desktop Environment Chooser
        self.render_left_column(gfx, &coords, opacity);

        // 7. Top Right: Status Capsule (Wi-Fi, Volume, Battery with stages)
        if coords.show_status_capsule {
            self.render_status_capsule(gfx, &coords, opacity);
        }

        // 8. Prominent Power Controls (Sleep, Restart, Shut Down)
        if coords.show_power_menu {
            self.render_power_menu(gfx, &coords, opacity);
        }
    }

    /// Renders modern vector digital clock and date (SF/Inter style digits)
    fn render_clock(
        &self,
        gfx: &mut Graphics,
        coords: &crate::prototype::login_config::ResolvedLoginCoords,
        time_str: &str,
        date_str: &str,
        opacity: f32,
    ) {
        let digit_w = 48;
        let digit_h = 84;
        let stroke_t = 8;
        let colon_w = 20;
        let gap = 14;

        let shadow_col = Color::new_rgba(10, 2, 18, (opacity * 180.0) as u8);
        let fg_col = Color::new_rgba(248, 244, 255, (opacity * 255.0) as u8);

        let mut cur_x = coords.time_x;
        let time_y = coords.time_y;

        let time_chars: Vec<char> = time_str.chars().collect();

        for &ch in &time_chars {
            let cw = if ch == ':' { colon_w } else { digit_w };
            Self::draw_vector_digit(gfx, cur_x + 3, time_y + 3, ch, cw, digit_h, stroke_t, shadow_col);
            Self::draw_vector_digit(gfx, cur_x, time_y, ch, cw, digit_h, stroke_t, fg_col);
            cur_x += cw + gap;
        }

        if coords.show_date {
            let total_clock_w = 4 * digit_w + colon_w + 4 * gap;
            let clock_cx = coords.time_x + total_clock_w / 2;
            let (date_w, _) = gfx.measure_text_sf(date_str, 16.0, FontWeight::Medium);
            let date_x = clock_cx - date_w / 2;
            let date_y = coords.date_y;
            let date_fg = Color::new_rgba(205, 185, 235, (opacity * 225.0) as u8);
            let date_sh = Color::new_rgba(12, 4, 22, (opacity * 140.0) as u8);

            gfx.draw_text_sf_shadow(date_x, date_y, date_str, 16.0, FontWeight::Medium, date_fg, date_sh);
        }

    }

    fn draw_vector_digit(gfx: &mut Graphics, x: i32, y: i32, d: char, w: i32, h: i32, t: i32, color: Color) {
        let r = 5;
        let mid_y = y + (h - t) / 2;
        let bot_y = y + h - t;
        let right_x = x + w - t;

        match d {
            '0' => {
                gfx.fill_rounded_rect(x, y, w, t, r, color);
                gfx.fill_rounded_rect(x, bot_y, w, t, r, color);
                gfx.fill_rounded_rect(x, y, t, h, r, color);
                gfx.fill_rounded_rect(right_x, y, t, h, r, color);
            }
            '1' => {
                let cx = x + (w - t) / 2;
                gfx.fill_rounded_rect(cx, y, t, h, r, color);
                gfx.fill_rounded_rect(cx - 12, y, 12, t, r, color);
                gfx.fill_rounded_rect(cx - 14, bot_y, t + 28, t, r, color);
            }
            '2' => {
                gfx.fill_rounded_rect(x, y, w, t, r, color);
                gfx.fill_rounded_rect(right_x, y, t, (h + t) / 2, r, color);
                gfx.fill_rounded_rect(x, mid_y, w, t, r, color);
                gfx.fill_rounded_rect(x, mid_y, t, (h + t) / 2, r, color);
                gfx.fill_rounded_rect(x, bot_y, w, t, r, color);
            }
            '3' => {
                gfx.fill_rounded_rect(x, y, w, t, r, color);
                gfx.fill_rounded_rect(x + 10, mid_y, w - 10, t, r, color);
                gfx.fill_rounded_rect(x, bot_y, w, t, r, color);
                gfx.fill_rounded_rect(right_x, y, t, h, r, color);
            }
            '4' => {
                gfx.fill_rounded_rect(x, y, t, (h + t) / 2, r, color);
                gfx.fill_rounded_rect(x, mid_y, w, t, r, color);
                gfx.fill_rounded_rect(right_x - 4, y, t, h, r, color);
            }
            '5' => {
                gfx.fill_rounded_rect(x, y, w, t, r, color);
                gfx.fill_rounded_rect(x, y, t, (h + t) / 2, r, color);
                gfx.fill_rounded_rect(x, mid_y, w, t, r, color);
                gfx.fill_rounded_rect(right_x, mid_y, t, (h + t) / 2, r, color);
                gfx.fill_rounded_rect(x, bot_y, w, t, r, color);
            }
            '6' => {
                gfx.fill_rounded_rect(x, y, w, t, r, color);
                gfx.fill_rounded_rect(x, y, t, h, r, color);
                gfx.fill_rounded_rect(x, mid_y, w, t, r, color);
                gfx.fill_rounded_rect(right_x, mid_y, t, (h + t) / 2, r, color);
                gfx.fill_rounded_rect(x, bot_y, w, t, r, color);
            }
            '7' => {
                gfx.fill_rounded_rect(x, y, w, t, r, color);
                gfx.fill_rounded_rect(right_x, y, t, h, r, color);
            }
            '8' => {
                gfx.fill_rounded_rect(x, y, w, t, r, color);
                gfx.fill_rounded_rect(x, bot_y, w, t, r, color);
                gfx.fill_rounded_rect(x, mid_y, w, t, r, color);
                gfx.fill_rounded_rect(x, y, t, h, r, color);
                gfx.fill_rounded_rect(right_x, y, t, h, r, color);
            }
            '9' => {
                gfx.fill_rounded_rect(x, y, w, t, r, color);
                gfx.fill_rounded_rect(x, y, t, (h + t) / 2, r, color);
                gfx.fill_rounded_rect(x, mid_y, w, t, r, color);
                gfx.fill_rounded_rect(right_x, y, t, h, r, color);
                gfx.fill_rounded_rect(x, bot_y, w, t, r, color);
            }
            ':' => {
                let dot_r = 4;
                let dot_x = x + w / 2;
                gfx.fill_circle(dot_x, y + h / 3, dot_r, color);
                gfx.fill_circle(dot_x, y + (2 * h) / 3, dot_r, color);
            }
            _ => {}
        }
    }

    /// Renders Leviathan Logo at top-left above the account switcher
    fn render_logo(
        &self,
        scene: &AbyssScene,
        gfx: &mut Graphics,
        coords: &crate::prototype::login_config::ResolvedLoginCoords,
        opacity: f32,
    ) {
        let lx = coords.logo_x;
        let ly = coords.logo_y;
        let lw = coords.logo_w;
        let lh = coords.logo_h;

        let cx = lx + (lw as i32) / 2;
        let cy = ly + (lh as i32) / 2;

        if scene.logo.is_some() {
            scene.render_logo(gfx, cx, cy, lw, lh, opacity);
        } else {
            // High-fidelity procedural oceanic dragon emblem fallback
            let r = (lw.min(lh) as i32) / 2;
            gfx.fill_circle(cx, cy, r, Color::new_rgba(24, 10, 42, (opacity * 240.0) as u8));
            gfx.draw_circle(cx, cy, r, Color::new_rgba(185, 115, 255, (opacity * 220.0) as u8));
            gfx.draw_circle(cx, cy, r - 5, Color::new_rgba(145, 75, 230, (opacity * 180.0) as u8));
            gfx.fill_circle(cx, cy, 7, Color::new_rgba(255, 255, 255, (opacity * 245.0) as u8));
        }
    }

    /// Renders full-left column:
    /// - Account Switcher (above password)
    /// - Password Input Box (full left)
    /// - Desktop Environment Chooser (below password)
    fn render_left_column(
        &self,
        gfx: &mut Graphics,
        coords: &crate::prototype::login_config::ResolvedLoginCoords,
        opacity: f32,
    ) {
        let cur_user = self.selected_user();
        let radius = 16; // sleek squircle corner radius

        // ─── 1. ACCOUNT SWITCHER (ABOVE PASSWORD FIELD) ──────────────────────
        let ux = coords.user_x;
        let uy = coords.user_y;
        let uw = coords.user_w;
        let uh = coords.user_h;
        let u_cy = uy + uh / 2;

        let is_user_focused = self.focus == FocusedElement::UserDropdown;

        // Base glass fill
        let user_bg = if is_user_focused {
            Color::new_rgba(32, 14, 52, (opacity * 215.0) as u8)
        } else {
            Color::new_rgba(18, 8, 30, (opacity * 185.0) as u8)
        };
        gfx.fill_rounded_rect(ux, uy, uw, uh, radius, user_bg);

        // Top-edge subtle highlight
        gfx.fill_rect(
            ux + radius, uy + 1,
            uw - radius * 2, 1,
            Color::new_rgba(255, 255, 255, (opacity * 26.0) as u8),
        );

        // Squircle border
        let user_border = if is_user_focused {
            Color::new_rgba(185, 115, 255, (opacity * 150.0) as u8)
        } else {
            Color::new_rgba(85, 55, 115, (opacity * 70.0) as u8)
        };
        gfx.draw_rounded_rect(ux, uy, uw, uh, radius, user_border);

        // Avatar Initial Circle
        let avatar_cx = ux + 36;
        let avatar_cy = u_cy;
        let avatar_r = 18;
        gfx.fill_circle(avatar_cx, avatar_cy, avatar_r, Color::new_rgba(34, 14, 56, (opacity * 255.0) as u8));
        gfx.draw_circle(avatar_cx, avatar_cy, avatar_r, Color::new_rgba(185, 115, 255, (opacity * 130.0) as u8));
        let init_s = cur_user.initial.to_string();
        let (iw, ih) = gfx.measure_text_sf(&init_s, 20.0, FontWeight::Bold);
        gfx.draw_text_sf_shadow(
            avatar_cx - iw / 2,
            avatar_cy - ih / 2 + 1,
            &init_s,
            20.0,
            FontWeight::Bold,
            Color::new_rgba(250, 246, 255, (opacity * 255.0) as u8),
            Color::new_rgba(0, 0, 0, (opacity * 80.0) as u8),
        );

        // Display Name (SF Pro Medium 18px)
        let name_col = if is_user_focused {
            Color::new_rgba(255, 255, 255, (opacity * 255.0) as u8)
        } else {
            Color::new_rgba(250, 246, 255, (opacity * 250.0) as u8)
        };
        gfx.draw_text_sf_shadow(
            ux + 66,
            uy + 14,
            &cur_user.display_name,
            18.0,
            FontWeight::Medium,
            name_col,
            Color::new_rgba(0, 0, 0, (opacity * 80.0) as u8),
        );

        // Secondary handle (@username, SF Pro Regular 13px)
        let handle_str = format!("@{}", cur_user.username);
        let handle_col = if is_user_focused {
            Color::new_rgba(195, 135, 255, (opacity * 240.0) as u8)
        } else {
            Color::new_rgba(165, 140, 195, (opacity * 200.0) as u8)
        };
        gfx.draw_text_sf(ux + 66, uy + 38, &handle_str, 13.0, FontWeight::Regular, handle_col);


        // Dropdown indicator chevron
        let chev_cx = ux + uw - 24;
        let chev_cy = u_cy;
        let chev_col = if is_user_focused || self.dropdown_open {
            Color::new_rgba(195, 135, 255, (opacity * 240.0) as u8)
        } else {
            Color::new_rgba(155, 130, 185, (opacity * 190.0) as u8)
        };
        if self.dropdown_open {
            gfx.draw_line(chev_cx - 4, chev_cy + 2, chev_cx, chev_cy - 2, chev_col);
            gfx.draw_line(chev_cx, chev_cy - 2, chev_cx + 4, chev_cy + 2, chev_col);
        } else {
            gfx.draw_line(chev_cx - 4, chev_cy - 2, chev_cx, chev_cy + 2, chev_col);
            gfx.draw_line(chev_cx, chev_cy + 2, chev_cx + 4, chev_cy - 2, chev_col);
        }

        // ─── 2. PASSWORD TEXT BOX (FULL LEFT) ────────────────────────────────
        let px = coords.password_x;
        let py = coords.password_y;
        let pw = coords.password_w;
        let ph = coords.password_h;
        let p_cy = py + ph / 2; // Strict vertical center axis

        let is_pw_focused = self.focus == FocusedElement::PasswordField;

        // Base glass fill
        let pw_bg = if self.auth_pending {
            Color::new_rgba(45, 15, 65, (opacity * 225.0) as u8)
        } else if is_pw_focused {
            Color::new_rgba(28, 12, 46, (opacity * 210.0) as u8)
        } else {
            Color::new_rgba(18, 8, 30, (opacity * 190.0) as u8)
        };
        gfx.fill_rounded_rect(px, py, pw, ph, radius, pw_bg);

        // Top highlight
        gfx.fill_rect(
            px + radius, py + 1,
            pw - radius * 2, 1,
            Color::new_rgba(255, 255, 255, (opacity * 26.0) as u8),
        );

        // Subtle purple focus border (restrained, no outer halo)
        let border_col = if self.auth_pending {
            Color::new_rgba(215, 120, 255, (opacity * 200.0) as u8)
        } else if is_pw_focused {
            Color::new_rgba(185, 115, 255, (opacity * 150.0) as u8)
        } else {
            Color::new_rgba(85, 55, 115, (opacity * 70.0) as u8)
        };
        gfx.draw_rounded_rect(px, py, pw, ph, radius, border_col);

        // Lock icon - centered precisely on p_cy
        let lock_x = px + 20;
        let lock_icon_col = Color::new_rgba(165, 140, 195, (opacity * 190.0) as u8);
        gfx.draw_rect(lock_x, p_cy - 4, 14, 11, lock_icon_col);
        gfx.draw_line(lock_x + 3, p_cy - 9, lock_x + 10, p_cy - 9, lock_icon_col);
        gfx.draw_line(lock_x + 3, p_cy - 8, lock_x + 3, p_cy - 4, lock_icon_col);
        gfx.draw_line(lock_x + 10, p_cy - 8, lock_x + 10, p_cy - 4, lock_icon_col);

        let input_start_x = px + 48;
        let dot_spacing = 14;

        if self.password_input.is_empty() {
            let (_, ph_h) = gfx.measure_text_sf("Password", 15.0, FontWeight::Regular);
            gfx.draw_text_sf(
                input_start_x,
                p_cy - ph_h / 2,
                "Password",
                15.0,
                FontWeight::Regular,
                Color::new_rgba(135, 110, 165, (opacity * 180.0) as u8),
            );
        } else {
            let dot_color = Color::new_rgba(250, 246, 255, (opacity * 255.0) as u8);
            for i in 0..self.password_input.len() {
                let dot_x = input_start_x + (i as i32) * dot_spacing + 4;
                gfx.fill_circle(dot_x, p_cy, 4, dot_color);
            }
        }

        // Blinking cursor - centered on p_cy
        if is_pw_focused && !self.auth_pending {
            let cursor_visible = (self.cursor_blink_timer % 1.0) < 0.5;
            if cursor_visible {
                let cursor_x = if self.password_input.is_empty() {
                    input_start_x
                } else {
                    input_start_x + (self.password_input.len() as i32) * dot_spacing + 10
                };
                gfx.fill_rect(
                    cursor_x,
                    p_cy - 9,
                    2,
                    18,
                    Color::new_rgba(195, 125, 255, (opacity * 245.0) as u8),
                );
            }
        }

        // Submit button - centered precisely on p_cy with subtle hover & pressed states
        let btn_cx = px + pw - 28;
        let btn_cy = p_cy;
        let is_arrow_hovered = (self.mouse_x - btn_cx) * (self.mouse_x - btn_cx) + (self.mouse_y - btn_cy) * (self.mouse_y - btn_cy) <= 18 * 18;
        let is_arrow_pressed = is_arrow_hovered && self.mouse_down;
        let has_input = !self.password_input.is_empty();

        let btn_bg = if is_arrow_pressed && has_input {
            Color::new_rgba(145, 75, 225, (opacity * 255.0) as u8)
        } else if is_arrow_hovered && has_input {
            Color::new_rgba(195, 125, 255, (opacity * 245.0) as u8)
        } else if has_input {
            Color::new_rgba(175, 105, 255, (opacity * 225.0) as u8)
        } else if is_arrow_hovered {
            Color::new_rgba(45, 20, 65, (opacity * 190.0) as u8)
        } else {
            Color::new_rgba(30, 14, 48, (opacity * 170.0) as u8)
        };
        gfx.fill_circle(btn_cx, btn_cy, 15, btn_bg);
        gfx.draw_circle(btn_cx, btn_cy, 15, Color::new_rgba(85, 55, 115, (opacity * 85.0) as u8));

        let arrow_col = if has_input {
            Color::new_rgba(250, 246, 255, (opacity * 255.0) as u8)
        } else {
            Color::new_rgba(135, 110, 165, (opacity * 170.0) as u8)
        };
        gfx.draw_line(btn_cx - 5, p_cy, btn_cx + 4, p_cy, arrow_col);
        gfx.draw_line(btn_cx, p_cy - 4, btn_cx + 4, p_cy, arrow_col);
        gfx.draw_line(btn_cx, p_cy + 4, btn_cx + 4, p_cy, arrow_col);

        // Status message below password box
        let status_y = py + ph + 12;
        if let Some((ref msg, _)) = self.status_message {
            let col = if self.auth_pending {
                Color::new_rgba(215, 120, 255, (opacity * 255.0) as u8)
            } else {
                Color::new_rgba(240, 195, 110, (opacity * 240.0) as u8)
            };
            gfx.draw_text_sf(px + 4, status_y, msg, 13.0, FontWeight::Medium, col);
        }


        // ─── 3. DESKTOP ENVIRONMENT SELECTOR (BELOW PASSWORD FIELD) ─────────
        let dx = coords.desktop_env_x;
        let dy = coords.desktop_env_y;
        let dw = coords.desktop_env_w;
        let dh = coords.desktop_env_h;
        let d_cy = dy + dh / 2;

        let is_de_focused = self.focus == FocusedElement::DesktopEnvDropdown;

        // Base glass fill
        let de_bg = if is_de_focused {
            Color::new_rgba(28, 12, 46, (opacity * 205.0) as u8)
        } else {
            Color::new_rgba(18, 8, 30, (opacity * 180.0) as u8)
        };
        gfx.fill_rounded_rect(dx, dy, dw, dh, radius, de_bg);

        // Top highlight
        gfx.fill_rect(
            dx + radius, dy + 1,
            dw - radius * 2, 1,
            Color::new_rgba(255, 255, 255, (opacity * 24.0) as u8),
        );

        // Squircle border
        let de_border = if is_de_focused {
            Color::new_rgba(185, 115, 255, (opacity * 145.0) as u8)
        } else {
            Color::new_rgba(85, 55, 115, (opacity * 60.0) as u8)
        };
        gfx.draw_rounded_rect(dx, dy, dw, dh, radius, de_border);

        // Monitor icon
        let mon_x = dx + 20;
        let icon_col = if is_de_focused {
            Color::new_rgba(195, 125, 255, (opacity * 230.0) as u8)
        } else {
            Color::new_rgba(165, 140, 195, (opacity * 190.0) as u8)
        };
        gfx.draw_rect(mon_x, d_cy - 5, 14, 10, icon_col);
        gfx.draw_line(mon_x + 5, d_cy + 5, mon_x + 8, d_cy + 5, icon_col);
        gfx.draw_line(mon_x + 3, d_cy + 7, mon_x + 10, d_cy + 7, icon_col);

        let cur_env = self.selected_environment();
        // Clear "Session: Hyprland" hierarchy
        let session_label = "Session: ";
        let (label_w, label_h) = gfx.measure_text_sf(session_label, 14.0, FontWeight::Regular);
        let s_y = d_cy - label_h / 2;
        gfx.draw_text_sf(
            dx + 48,
            s_y,
            session_label,
            14.0,
            FontWeight::Regular,
            Color::new_rgba(165, 140, 195, (opacity * 190.0) as u8),
        );
        let env_text_col = if is_de_focused {
            Color::new_rgba(195, 125, 255, (opacity * 255.0) as u8)
        } else {
            Color::new_rgba(250, 246, 255, (opacity * 250.0) as u8)
        };
        gfx.draw_text_sf(
            dx + 48 + label_w,
            s_y,
            &cur_env.name,
            14.0,
            FontWeight::Medium,
            env_text_col,
        );

        // Dropdown indicator chevron
        let de_chev_col = if is_de_focused {
            Color::new_rgba(195, 125, 255, (opacity * 230.0) as u8)
        } else {
            Color::new_rgba(155, 130, 185, (opacity * 190.0) as u8)
        };
        gfx.draw_line(dx + dw - 27, d_cy - 2, dx + dw - 24, d_cy + 2, de_chev_col);
        gfx.draw_line(dx + dw - 24, d_cy + 2, dx + dw - 21, d_cy - 2, de_chev_col);

        // ─── 4. FLOATING USER DROPDOWN (OVERLAY WITHOUT SHIFTING LAYOUT - OPENS UPWARD) ─────
        if self.dropdown_open {
            let item_h = 44;
            let menu_h = (self.users.len() as i32) * item_h + 12;
            let menu_w = uw;
            let menu_x = ux;
            let menu_y = uy - menu_h - 8;

            // Glass dropdown panel
            gfx.fill_rounded_rect(
                menu_x, menu_y, menu_w, menu_h, radius,
                Color::new_rgba(18, 8, 30, (opacity * 245.0) as u8),
            );
            gfx.draw_rounded_rect(
                menu_x, menu_y, menu_w, menu_h, radius,
                Color::new_rgba(185, 115, 255, (opacity * 150.0) as u8),
            );

            for (i, user) in self.users.iter().enumerate() {
                let item_y = menu_y + 6 + (i as i32) * item_h;
                let is_selected = i == self.selected_user_index;

                if is_selected {
                    gfx.fill_rounded_rect(
                        menu_x + 6, item_y,
                        menu_w - 12, item_h - 2,
                        10,
                        Color::new_rgba(42, 18, 68, (opacity * 235.0) as u8),
                    );
                    gfx.draw_rounded_rect(
                        menu_x + 6, item_y,
                        menu_w - 12, item_h - 2,
                        10,
                        Color::new_rgba(185, 115, 255, (opacity * 160.0) as u8),
                    );
                }

                let b_x = menu_x + 24;
                let b_y = item_y + (item_h - 2) / 2;
                gfx.fill_circle(b_x, b_y, 12, Color::new_rgba(30, 12, 48, (opacity * 240.0) as u8));
                let init_s = user.initial.to_string();
                let (iw, ih) = gfx.measure_text_sf(&init_s, 13.0, FontWeight::Bold);
                gfx.draw_text_sf(b_x - iw / 2, b_y - ih / 2 + 1, &init_s, 13.0, FontWeight::Bold, Color::new_rgba(195, 125, 255, (opacity * 255.0) as u8));

                let n_col = if is_selected {
                    Color::new_rgba(250, 246, 255, (opacity * 255.0) as u8)
                } else {
                    Color::new_rgba(205, 185, 235, (opacity * 220.0) as u8)
                };
                gfx.draw_text_sf(menu_x + 44, item_y + 4, &user.display_name, 14.0, FontWeight::Medium, n_col);
                gfx.draw_text_sf(
                    menu_x + 44,
                    item_y + 22,
                    &format!("@{}", user.username),
                    12.0,
                    FontWeight::Regular,
                    Color::new_rgba(165, 140, 195, (opacity * 190.0) as u8),
                );
            }
        }
    }

    /// Renders clean, floating top-right hardware status indicators (Wi-Fi, Volume, Battery) without heavy enclosing capsule
    fn render_status_capsule(
        &self,
        gfx: &mut Graphics,
        coords: &crate::prototype::login_config::ResolvedLoginCoords,
        opacity: f32,
    ) {
        if !coords.show_status_capsule {
            return;
        }

        let cx = coords.status_capsule_x;
        let cy = coords.status_capsule_y;
        let cw = coords.status_capsule_w;
        let ch = coords.status_capsule_h;
        let mid_y = cy + ch / 2;

        let is_hovered = self.mouse_x >= cx && self.mouse_x <= cx + cw && self.mouse_y >= cy && self.mouse_y <= cy + ch;

        // 1. Wi-Fi indicator
        let wifi_x = cx + 14;
        let wifi_y = mid_y;
        let wifi_col = if self.system_status.wifi_connected {
            Color::new_rgba(205, 185, 235, (opacity * 225.0) as u8)
        } else {
            Color::new_rgba(135, 110, 160, (opacity * 160.0) as u8)
        };

        // Broadcast dot
        gfx.fill_circle(wifi_x, wifi_y + 4, 2, wifi_col);

        // Concentric arcs
        let arc_steps = 12;
        // Inner arc (radius 4.5)
        for s in 0..arc_steps {
            let deg = 220.0 + (s as f32) * (100.0 / arc_steps as f32);
            let rad = deg.to_radians();
            let px = wifi_x + (rad.cos() * 4.5).round() as i32;
            let py = wifi_y + 4 + (rad.sin() * 4.5).round() as i32;
            gfx.put_pixel(px, py, wifi_col);
        }
        // Outer arc (radius 7.5)
        for s in 0..arc_steps {
            let deg = 215.0 + (s as f32) * (110.0 / arc_steps as f32);
            let rad = deg.to_radians();
            let px = wifi_x + (rad.cos() * 7.5).round() as i32;
            let py = wifi_y + 4 + (rad.sin() * 7.5).round() as i32;
            gfx.put_pixel(px, py, wifi_col);
        }

        if !self.system_status.wifi_connected {
            gfx.draw_line(wifi_x - 5, wifi_y - 3, wifi_x + 5, wifi_y + 5, Color::new_rgba(255, 90, 90, (opacity * 220.0) as u8));
        }

        // 2. Volume indicator
        let vol_x = cx + 52;
        let vol_y = mid_y;
        let vol_col = Color::new_rgba(205, 185, 235, (opacity * 225.0) as u8);

        // Speaker box (3x6)
        gfx.fill_rect(vol_x - 6, vol_y - 3, 3, 6, vol_col);
        // Trapezoid speaker cone
        gfx.draw_line(vol_x - 3, vol_y - 3, vol_x, vol_y - 5, vol_col);
        gfx.draw_line(vol_x, vol_y - 5, vol_x, vol_y + 5, vol_col);
        gfx.draw_line(vol_x, vol_y + 5, vol_x - 3, vol_y + 3, vol_col);
        gfx.fill_rect(vol_x - 3, vol_y - 3, 3, 6, vol_col);

        // Sound waves
        if self.system_status.volume_percent > 0 {
            for s in 0..6 {
                let deg = -40.0 + (s as f32) * (80.0 / 6.0);
                let rad = deg.to_radians();
                let px = vol_x + 3 + (rad.cos() * 3.5).round() as i32;
                let py = vol_y + (rad.sin() * 3.5).round() as i32;
                gfx.put_pixel(px, py, vol_col);
            }
        }
        if self.system_status.volume_percent > 45 {
            for s in 0..8 {
                let deg = -40.0 + (s as f32) * (80.0 / 8.0);
                let rad = deg.to_radians();
                let px = vol_x + 3 + (rad.cos() * 6.5).round() as i32;
                let py = vol_y + (rad.sin() * 6.5).round() as i32;
                gfx.put_pixel(px, py, vol_col);
            }
        }

        // 3. Battery indicator & percentage
        let bat_x = cx + 86;
        let bat_y = mid_y - 5;
        let bat_w = 20;
        let bat_h = 10;

        let pct = self.system_status.battery_percent;
        let bat_rim_col = Color::new_rgba(155, 130, 185, (opacity * 200.0) as u8);

        // Battery outer shell
        gfx.draw_rounded_rect(bat_x, bat_y, bat_w, bat_h, 2, bat_rim_col);
        // Positive terminal nub
        gfx.fill_rect(bat_x + bat_w, mid_y - 2, 2, 4, bat_rim_col);

        // Number of bars
        let (num_bars, bar_col) = if pct >= 75 {
            (4, Color::new_rgba(195, 125, 255, (opacity * 245.0) as u8))
        } else if pct >= 50 {
            (3, Color::new_rgba(185, 115, 255, (opacity * 235.0) as u8))
        } else if pct >= 25 {
            (2, Color::new_rgba(175, 105, 245, (opacity * 225.0) as u8))
        } else if pct >= 12 {
            (1, Color::new_rgba(240, 185, 60, (opacity * 235.0) as u8))
        } else {
            (1, Color::new_rgba(255, 80, 80, (opacity * 240.0) as u8))
        };

        for b in 0..num_bars {
            let bar_x = bat_x + 2 + (b as i32) * 4;
            let bar_y = bat_y + 2;
            let bar_w = 2;
            let bar_h = bat_h - 4;
            gfx.fill_rect(bar_x, bar_y, bar_w, bar_h, bar_col);
        }

        // Charging indicator bolt
        if self.system_status.battery_charging {
            let bolt_x = bat_x + bat_w / 2;
            let bolt_y = mid_y;
            let bolt_col = Color::new_rgba(255, 235, 90, (opacity * 255.0) as u8);
            gfx.draw_line(bolt_x, bolt_y - 3, bolt_x - 2, bolt_y, bolt_col);
            gfx.draw_line(bolt_x - 2, bolt_y, bolt_x + 1, bolt_y, bolt_col);
            gfx.draw_line(bolt_x + 1, bolt_y, bolt_x - 1, bolt_y + 3, bolt_col);
        }

        // Battery percentage text
        let pct_text = format!("{}%", pct);
        let text_x = bat_x + bat_w + 6;
        let (_, pct_h) = gfx.measure_text_sf(&pct_text, 12.0, FontWeight::Medium);
        let text_y = mid_y - pct_h / 2;
        gfx.draw_text_sf(
            text_x,
            text_y,
            &pct_text,
            12.0,
            FontWeight::Medium,
            Color::new_rgba(235, 225, 250, (opacity * 235.0) as u8),
        );

        // 4. Subtle tooltip on hover
        if is_hovered {
            let wifi_str = if self.system_status.wifi_connected { "Wi-Fi: Connected" } else { "Wi-Fi: Offline" };
            let chg_str = if self.system_status.battery_charging { "(Charging)" } else { "(Discharging)" };
            let tooltip_msg = format!("{} | Vol: {}% | Battery: {}% {}", wifi_str, self.system_status.volume_percent, pct, chg_str);

            let (tip_msg_w, tip_msg_h) = gfx.measure_text_sf(&tooltip_msg, 12.0, FontWeight::Regular);
            let tip_w = tip_msg_w + 20;
            let tip_h = tip_msg_h + 8;
            let tip_x = (cx + cw) - tip_w;
            let tip_y = cy + ch + 6;

            gfx.fill_rounded_rect(
                tip_x, tip_y, tip_w, tip_h, 6,
                Color::new_rgba(16, 8, 28, (opacity * 245.0) as u8),
            );
            gfx.draw_rounded_rect(
                tip_x, tip_y, tip_w, tip_h, 6,
                Color::new_rgba(185, 115, 255, (opacity * 140.0) as u8),
            );
            gfx.draw_text_sf(
                tip_x + 10, tip_y + 4,
                &tooltip_msg,
                12.0,
                FontWeight::Regular,
                Color::new_rgba(250, 246, 255, (opacity * 255.0) as u8),
            );
        }
    }

    /// Renders restrained minimal power controls (Sleep, Restart, Shut Down) with 40px buttons and neutral styling at rest
    fn render_power_menu(
        &self,
        gfx: &mut Graphics,
        coords: &crate::prototype::login_config::ResolvedLoginCoords,
        opacity: f32,
    ) {
        let btn_d = 40;
        let btn_r = btn_d / 2;
        let spacing = 14;

        let start_x = coords.power_x;
        let cy = coords.power_y;

        let mut active_tooltip: Option<(&str, i32, i32, Color)> = None;

        for idx in 0..3 {
            let cx = start_x + (idx as i32) * (btn_d + spacing) + btn_r;
            let is_focused = self.focus == FocusedElement::PowerMenu(idx);
            let dist_sq = (self.mouse_x - cx) * (self.mouse_x - cx) + (self.mouse_y - cy) * (self.mouse_y - cy);
            let is_hovered = dist_sq <= (btn_r + 2) * (btn_r + 2);
            let is_active = is_focused || is_hovered;

            let (label, accent_col) = match idx {
                0 => ("Sleep", Color::new_rgba(195, 135, 255, (opacity * 255.0) as u8)),
                1 => ("Restart", Color::new_rgba(215, 115, 255, (opacity * 255.0) as u8)),
                2 => ("Shut Down", Color::new_rgba(255, 95, 130, (opacity * 255.0) as u8)),
                _ => ("", Color::new_rgba(255, 255, 255, (opacity * 255.0) as u8)),
            };

            if is_active {
                active_tooltip = Some((label, cx, cy, accent_col));
            }

            // Restrained neutral background at rest; subtle tinted fill when hovered/focused
            let bg = if is_active {
                match idx {
                    0 => Color::new_rgba(38, 16, 68, (opacity * 235.0) as u8),
                    1 => Color::new_rgba(42, 14, 62, (opacity * 235.0) as u8),
                    2 => Color::new_rgba(68, 16, 36, (opacity * 235.0) as u8),
                    _ => Color::new_rgba(32, 14, 52, (opacity * 220.0) as u8),
                }
            } else {
                Color::new_rgba(18, 8, 30, (opacity * 180.0) as u8)
            };

            let border = if is_active {
                accent_col
            } else {
                Color::new_rgba(85, 55, 115, (opacity * 85.0) as u8)
            };

            gfx.fill_circle(cx, cy, btn_r, bg);
            gfx.draw_circle(cx, cy, btn_r, border);

            // Icon color: neutral off-white at rest, crisp white when active
            let icon_col = if is_active {
                Color::new_rgba(255, 255, 255, (opacity * 255.0) as u8)
            } else {
                Color::new_rgba(185, 165, 215, (opacity * 190.0) as u8)
            };

            match idx {
                0 => {
                    // Sleep / Suspend: clean crescent moon
                    gfx.fill_circle(cx - 1, cy, 7, icon_col);
                    gfx.fill_circle(cx + 3, cy - 2, 6, bg);
                    // Single subtle star sparkle
                    gfx.put_pixel(cx + 5, cy + 3, icon_col);
                }
                1 => {
                    // Restart: clean circular refresh arrow
                    let r = 7.0f32;
                    for step in 0..24 {
                        let deg = 35.0 + (step as f32) * 11.5;
                        let rad = deg.to_radians();
                        let px = cx + (rad.cos() * r).round() as i32;
                        let py = cy + (rad.sin() * r).round() as i32;
                        gfx.put_pixel(px, py, icon_col);
                    }
                    // Arrowhead at top right
                    gfx.draw_line(cx + 4, cy - 8, cx + 8, cy - 5, icon_col);
                    gfx.draw_line(cx + 8, cy - 5, cx + 4, cy - 2, icon_col);
                }
                2 => {
                    // Shut Down: clean IEC power symbol (arc + vertical line)
                    let r = 7.0f32;
                    for step in 0..24 {
                        let deg = 125.0 + (step as f32) * 11.5;
                        let rad = deg.to_radians();
                        let px = cx + (rad.cos() * r).round() as i32;
                        let py = cy + 1 + (rad.sin() * r).round() as i32;
                        gfx.put_pixel(px, py, icon_col);
                    }
                    // Vertical line
                    gfx.draw_line(cx, cy - 8, cx, cy - 2, icon_col);
                }
                _ => {}
            }
        }

        // Render clean tooltip above active button
        if let Some((label, cx, cy, accent)) = active_tooltip {
            let (lw, lh) = gfx.measure_text_sf(label, 13.0, FontWeight::Medium);
            let tip_w = lw + 20;
            let tip_h = lh + 10;
            let tip_x = cx - tip_w / 2;
            let tip_y = cy - btn_r - tip_h - 8;

            gfx.fill_rounded_rect(
                tip_x, tip_y, tip_w, tip_h, 6,
                Color::new_rgba(16, 8, 28, (opacity * 245.0) as u8),
            );
            gfx.draw_rounded_rect(
                tip_x, tip_y, tip_w, tip_h, 6,
                accent,
            );
            gfx.draw_text_sf(
                tip_x + 10,
                tip_y + 5,
                label,
                13.0,
                FontWeight::Medium,
                Color::new_rgba(250, 246, 255, (opacity * 255.0) as u8),
            );
        }
    }
}
