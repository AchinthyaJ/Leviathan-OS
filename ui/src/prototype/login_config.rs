use std::fs;
use std::path::Path;

/// Configurable layout and theme coordinates for the Leviathan login screen.
/// All positions can be set directly to pixel X and Y coordinates.
/// Special value -1 for time/power indicates automatic centering or corner anchoring.
#[derive(Debug, Clone)]
pub struct LoginConfig {
    // Leviathan Logo
    pub logo_x: i32,
    pub logo_y: i32,
    pub logo_w: u32,
    pub logo_h: u32,

    // Account Switcher
    pub user_x: i32,
    pub user_y: i32,
    pub user_w: i32,
    pub user_h: i32,

    // Password Text Box (full left)
    pub password_x: i32,
    pub password_y: i32,
    pub password_w: i32,
    pub password_h: i32,

    // Desktop Environment Selector
    pub desktop_env_x: i32,
    pub desktop_env_y: i32,
    pub desktop_env_w: i32,
    pub desktop_env_h: i32,

    // Clock / Time (middle of screen)
    pub time_x: i32,
    pub time_y: i32,
    pub time_scale: u32,
    pub show_date: bool,
    pub date_y: i32,

    // Power Controls
    pub power_x: i32,
    pub power_y: i32,
    pub show_power_menu: bool,

    // Status Capsule (top-right: Wi-Fi, Volume, Battery)
    pub status_capsule_x: i32,
    pub status_capsule_y: i32,
    pub show_status_capsule: bool,

    // Visual options
    pub show_volumetric_light: bool,
    pub show_ambient_leviathan: bool,
    pub show_particles: bool,
}

