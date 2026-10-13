use std::fs;
use std::path::PathBuf;
use crate::layer::Layer;

#[derive(Debug, Clone)]
pub struct LayerManager {
    layers_dir: PathBuf,
    pub layers: Vec<Layer>,
    pub active_layer_name: Option<String>,
}

impl LayerManager {
    pub fn new() -> Self {
        let layers_dir = Self::find_layers_dir();

        let mut manager = LayerManager {
            layers_dir,
            layers: Vec::new(),
            active_layer_name: None,
        };

        if let Err(e) = fs::create_dir_all(&manager.layers_dir) {
            eprintln!("leviathan-ui: Warning: could not create layers dir {:?}: {}", manager.layers_dir, e);
        }

        manager.load_layers();
        manager
    }

    /// Determines the persistent layers directory.
    ///
    /// Priority:
    /// 1. `/mnt/data/leviathan/layers`  — writable QEMU data disk (survives reboots)
    /// 2. `/home/leviathan/.config/leviathan/layers` — home dir fallback
    fn find_layers_dir() -> PathBuf {
        // Prefer the persistent data disk mount point
        let persistent = PathBuf::from("/mnt/data/leviathan/layers");
        if persistent.parent().map(|p| p.exists()).unwrap_or(false)
            || fs::create_dir_all(&persistent).is_ok()
        {
            return persistent;
        }

        // Fallback: standard XDG config home
        let mut fallback = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/leviathan"));
        fallback.push(".config");
        fallback.push("leviathan");
        fallback.push("layers");
        fallback
    }

    pub fn load_layers(&mut self) {
        self.layers.clear();
        match fs::read_dir(&self.layers_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        match fs::read_to_string(&path) {
                            Ok(content) => {
                                match serde_json::from_str::<Layer>(&content) {
                                    Ok(layer) => self.layers.push(layer),
                                    Err(e) => {
                                        eprintln!("leviathan-ui: Skipping corrupt layer {:?}: {}", path, e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("leviathan-ui: Could not read layer file {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Directory doesn't exist or isn't readable - that's OK on first boot
            }
        }
    }

    /// Sanitizes a layer name to a safe filename component (no slashes, no dots, no spaces)
    fn safe_filename(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }

    pub fn save_layer(&mut self, layer: &Layer) -> Result<(), std::io::Error> {
        // Ensure directory exists (might not if data disk wasn't mounted at startup)
        fs::create_dir_all(&self.layers_dir)?;

        let filename = format!("{}.json", Self::safe_filename(&layer.name));
        let mut path = self.layers_dir.clone();
        path.push(&filename);

        let content = serde_json::to_string_pretty(layer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // Write atomically: write to .tmp then rename
        let mut tmp_path = path.clone();
        tmp_path.set_extension("json.tmp");
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, &path)?;

        // Update in-memory state
        if let Some(existing) = self.layers.iter_mut().find(|l| l.name == layer.name) {
            *existing = layer.clone();
        } else {
            self.layers.push(layer.clone());
        }

        eprintln!("leviathan-ui: Saved layer '{}' to {:?}", layer.name, path);
        Ok(())
    }

    pub fn delete_layer(&mut self, name: &str) -> Result<(), std::io::Error> {
        let filename = format!("{}.json", Self::safe_filename(name));
        let mut path = self.layers_dir.clone();
        path.push(filename);
        let _ = fs::remove_file(&path);

        self.layers.retain(|l| l.name != name);
        if self.active_layer_name.as_deref() == Some(name) {
            self.active_layer_name = None;
        }
        Ok(())
    }

    pub fn get_layer(&self, name: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.name == name)
    }

    pub fn get_layer_mut(&mut self, name: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.name == name)
    }

    pub fn set_active_layer(&mut self, name: &str) {
        if self.layers.iter().any(|l| l.name == name) {
            self.active_layer_name = Some(name.to_string());
        }
    }

    pub fn get_active_layer(&self) -> Option<&Layer> {
        self.active_layer_name.as_deref().and_then(|n| self.get_layer(n))
    }

    pub fn get_active_layer_mut(&mut self) -> Option<&mut Layer> {
        let name = self.active_layer_name.clone()?;
        self.layers.iter_mut().find(|l| l.name == name)
    }

    /// Returns the path where layers are stored (for informational display)
    pub fn layers_dir(&self) -> &std::path::Path {
        &self.layers_dir
    }
}