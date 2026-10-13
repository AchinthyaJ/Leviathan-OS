use crate::icons::IconId;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon_id: IconId,
    pub icon: String,
    pub comment: String,
    pub terminal: bool,
    pub categories: String,
    /// TryExec= validated binary path; None for built-ins and entries that passed validation
    pub try_exec: Option<String>,
    /// Path= working directory override from the desktop entry
    pub path: Option<String>,
    pub dbus_activatable: bool,
    pub startup_wm_class: Option<String>,
}

pub struct AppScanner;

impl AppScanner {
    pub fn scan_applications() -> Vec<AppEntry> {
        let mut apps = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 1. Core Leviathan built-in system applications
        let builtins = vec![
            AppEntry {
                name: "Terminal".into(),
                exec: "terminal".into(),
                icon_id: IconId::Terminal,
                icon: "terminal".into(),
                comment: "Interactive command line terminal with root/user permissions".into(),
                terminal: false,
                categories: "System;Utility;TerminalEmulator;".into(),
                try_exec: None,
                path: None,
                dbus_activatable: false,
                startup_wm_class: None,
            },
            AppEntry {
                name: "Settings".into(),
                exec: "settings".into(),
                icon_id: IconId::Settings,
                icon: "settings".into(),
                comment: "System diagnostics, network management, and display control".into(),
                terminal: false,
                categories: "Settings;System;".into(),
                try_exec: None,
                path: None,
                dbus_activatable: false,
                startup_wm_class: None,
            },
            AppEntry {
                name: "Packages".into(),
                exec: "packages".into(),
                icon_id: IconId::Packages,
                icon: "packages".into(),
                comment: "Discover, install, and update Leviathan applications".into(),
                terminal: false,
                categories: "System;PackageManager;".into(),
                try_exec: None,
                path: None,
                dbus_activatable: false,
                startup_wm_class: None,
            },
            AppEntry {
                name: "Files".into(),
                exec: "files".into(),
                icon_id: IconId::Files,
                icon: "files".into(),
                comment: "Explore filesystem and user home directories".into(),
                terminal: false,
                categories: "System;FileManager;Utility;".into(),
                try_exec: None,
                path: None,
                dbus_activatable: false,
                startup_wm_class: None,
            },
            AppEntry {
                name: "3D Abyss".into(),
                exec: "abyss3d".into(),
                icon_id: IconId::Abyss3D,
                icon: "abyss3d".into(),
                comment: "Interactive underwater 3D mesh rendering viewport".into(),
                terminal: false,
                categories: "Graphics;Viewer;".into(),
                try_exec: None,
                path: None,
                dbus_activatable: false,
                startup_wm_class: None,
            },
        ];

        for b in builtins {
            seen.insert(b.name.to_lowercase());
            apps.push(b);
        }

        // 2. Scan standard Linux XDG application directories
        let search_dirs = [
            "/usr/share/applications",
            "/usr/local/share/applications",
            "/home/leviathan/.local/share/applications",
        ];

        for dir_path in &search_dirs {
            let p = Path::new(dir_path);
            if !p.is_dir() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(p) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Some(app) = Self::parse_desktop_file(&content) {
                                let key = app.name.to_lowercase();
                                if !seen.contains(&key) {
                                    seen.insert(key);
                                    apps.push(app);
                                }
                            }
                        }
                    }
                }
            }
        }

        apps
    }

    fn parse_desktop_file(content: &str) -> Option<AppEntry> {
        let mut in_desktop_entry = false;
        let mut name = String::new();
        let mut exec = String::new();
        let mut icon = String::new();
        let mut comment = String::new();
        let mut terminal = false;
        let mut no_display = false;
        let mut categories = String::new();
        let mut try_exec: Option<String> = None;
        let mut path: Option<String> = None;
        let mut dbus_activatable = false;
        let mut startup_wm_class: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                in_desktop_entry = line == "[Desktop Entry]";
                continue;
            }
            if !in_desktop_entry {
                continue;
            }

            if let Some(r) = line.strip_prefix("Name=") {
                if name.is_empty() {
                    name = r.trim().to_string();
                }
            } else if let Some(r) = line.strip_prefix("Exec=") {
                if exec.is_empty() {
                    exec = r.trim().to_string();
                }
            } else if let Some(r) = line.strip_prefix("Icon=") {
                if icon.is_empty() {
                    icon = r.trim().to_string();
                }
            } else if let Some(r) = line.strip_prefix("Comment=") {
                if comment.is_empty() {
                    comment = r.trim().to_string();
                }
            } else if let Some(r) = line.strip_prefix("Terminal=") {
                terminal = r.trim().eq_ignore_ascii_case("true");
            } else if let Some(r) = line.strip_prefix("NoDisplay=") {
                no_display = r.trim().eq_ignore_ascii_case("true");
            } else if let Some(r) = line.strip_prefix("Categories=") {
                if categories.is_empty() {
                    categories = r.trim().to_string();
                }
            } else if let Some(r) = line.strip_prefix("TryExec=") {
                if try_exec.is_none() {
                    try_exec = Some(r.trim().to_string());
                }
            } else if let Some(r) = line.strip_prefix("Path=") {
                if path.is_none() {
                    let p = r.trim().to_string();
                    if !p.is_empty() {
                        path = Some(p);
                    }
                }
            } else if let Some(r) = line.strip_prefix("DBusActivatable=") {
                dbus_activatable = r.trim().eq_ignore_ascii_case("true");
            } else if let Some(r) = line.strip_prefix("StartupWMClass=") {
                if startup_wm_class.is_none() {
                    let v = r.trim().to_string();
                    if !v.is_empty() {
                        startup_wm_class = Some(v);
                    }
                }
            }
        }

        if no_display || name.is_empty() || exec.is_empty() {
            return None;
        }

        // TryExec validation: if TryExec is specified and the binary cannot be found, skip entry.
        if let Some(ref try_bin) = try_exec {
            if !Self::binary_is_accessible(try_bin) {
                return None;
            }
        }

        let icon_id = Self::resolve_icon(&name, &exec, &icon, &categories);

        Some(AppEntry {
            name,
            exec,
            icon_id,
            icon,
            comment,
            terminal,
            categories,
            try_exec,
            path,
            dbus_activatable,
            startup_wm_class,
        })
    }

    /// Check whether a binary name or absolute path is reachable on this system.
    fn binary_is_accessible(name: &str) -> bool {
        if name.starts_with('/') {
            Path::new(name).exists()
        } else {
            let search = [
                "/usr/bin", "/bin", "/usr/local/bin",
                "/usr/games", "/sbin", "/usr/sbin",
                "/opt/hypr/bin",
            ];
            search.iter().any(|dir| Path::new(dir).join(name).exists())
        }
    }

    fn resolve_icon(name: &str, exec: &str, icon_str: &str, categories: &str) -> IconId {
        if let Some(ic) = IconId::from_name(icon_str) {
            return ic;
        }
        if let Some(ic) = IconId::from_name(name) {
            return ic;
        }
        if let Some(ic) = IconId::from_name(exec) {
            return ic;
        }
        let n = name.to_lowercase();
        let e = exec.to_lowercase();
        if n.contains("terminal") || e.contains("terminal") || n.contains("bash") || n.contains("xterm") || n.contains("kitty") || e.contains("kitty") {
            IconId::Terminal
        } else if n.contains("settings") || n.contains("config") || n.contains("control") {
            IconId::Settings
        } else if n.contains("files") || n.contains("manager") || n.contains("file") || n.contains("dolphin") || e.contains("dolphin") {
            IconId::Files
        } else if n.contains("firefox") || n.contains("browser") || n.contains("web") || categories.contains("Network") || categories.contains("WebBrowser") {
            IconId::Globe
        } else if n.contains("package") || n.contains("software") || n.contains("store") || n.contains("apt") {
            IconId::Packages
        } else if n.contains("3d") || n.contains("mesh") {
            IconId::Abyss3D
        } else if n.contains("layer") || n.contains("editor") {
            IconId::LayerEditor
        } else if categories.contains("Development") || categories.contains("Utility") {
            IconId::Terminal
        } else {
            IconId::Monitor
        }
    }

    /// Tokenise an Exec= value using the XDG Desktop Entry spec shell-like quoting rules.
    fn tokenize_exec(exec_line: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_single = false;
        let mut in_double = false;
        let mut chars = exec_line.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '\'' if !in_double => {
                    in_single = !in_single;
                }
                '"' if !in_single => {
                    in_double = !in_double;
                }
                '\\' if in_double => {
                    // Inside double quotes only `\"`, `\\`, `` ` ``, `$` are escape sequences
                    if let Some(&next) = chars.peek() {
                        if matches!(next, '"' | '\\' | '`' | '$') {
                            current.push(next);
                            chars.next();
                            continue;
                        }
                    }
                    current.push('\\');
                }
                ' ' | '\t' if !in_single && !in_double => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }
        if !current.is_empty() {
            tokens.push(current);
        }
        tokens
    }

    /// Expand field codes embedded inside a single token (e.g. `--icon=%i`).
    fn expand_token_fields(token: &str, name: &str, icon: &str) -> String {
        let mut result = String::with_capacity(token.len());
        let mut chars = token.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '%' {
                result.push(c);
                continue;
            }
            match chars.next() {
                Some('%') => result.push('%'),
                Some('c') => result.push_str(name),
                Some('i') => result.push_str(icon),
                // All file/URL and deprecated codes → drop
                Some('f') | Some('F') | Some('u') | Some('U')
                | Some('d') | Some('D') | Some('n') | Some('N')
                | Some('v') | Some('m') | Some('k') => {}
                Some(_) => {}
                None => {}
            }
        }
        result
    }

    /// Parse a desktop entry Exec= command line into binary path and argument list.
    ///
    /// `name` and `icon` are the application name and icon string from the same entry,
    /// used to expand the `%c` and `%i` field codes respectively.
    pub fn parse_exec_line(exec_line: &str, name: &str, icon: &str) -> (String, Vec<String>) {
        let tokens = Self::tokenize_exec(exec_line);

        let mut bin = String::new();
        let mut args: Vec<String> = Vec::new();

        for token in tokens {
            match token.as_str() {
                // Single-file / single-URL placeholders — no file is supplied at launch time
                "%f" | "%u" => {}
                // Multi-file / multi-URL placeholders
                "%F" | "%U" => {}
                // Deprecated codes (removed in spec version 1.4+)
                "%d" | "%D" | "%n" | "%N" | "%v" | "%m" => {}
                // Desktop file path — not tracked
                "%k" => {}
                // Application name
                "%c" => {
                    let s = name.to_string();
                    if bin.is_empty() { bin = s; } else { args.push(s); }
                }
                // Icon expansion: --icon <iconname>
                "%i" => {
                    if !icon.is_empty() {
                        if bin.is_empty() {
                            // Degenerate edge case: treat binary name as icon
                            bin = icon.to_string();
                        } else {
                            args.push("--icon".to_string());
                            args.push(icon.to_string());
                        }
                    }
                }
                _ => {
                    // Expand any embedded field codes (e.g. `--url=%u`, `%%`)
                    let expanded = Self::expand_token_fields(&token, name, icon);
                    if !expanded.is_empty() {
                        if bin.is_empty() { bin = expanded; } else { args.push(expanded); }
                    }
                }
            }
        }

        (bin, args)
    }
}