impl Default for LoginConfig {
    fn default() -> Self {
        Self {
            // Top-left Leviathan logo - isolated, crisp silhouette
            logo_x: 80,
            logo_y: 60,
            logo_w: 96,
            logo_h: 96,

            // Account Switcher above password box - refined hierarchy, restrained height
            user_x: 80,
            user_y: 350,
            user_w: 440,
            user_h: 72,

            // Full-left Password input box - roomy, aligned
            password_x: 80,
            password_y: 436,
            password_w: 440,
            password_h: 62,

            // Session Selector below password box - visually secondary
            desktop_env_x: 80,
            desktop_env_y: 512,
            desktop_env_w: 440,
            desktop_env_h: 52,

            // Clock / Time - intentional right-bias (68% across) and balanced vertical eye-level
            time_x: -2,
            time_y: -1,
            time_scale: 6,
            show_date: true,
            date_y: -1,

            // Power Controls (bottom-right: restrained neutral buttons)
            power_x: -1,
            power_y: -1,
            show_power_menu: true,

            // Top-right status indicators
            status_capsule_x: -1,
            status_capsule_y: 36,
            show_status_capsule: true,

            // Atmosphere - sleek, premium, minimal bloat
            show_volumetric_light: true,
            show_ambient_leviathan: false,
            show_particles: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedLoginCoords {
    pub logo_x: i32,
    pub logo_y: i32,
    pub logo_w: u32,
    pub logo_h: u32,

    pub user_x: i32,
    pub user_y: i32,
    pub user_w: i32,
    pub user_h: i32,

    pub password_x: i32,
    pub password_y: i32,
    pub password_w: i32,
    pub password_h: i32,

    pub desktop_env_x: i32,
    pub desktop_env_y: i32,
    pub desktop_env_w: i32,
    pub desktop_env_h: i32,

    pub time_x: i32,
    pub time_y: i32,
    pub time_scale: u32,
    pub show_date: bool,
    pub date_x: i32,
    pub date_y: i32,

    pub power_x: i32,
    pub power_y: i32,
    pub show_power_menu: bool,

    pub status_capsule_x: i32,
    pub status_capsule_y: i32,
    pub status_capsule_w: i32,
    pub status_capsule_h: i32,
    pub show_status_capsule: bool,
}

impl LoginConfig {
    pub const CANDIDATE_PATHS: [&'static str; 6] = [
        "login.conf",
        "ui/login.conf",
        "/etc/leviathan/login.conf",
        "rootfs/etc/leviathan/login.conf",
        "/home/leviathan/.config/leviathan/login.conf",
        "/tmp/leviathan_login.conf",
    ];

    /// Load login configuration from the environment, candidate file paths, or fallback to default.
    pub fn load() -> Self {
        if let Ok(env_path) = std::env::var("LEVIATHAN_LOGIN_CONF") {
            if let Ok(config) = Self::load_from_path(&env_path) {
                return config;
            }
        }

        for path in Self::CANDIDATE_PATHS {
            if let Ok(config) = Self::load_from_path(path) {
                return config;
            }
        }

        Self::default()
    }

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read {}: {}", path.as_ref().display(), e))?;
        Ok(Self::parse(&content))
    }

    pub fn parse(content: &str) -> Self {
        let mut config = Self::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') || line.starts_with('[') {
                continue;
            }

            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim();
                let val = val.trim();

                match key {
                    // Logo coordinates & size
                    "logo_x" => if let Ok(v) = val.parse::<i32>() { config.logo_x = v; },
                    "logo_y" => if let Ok(v) = val.parse::<i32>() { config.logo_y = v; },
                    "logo_w" | "logo_size" | "logo_width" => if let Ok(v) = val.parse::<u32>() {
                        config.logo_w = v;
                        if config.logo_h == 64 { config.logo_h = v; }
                    },
                    "logo_h" | "logo_height" => if let Ok(v) = val.parse::<u32>() { config.logo_h = v; },

                    // Account switcher coordinates & size
                    "user_x" | "account_x" | "user_switcher_x" => if let Ok(v) = val.parse::<i32>() { config.user_x = v; },
                    "user_y" | "account_y" | "user_switcher_y" => if let Ok(v) = val.parse::<i32>() { config.user_y = v; },
                    "user_w" | "account_w" => if let Ok(v) = val.parse::<i32>() { config.user_w = v; },
                    "user_h" | "account_h" => if let Ok(v) = val.parse::<i32>() { config.user_h = v; },

                    // Password input box coordinates & size
                    "password_x" | "password_box_x" => if let Ok(v) = val.parse::<i32>() { config.password_x = v; },
                    "password_y" | "password_box_y" => if let Ok(v) = val.parse::<i32>() { config.password_y = v; },
                    "password_w" | "password_box_w" | "password_width" => if let Ok(v) = val.parse::<i32>() { config.password_w = v; },
                    "password_h" | "password_box_h" | "password_height" => if let Ok(v) = val.parse::<i32>() { config.password_h = v; },

                    // Desktop environment selector coordinates & size
                    "desktop_env_x" | "de_x" | "session_x" => if let Ok(v) = val.parse::<i32>() { config.desktop_env_x = v; },
                    "desktop_env_y" | "de_y" | "session_y" => if let Ok(v) = val.parse::<i32>() { config.desktop_env_y = v; },
                    "desktop_env_w" | "de_w" | "session_w" => if let Ok(v) = val.parse::<i32>() { config.desktop_env_w = v; },
                    "desktop_env_h" | "de_h" | "session_h" => if let Ok(v) = val.parse::<i32>() { config.desktop_env_h = v; },

                    // Clock / Time coordinates
                    "time_x" | "clock_x" => if let Ok(v) = val.parse::<i32>() { config.time_x = v; },
                    "time_y" | "clock_y" => if let Ok(v) = val.parse::<i32>() { config.time_y = v; },
                    "time_scale" | "clock_scale" => if let Ok(v) = val.parse::<u32>() { config.time_scale = v.clamp(1, 8); },
                    "show_date" => config.show_date = val.eq_ignore_ascii_case("true") || val == "1",
                    "date_y" => if let Ok(v) = val.parse::<i32>() { config.date_y = v; },

                    // Power menu coordinates
                    "power_x" => if let Ok(v) = val.parse::<i32>() { config.power_x = v; },
                    "power_y" => if let Ok(v) = val.parse::<i32>() { config.power_y = v; },
                    "show_power_menu" => config.show_power_menu = val.eq_ignore_ascii_case("true") || val == "1",

                    // Status Capsule coordinates
                    "status_capsule_x" | "capsule_x" => if let Ok(v) = val.parse::<i32>() { config.status_capsule_x = v; },
                    "status_capsule_y" | "capsule_y" => if let Ok(v) = val.parse::<i32>() { config.status_capsule_y = v; },
                    "show_status_capsule" | "show_capsule" => config.show_status_capsule = val.eq_ignore_ascii_case("true") || val == "1",

                    // Theme options
                    "show_volumetric_light" => config.show_volumetric_light = val.eq_ignore_ascii_case("true") || val == "1",
                    "show_ambient_leviathan" => config.show_ambient_leviathan = val.eq_ignore_ascii_case("true") || val == "1",
                    "show_particles" => config.show_particles = val.eq_ignore_ascii_case("true") || val == "1",

                    _ => {}
                }
            }
        }

        config
    }

