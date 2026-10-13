use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub section: String,
    pub category: String,
    pub installed_size_kb: u64,
    pub is_installed: bool,
    pub is_upgradable: bool,
}

#[derive(Debug, Clone)]
pub struct PackageDetails {
    pub info: PackageInfo,
    pub maintainer: String,
    pub architecture: String,
    pub dependencies: Vec<String>,
    pub homepage: String,
    pub status: String,
}

pub struct PackageManager;

impl PackageManager {
    const DPKG_STATUS_PATH: &'static str = "/var/lib/dpkg/status";

    /// Parse /var/lib/dpkg/status into a list of installed packages
    pub fn list_installed() -> Vec<PackageInfo> {
        let path = Path::new(Self::DPKG_STATUS_PATH);
        if !path.exists() {
            return Vec::new();
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut packages = Vec::new();
        let mut cur_name = String::new();
        let mut cur_version = String::new();
        let mut cur_desc = String::new();
        let mut cur_section = String::new();
        let mut cur_size = 0u64;
        let mut is_installed = false;

        for line in content.lines() {
            if line.trim().is_empty() {
                if !cur_name.is_empty() && is_installed {
                    let cat = Self::categorize(&cur_section, &cur_name);
                    packages.push(PackageInfo {
                        name: cur_name.clone(),
                        version: cur_version.clone(),
                        description: cur_desc.clone(),
                        section: cur_section.clone(),
                        category: cat,
                        installed_size_kb: cur_size,
                        is_installed: true,
                        is_upgradable: false,
                    });
                }
                cur_name.clear();
                cur_version.clear();
                cur_desc.clear();
                cur_section.clear();
                cur_size = 0;
                is_installed = false;
                continue;
            }

            if let Some(rest) = line.strip_prefix("Package: ") {
                cur_name = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("Status: ") {
                if rest.contains("installed") && !rest.contains("not-installed") {
                    is_installed = true;
                }
            } else if let Some(rest) = line.strip_prefix("Version: ") {
                cur_version = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("Description: ") {
                cur_desc = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("Section: ") {
                cur_section = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("Installed-Size: ") {
                cur_size = rest.trim().parse::<u64>().unwrap_or(0);
            }
        }

        if !cur_name.is_empty() && is_installed {
            let cat = Self::categorize(&cur_section, &cur_name);
            packages.push(PackageInfo {
                name: cur_name,
                version: cur_version,
                description: cur_desc,
                section: cur_section,
                category: cat,
                installed_size_kb: cur_size,
                is_installed: true,
                is_upgradable: false,
            });
        }

        packages
    }

    /// Curated list of popular essential applications & tools for Leviathan
    pub fn curated_catalog() -> Vec<PackageInfo> {
        vec![
            PackageInfo {
                name: "firefox".into(),
                version: "128.0".into(),
                description: "Fast, privacy-focused modern web browser".into(),
                section: "web".into(),
                category: "Internet".into(),
                installed_size_kb: 198000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "vlc".into(),
                version: "3.0.20".into(),
                description: "Universal multimedia player and streaming engine".into(),
                section: "video".into(),
                category: "Media".into(),
                installed_size_kb: 75000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "htop".into(),
                version: "3.3.0".into(),
                description: "Interactive real-time process viewer and system monitor".into(),
                section: "utils".into(),
                category: "Tools".into(),
                installed_size_kb: 420,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "neofetch".into(),
                version: "7.1.0".into(),
                description: "Fast CLI system info visualizer with custom logo".into(),
                section: "utils".into(),
                category: "Tools".into(),
                installed_size_kb: 340,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "lynx".into(),
                version: "2.9.0".into(),
                description: "Text-based terminal web browser with SSL support".into(),
                section: "web".into(),
                category: "Internet".into(),
                installed_size_kb: 2400,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "nano".into(),
                version: "7.2".into(),
                description: "Simple, user-friendly console text editor".into(),
                section: "editors".into(),
                category: "Tools".into(),
                installed_size_kb: 890,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "vim".into(),
                version: "9.1".into(),
                description: "Powerful extensible modal text editor".into(),
                section: "editors".into(),
                category: "Tools".into(),
                installed_size_kb: 3800,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "git".into(),
                version: "2.43.0".into(),
                description: "Fast, distributed revision control system".into(),
                section: "devel".into(),
                category: "Development".into(),
                installed_size_kb: 24000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "python3".into(),
                version: "3.12.3".into(),
                description: "Interactive high-level object-oriented language".into(),
                section: "python".into(),
                category: "Development".into(),
                installed_size_kb: 31000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "curl".into(),
                version: "8.5.0".into(),
                description: "Command line tool for transferring data with URL syntax".into(),
                section: "web".into(),
                category: "Internet".into(),
                installed_size_kb: 560,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "tmux".into(),
                version: "3.4".into(),
                description: "Terminal multiplexer with session management".into(),
                section: "utils".into(),
                category: "Tools".into(),
                installed_size_kb: 1100,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "tree".into(),
                version: "2.1.1".into(),
                description: "List directory contents in a tree-like format".into(),
                section: "utils".into(),
                category: "Tools".into(),
                installed_size_kb: 120,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "wget".into(),
                version: "1.21.4".into(),
                description: "Non-interactive network downloader for HTTP/HTTPS/FTP".into(),
                section: "web".into(),
                category: "Internet".into(),
                installed_size_kb: 960,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "openssh-client".into(),
                version: "9.6p1".into(),
                description: "Secure shell (SSH) client for remote login".into(),
                section: "net".into(),
                category: "Internet".into(),
                installed_size_kb: 5200,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "gcc".into(),
                version: "13.2.0".into(),
                description: "GNU C compiler for building native executables".into(),
                section: "devel".into(),
                category: "Development".into(),
                installed_size_kb: 42000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "make".into(),
                version: "4.3".into(),
                description: "Build automation tool for compiling software".into(),
                section: "devel".into(),
                category: "Development".into(),
                installed_size_kb: 1600,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "nodejs".into(),
                version: "20.11.1".into(),
                description: "JavaScript runtime built on V8 engine".into(),
                section: "devel".into(),
                category: "Development".into(),
                installed_size_kb: 35000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "nmap".into(),
                version: "7.94".into(),
                description: "Network exploration tool and security scanner".into(),
                section: "net".into(),
                category: "Internet".into(),
                installed_size_kb: 27000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "ffmpeg".into(),
                version: "6.1.1".into(),
                description: "Complete solution for recording and streaming audio/video".into(),
                section: "video".into(),
                category: "Media".into(),
                installed_size_kb: 94000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "neovim".into(),
                version: "0.9.5".into(),
                description: "Hyperextensible Vim-based text editor".into(),
                section: "editors".into(),
                category: "Tools".into(),
                installed_size_kb: 8200,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "ripgrep".into(),
                version: "14.1.0".into(),
                description: "Blazing fast recursive line-oriented search tool".into(),
                section: "utils".into(),
                category: "Tools".into(),
                installed_size_kb: 5800,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "docker.io".into(),
                version: "24.0.7".into(),
                description: "Linux container runtime for building and shipping apps".into(),
                section: "admin".into(),
                category: "Tools".into(),
                installed_size_kb: 120000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "gimp".into(),
                version: "2.10.36".into(),
                description: "GNU Image Manipulation Program for photo editing".into(),
                section: "graphics".into(),
                category: "Media".into(),
                installed_size_kb: 68000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "libreoffice".into(),
                version: "24.2.2".into(),
                description: "Full-featured office productivity suite".into(),
                section: "editors".into(),
                category: "Applications".into(),
                installed_size_kb: 480000,
                is_installed: false,
                is_upgradable: false,
            },
            PackageInfo {
                name: "thunderbird".into(),
                version: "115.9".into(),
                description: "Free email client and personal information manager".into(),
                section: "mail".into(),
                category: "Internet".into(),
                installed_size_kb: 210000,
                is_installed: false,
                is_upgradable: false,
            },
        ]
    }

    /// Search packages across installed packages, curated catalog, and apt-cache
    pub fn search(query: &str) -> Vec<PackageInfo> {
        let installed = Self::list_installed();
        let query_lower = query.to_lowercase().trim().to_string();

        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 1. Check installed packages
        for pkg in &installed {
            if query_lower.is_empty()
                || pkg.name.to_lowercase().contains(&query_lower)
                || pkg.description.to_lowercase().contains(&query_lower)
            {
                seen.insert(pkg.name.clone());
                results.push(pkg.clone());
            }
        }

        // 2. Check curated catalog
        for pkg in Self::curated_catalog() {
            if seen.contains(&pkg.name) {
                continue;
            }
            if query_lower.is_empty()
                || pkg.name.to_lowercase().contains(&query_lower)
                || pkg.description.to_lowercase().contains(&query_lower)
            {
                seen.insert(pkg.name.clone());
                results.push(pkg);
            }
        }

        // 3. If online/configured and query is specific, query apt-cache search
        if !query_lower.is_empty() {
            if let Ok(output) = Command::new("/bin/apt-cache")
                .args(["search", &query_lower])
                .env("LD_LIBRARY_PATH", "/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu")
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines().take(15) {
                    if let Some((name, desc)) = line.split_once(" - ") {
                        let name = name.trim().to_string();
                        if !seen.contains(&name) {
                            seen.insert(name.clone());
                            results.push(PackageInfo {
                                name: name.clone(),
                                version: "latest".into(),
                                description: desc.trim().to_string(),
                                section: "universe".into(),
                                category: "General".into(),
                                installed_size_kb: 0,
                                is_installed: false,
                                is_upgradable: false,
                            });
                        }
                    }
                }
            }
        }

        results
    }

    /// Get detailed package information
    pub fn info(name: &str) -> Option<PackageDetails> {
        // 1. Check if installed in dpkg status
        let path = Path::new(Self::DPKG_STATUS_PATH);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                let mut cur_pkg = String::new();
                let mut cur_status = String::new();
                let mut cur_ver = String::new();
                let mut cur_desc = String::new();
                let mut cur_sec = String::new();
                let mut cur_maint = String::new();
                let mut cur_arch = String::new();
                let mut cur_deps = Vec::new();
                let mut cur_size = 0u64;

                for line in content.lines() {
                    if line.trim().is_empty() {
                        if cur_pkg == name {
                            return Some(PackageDetails {
                                info: PackageInfo {
                                    name: cur_pkg,
                                    version: cur_ver,
                                    description: cur_desc,
                                    section: cur_sec.clone(),
                                    category: Self::categorize(&cur_sec, name),
                                    installed_size_kb: cur_size,
                                    is_installed: cur_status.contains("installed"),
                                    is_upgradable: false,
                                },
                                maintainer: cur_maint,
                                architecture: cur_arch,
                                dependencies: cur_deps,
                                homepage: "https://leviathan-os.org".into(),
                                status: cur_status,
                            });
                        }
                        cur_pkg.clear();
                        cur_status.clear();
                        cur_ver.clear();
                        cur_desc.clear();
                        cur_sec.clear();
                        cur_maint.clear();
                        cur_arch.clear();
                        cur_deps.clear();
                        cur_size = 0;
                        continue;
                    }

                    if let Some(r) = line.strip_prefix("Package: ") {
                        cur_pkg = r.trim().to_string();
                    } else if let Some(r) = line.strip_prefix("Status: ") {
                        cur_status = r.trim().to_string();
                    } else if let Some(r) = line.strip_prefix("Version: ") {
                        cur_ver = r.trim().to_string();
                    } else if let Some(r) = line.strip_prefix("Description: ") {
                        cur_desc = r.trim().to_string();
                    } else if let Some(r) = line.strip_prefix("Section: ") {
                        cur_sec = r.trim().to_string();
                    } else if let Some(r) = line.strip_prefix("Maintainer: ") {
                        cur_maint = r.trim().to_string();
                    } else if let Some(r) = line.strip_prefix("Architecture: ") {
                        cur_arch = r.trim().to_string();
                    } else if let Some(r) = line.strip_prefix("Depends: ") {
                        cur_deps = r.split(',').map(|s| s.trim().to_string()).collect();
                    } else if let Some(r) = line.strip_prefix("Installed-Size: ") {
                        cur_size = r.trim().parse::<u64>().unwrap_or(0);
                    }
                }
            }
        }

        // 2. Fall back to curated catalog
        for pkg in Self::curated_catalog() {
            if pkg.name == name {
                return Some(PackageDetails {
                    info: pkg,
                    maintainer: "Leviathan Packaging Team <packages@leviathan-os.org>".into(),
                    architecture: "amd64".into(),
                    dependencies: vec!["libc6 (>= 2.38)".into()],
                    homepage: format!("https://packages.leviathan-os.org/pkg/{}", name),
                    status: "available in repository".into(),
                });
            }
        }

        None
    }

    /// Update package lists via APT
    pub fn update<F>(mut on_progress: F) -> Result<(), String>
    where
        F: FnMut(&str),
    {
        on_progress("Connecting to Leviathan package mirrors...");

        let is_root = unsafe { libc::geteuid() } == 0;
        let prefix = if is_root { "" } else { "/bin/sudo " };
        let command_str = format!("{}/bin/apt-get update -o Acquire::Languages=none", prefix);

        let mut child = Command::new("/bin/sh")
            .args(["-c", &format!("{} 2>&1", command_str)])
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("DEBCONF_NONINTERACTIVE_SEEN", "true")
            .env("LD_LIBRARY_PATH", "/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu")
            .env("HOME", "/root")
            .env("PATH", "/bin:/usr/bin:/sbin:/usr/sbin:/usr/local/bin:/usr/games")
            .env("DPKG_DATADIR", "/usr/share/dpkg")
            .env("DPKG_MAINTSCRIPT_ARCH", "amd64")
            .env("APT_CONFIG", "/etc/apt/apt.conf.d/99leviathan.conf")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn apt-get update: {}", e))?;

        let mut all_output = Vec::new();

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                all_output.push(line.clone());
                on_progress(&line);
            }
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            on_progress("Package database updated successfully.");
            Ok(())
        } else {
            let err_detail = all_output
                .iter()
                .rev()
                .find(|l| l.contains("E: ") || l.contains("error") || l.contains("Failed") || l.contains("Unable"))
                .cloned()
                .unwrap_or_else(|| format!("apt-get update failed with exit code: {}", status));
            Err(err_detail)
        }
    }

