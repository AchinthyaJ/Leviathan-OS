use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DesktopConfig {
    pub wallpaper_idx: usize,
    pub accent_idx: usize,
    pub ui_scale: f32,
    pub mouse_sensitivity: f32,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            wallpaper_idx: 0,
            accent_idx: 0,
            ui_scale: 1.0,
            mouse_sensitivity: 1.0,
        }
    }
}

impl DesktopConfig {
    const CONFIG_PATHS: [&'static str; 2] = [
        "/home/leviathan/.config/leviathan/desktop.conf",
        "/tmp/leviathan_desktop.conf",
    ];

    pub fn load() -> Self {
        let mut config = Self::default();

        for path_str in Self::CONFIG_PATHS {
            if let Ok(content) = fs::read_to_string(path_str) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = line.split_once('=') {
                        let k = k.trim();
                        let v = v.trim();
                        match k {
                            "wallpaper" => {
                                if let Ok(idx) = v.parse::<usize>() {
                                    config.wallpaper_idx = idx;
                                }
                            }
                            "accent" => {
                                if let Ok(idx) = v.parse::<usize>() {
                                    config.accent_idx = idx;
                                }
                            }
                            "ui_scale" => {
                                if let Ok(s) = v.parse::<f32>() {
                                    config.ui_scale = s.clamp(0.75, 2.5);
                                }
                            }
                            "mouse_sensitivity" => {
                                if let Ok(sens) = v.parse::<f32>() {
                                    config.mouse_sensitivity = sens.clamp(0.5, 2.5);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                break;
            }
        }

        config
    }

    pub fn save(&self) {
        let content = format!(
            "# Leviathan Desktop Configuration\nwallpaper={}\naccent={}\nui_scale={:.2}\nmouse_sensitivity={:.2}\n",
            self.wallpaper_idx, self.accent_idx, self.ui_scale, self.mouse_sensitivity
        );

        for path_str in Self::CONFIG_PATHS {
            let path = Path::new(path_str);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::write(path, &content).is_ok() {
                break;
            }
        }
    }
}