    /// Resolve layout coordinates against screen dimensions.
    /// Handles special -1 sentinel values for auto-centering and corner anchoring.
    pub fn resolve(&self, screen_w: u32, screen_h: u32, _time_str: &str, date_str: &str) -> ResolvedLoginCoords {
        let sw = screen_w as i32;
        let sh = screen_h as i32;

        // Calculate modern vector clock dimensions: 4 digits + 1 colon + 4 gaps
        let digit_w = 48;
        let colon_w = 20;
        let gap = 14;
        let time_w = 4 * digit_w + colon_w + 4 * gap;
        let time_h = 84;

        let time_x = if self.time_x == -2 {
            // Right-biased: 68% across the screen, minus half the clock width
            (sw * 68 / 100) - time_w / 2
        } else if self.time_x < 0 {
            (sw - time_w) / 2
        } else {
            self.time_x
        };

        let time_y = if self.time_y < 0 {
            (sh / 2) - 100
        } else {
            self.time_y
        };

        let date_w = (date_str.len() as i32) * 8;
        let date_x = time_x + (time_w - date_w) / 2;
        let date_y = if self.date_y < 0 {
            time_y + time_h + 16
        } else {
            self.date_y
        };

        // Restrained power menu resolution (3 circular buttons of diameter 40 + spacing 14)
        let btn_d = 40;
        let spacing = 14;
        let total_power_w = 3 * btn_d + 2 * spacing;
        let power_x = if self.power_x < 0 {
            sw - 48 - total_power_w
        } else {
            self.power_x
        };

        let power_y = if self.power_y < 0 {
            sh - 48
        } else {
            self.power_y
        };

        // Top-right status indicators resolution (clean floating indicators)
        let status_capsule_w = 160;
        let status_capsule_h = 24;
        let status_capsule_x = if self.status_capsule_x < 0 {
            sw - 48 - status_capsule_w
        } else {
            self.status_capsule_x
        };
        let status_capsule_y = if self.status_capsule_y < 0 {
            36
        } else {
            self.status_capsule_y
        };

        ResolvedLoginCoords {
            logo_x: self.logo_x,
            logo_y: self.logo_y,
            logo_w: self.logo_w,
            logo_h: self.logo_h,

            user_x: self.user_x,
            user_y: self.user_y,
            user_w: self.user_w,
            user_h: self.user_h,

            password_x: self.password_x,
            password_y: self.password_y,
            password_w: self.password_w,
            password_h: self.password_h,

            desktop_env_x: self.desktop_env_x,
            desktop_env_y: self.desktop_env_y,
            desktop_env_w: self.desktop_env_w,
            desktop_env_h: self.desktop_env_h,

            time_x,
            time_y,
            time_scale: self.time_scale,
            show_date: self.show_date,
            date_x,
            date_y,

            power_x,
            power_y,
            show_power_menu: self.show_power_menu,

            status_capsule_x,
            status_capsule_y,
            status_capsule_w,
            status_capsule_h,
            show_status_capsule: self.show_status_capsule,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_config_default() {
        let cfg = LoginConfig::default();
        assert_eq!(cfg.logo_x, 80);
        assert_eq!(cfg.logo_y, 60);
        assert_eq!(cfg.password_x, 80);
        assert_eq!(cfg.desktop_env_x, 80);
        assert_eq!(cfg.time_x, -2);
        assert_eq!(cfg.time_y, -1);

        let resolved = cfg.resolve(1920, 1080, "22:45", "Tuesday, October 14");
        assert_eq!(resolved.password_x, 80);
        // Time is right-biased at 68% of screen width
        let digit_w = 48;
        let colon_w = 20;
        let gap = 14;
        let time_w = 4 * digit_w + colon_w + 4 * gap;
        assert_eq!(resolved.time_x, (1920 * 68 / 100) - time_w / 2);
        assert_eq!(resolved.status_capsule_x, 1920 - 48 - 160);
        assert_eq!(resolved.status_capsule_y, 36);
        assert!(resolved.show_status_capsule);
    }

    #[test]
    fn test_login_config_custom_xy() {
        let conf_str = r#"
            # Custom layout coordinates
            logo_x = 120
            logo_y = 80
            user_x = 120
            user_y = 190
            password_x = 120
            password_y = 280
            desktop_env_x = 120
            desktop_env_y = 350
            time_x = 850
            time_y = 200
            time_scale = 3
            power_x = 1600
            power_y = 950
            status_capsule_x = 1650
            status_capsule_y = 50
        "#;

        let cfg = LoginConfig::parse(conf_str);
        assert_eq!(cfg.logo_x, 120);
        assert_eq!(cfg.logo_y, 80);
        assert_eq!(cfg.user_x, 120);
        assert_eq!(cfg.user_y, 190);
        assert_eq!(cfg.password_x, 120);
        assert_eq!(cfg.password_y, 280);
        assert_eq!(cfg.desktop_env_x, 120);
        assert_eq!(cfg.desktop_env_y, 350);
        assert_eq!(cfg.time_x, 850);
        assert_eq!(cfg.time_y, 200);
        assert_eq!(cfg.time_scale, 3);
        assert_eq!(cfg.power_x, 1600);
        assert_eq!(cfg.power_y, 950);
        assert_eq!(cfg.status_capsule_x, 1650);
        assert_eq!(cfg.status_capsule_y, 50);

        let resolved = cfg.resolve(1920, 1080, "22:45", "Tuesday, October 14");
        assert_eq!(resolved.password_x, 120);
        assert_eq!(resolved.time_x, 850);
        assert_eq!(resolved.time_y, 200);
        assert_eq!(resolved.power_x, 1600);
        assert_eq!(resolved.power_y, 950);
        assert_eq!(resolved.status_capsule_x, 1650);
        assert_eq!(resolved.status_capsule_y, 50);
    }
}