    /// Upgrade all installed packages via APT
    pub fn upgrade<F>(mut on_progress: F) -> Result<(), String>
    where
        F: FnMut(&str),
    {
        on_progress("Upgrading system packages via APT...");

        let is_root = unsafe { libc::geteuid() } == 0;
        let prefix = if is_root { "" } else { "/bin/sudo " };
        let command_str = format!("{}/bin/apt-get upgrade -y --no-install-recommends", prefix);

        let mut child = Command::new("/bin/sh")
            .args(["-c", &format!("{} 2>&1", command_str)])
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("DEBCONF_NONINTERACTIVE_SEEN", "true")
            .env("LD_LIBRARY_PATH", "/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu")
            .env("HOME", "/root")
            .env("PATH", "/bin:/usr/bin:/sbin:/usr/sbin:/usr/local/bin:/usr/games")
            .env("DPKG_DATADIR", "/usr/share/dpkg")
            .env("DPKG_MAINTSCRIPT_ARCH", "amd64")
            .env("APT_CONFIG", "/etc/apt/apt.conf.d/99leviathan.conf")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn package upgrade: {}", e))?;

        let mut all_output = Vec::new();

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                all_output.push(line.clone());
                on_progress(&line);
            }
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            on_progress("All packages upgraded successfully.");
            Ok(())
        } else {
            let err_detail = all_output
                .iter()
                .rev()
                .find(|l| l.contains("E: ") || l.contains("error") || l.contains("Failed") || l.contains("Unable"))
                .cloned()
                .unwrap_or_else(|| format!("Upgrade failed with exit code: {}", status));
            Err(err_detail)
        }
    }

    /// Search local package caches for a matching .deb file
    pub fn find_local_deb(name: &str) -> Option<String> {
        let search_dirs = [
            "/home/leviathan/Downloads",
            "/home/leviathan",
            "/var/cache/apt/archives",
            "/usr/share/leviathan/packages",
            "/tmp",
        ];

        let target = name.to_lowercase();
        let clean = name.strip_suffix(".deb").unwrap_or(name).to_lowercase();

        for dir in &search_dirs {
            let p = Path::new(dir);
            if let Ok(entries) = fs::read_dir(p) {
                for entry in entries.flatten() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if fname.ends_with(".deb") {
                        let lower = fname.to_lowercase();
                        if lower.starts_with(&format!("{}_", clean))
                            || lower.starts_with(&format!("{}-", clean))
                            || lower == format!("{}.deb", clean)
                            || lower.starts_with(&format!("{}_", target))
                            || lower.starts_with(&format!("{}-", target))
                            || lower == format!("{}.deb", target)
                            || lower == target
                        {
                            return Some(entry.path().to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Install a package via APT / dpkg
    pub fn install<F>(name: &str, mut on_progress: F) -> Result<(), String>
    where
        F: FnMut(&str),
    {
        on_progress(&format!("Resolving dependencies for {}...", name));

        let is_root = unsafe { libc::geteuid() } == 0;
        let prefix = if is_root { "" } else { "/bin/sudo " };

        // 1. Check if a local .deb package exists in archives or packages dir
        let local_deb = Self::find_local_deb(name);

        let command_str = if let Some(ref deb_path) = local_deb {
            on_progress(&format!("Found local Debian package: {}", deb_path));
            format!("{}/bin/dpkg --force-confdef --force-confold --force-architecture -i '{}' || ({}/bin/apt-get --fix-broken install -y && {}/bin/dpkg --force-confdef --force-confold --force-architecture -i '{}')", prefix, deb_path, prefix, prefix, deb_path)
        } else if name.ends_with(".deb") {
            let deb_arg = if Path::new(name).is_absolute() || name.starts_with("./") {
                name.to_string()
            } else if Path::new(&format!("/home/leviathan/Downloads/{}", name)).exists() {
                format!("/home/leviathan/Downloads/{}", name)
            } else if Path::new(&format!("/tmp/{}", name)).exists() {
                format!("/tmp/{}", name)
            } else if Path::new(&format!("/home/leviathan/{}", name)).exists() {
                format!("/home/leviathan/{}", name)
            } else {
                format!("./{}", name)
            };
            format!("{}/bin/dpkg --force-confdef --force-confold --force-architecture -i '{}' || ({}/bin/apt-get --fix-broken install -y && {}/bin/dpkg --force-confdef --force-confold --force-architecture -i '{}')", prefix, deb_arg, prefix, prefix, deb_arg)
        } else {
            // Check if /var/lib/apt/lists has package lists. If not, auto-update first!
            let lists_exist = Path::new("/var/lib/apt/lists").read_dir().map(|entries| {
                entries.flatten().any(|e| e.file_name().to_string_lossy().contains("Packages"))
            }).unwrap_or(false);

            if !lists_exist {
                on_progress("Synchronizing package lists via apt update...");
                let _ = Self::update(|l| on_progress(l));
            }

            format!("{}/bin/apt-get install -y --no-install-recommends -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold '{}'", prefix, name)
        };

        on_progress(&format!("Installing {} via dpkg/APT...", name));

        let mut child = Command::new("/bin/sh")
            .args(["-c", &format!("{} 2>&1", command_str)])
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("DEBCONF_NONINTERACTIVE_SEEN", "true")
            .env("LD_LIBRARY_PATH", "/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu")
            .env("HOME", "/root")
            .env("PATH", "/bin:/usr/bin:/sbin:/usr/sbin:/usr/local/bin:/usr/games")
            .env("DPKG_DATADIR", "/usr/share/dpkg")
            .env("DPKG_MAINTSCRIPT_ARCH", "amd64")
            .env("APT_CONFIG", "/etc/apt/apt.conf.d/99leviathan.conf")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start package installation: {}", e))?;

        let mut all_output = Vec::new();

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                all_output.push(line.clone());
                on_progress(&line);
            }
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            // Also register a desktop entry if one does not exist for common apps
            Self::ensure_desktop_entry(name);
            on_progress(&format!("{} installed successfully.", name));
            Ok(())
        } else {
            let err_detail = all_output
                .iter()
                .rev()
                .find(|l| l.contains("E: ") || l.contains("error") || l.contains("Failed") || l.contains("Unable") || l.contains("privilege"))
                .cloned()
                .unwrap_or_else(|| format!("Installation failed with exit code: {}", status));
            Err(err_detail)
        }
    }

    /// Remove a package via APT / dpkg
    pub fn remove<F>(name: &str, mut on_progress: F) -> Result<(), String>
    where
        F: FnMut(&str),
    {
        on_progress(&format!("Removing {}...", name));

        let is_root = unsafe { libc::geteuid() } == 0;
        let prefix = if is_root { "" } else { "/bin/sudo " };
        let command_str = format!("{}/bin/apt-get remove -y '{}' || {}/bin/dpkg -r '{}'", prefix, name, prefix, name);

        let mut child = Command::new("/bin/sh")
            .args(["-c", &format!("{} 2>&1", command_str)])
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("DEBCONF_NONINTERACTIVE_SEEN", "true")
            .env("LD_LIBRARY_PATH", "/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu")
            .env("HOME", "/root")
            .env("PATH", "/bin:/usr/bin:/sbin:/usr/sbin:/usr/local/bin:/usr/games")
            .env("DPKG_DATADIR", "/usr/share/dpkg")
            .env("DPKG_MAINTSCRIPT_ARCH", "amd64")
            .env("APT_CONFIG", "/etc/apt/apt.conf.d/99leviathan.conf")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn package removal: {}", e))?;

        let mut all_output = Vec::new();

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                all_output.push(line.clone());
                on_progress(&line);
            }
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            on_progress(&format!("{} removed successfully.", name));
            Ok(())
        } else {
            let err_detail = all_output
                .iter()
                .rev()
                .find(|l| l.contains("E: ") || l.contains("error") || l.contains("Failed") || l.contains("Unable") || l.contains("privilege"))
                .cloned()
                .unwrap_or_else(|| format!("Removal failed with exit code: {}", status));
            Err(err_detail)
        }
    }

    /// Check for upgradable packages
    pub fn check_updates() -> Vec<PackageInfo> {
        let mut updates = Vec::new();
        if let Ok(output) = Command::new("/bin/apt-get")
            .args(["-s", "upgrade"])
            .env("LD_LIBRARY_PATH", "/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu")
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("Inst ") {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if !parts.is_empty() {
                        let name = parts[0];
                        let ver = parts.get(1).unwrap_or(&"new");
                        updates.push(PackageInfo {
                            name: name.to_string(),
                            version: ver.trim_matches(|c| c == '[' || c == ']').to_string(),
                            description: format!("Software update for {}", name),
                            section: "updates".into(),
                            category: "Update".into(),
                            installed_size_kb: 0,
                            is_installed: true,
                            is_upgradable: true,
                        });
                    }
                }
            }
        }
        updates
    }

    /// Derives or creates standard Linux .desktop file in /usr/share/applications/
    pub fn ensure_desktop_entry(name: &str) {
        let app_dir = Path::new("/usr/share/applications");
        if !app_dir.exists() {
            let _ = fs::create_dir_all(app_dir);
        }

        let desktop_file = app_dir.join(format!("{}.desktop", name));
        if !desktop_file.exists() {
            let content = match name {
                "firefox" => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=Mozilla Firefox\n\
                     Exec=/usr/bin/firefox\n\
                     Icon=firefox\n\
                     Comment=Browse the World Wide Web\n\
                     Categories=Network;WebBrowser;\n"
                ),
                "vlc" => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=VLC Media Player\n\
                     Exec=/usr/bin/vlc\n\
                     Icon=vlc\n\
                     Comment=Universal multimedia player\n\
                     Categories=AudioVideo;Player;\n"
                ),
                "gimp" => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=GNU Image Manipulation Program\n\
                     Exec=/usr/bin/gimp\n\
                     Icon=gimp\n\
                     Comment=Create and edit images\n\
                     Categories=Graphics;2DGraphics;RasterGraphics;\n"
                ),
                "thunderbird" => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=Mozilla Thunderbird\n\
                     Exec=/usr/bin/thunderbird\n\
                     Icon=thunderbird\n\
                     Comment=Send and receive mail\n\
                     Categories=Network;Email;\n"
                ),
                "libreoffice" => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=LibreOffice\n\
                     Exec=/usr/bin/libreoffice\n\
                     Icon=libreoffice-main\n\
                     Comment=Office productivity suite\n\
                     Categories=Office;\n"
                ),
                _ => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name={}\n\
                     Exec={}\n\
                     Icon={}\n\
                     Comment={} for Leviathan OS\n\
                     Categories=Utility;\n",
                    name.to_uppercase(),
                    name,
                    name,
                    name
                ),
            };
            let _ = fs::write(desktop_file, content);
        }
    }

    fn categorize(section: &str, name: &str) -> String {
        let sec = section.to_lowercase();
        let n = name.to_lowercase();
        if sec.contains("web") || sec.contains("net") || sec.contains("mail")
            || n == "firefox" || n == "lynx" || n == "curl" || n == "wget"
            || n == "openssh-client" || n == "nmap" || n == "thunderbird"
        {
            "Internet".into()
        } else if sec.contains("video") || sec.contains("sound") || sec.contains("graphics")
            || n == "vlc" || n == "ffmpeg" || n == "gimp"
        {
            "Media".into()
        } else if sec.contains("devel") || sec.contains("python") || sec.contains("rust")
            || n == "git" || n == "gcc" || n == "make" || n == "nodejs"
        {
            "Development".into()
        } else if sec.contains("editors") || n == "libreoffice" {
            "Applications".into()
        } else if sec.contains("admin") || sec.contains("base") || sec.contains("utils")
            || n == "htop" || n == "tmux" || n == "tree" || n == "neovim"
            || n == "ripgrep" || n == "docker.io"
        {
            "Tools".into()
        } else {
            "Applications".into()
        }
    }
}
