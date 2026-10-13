use std::collections::HashMap;
use crate::image::Image;

pub struct ImageCache {
    images: HashMap<String, Image>,
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
        }
    }

    pub fn load(&mut self, path: &str) -> Result<(), String> {
        if self.images.contains_key(path) {
            return Ok(());
        }
        let img = Image::load_png(path)?;
        self.images.insert(path.to_string(), img);
        Ok(())
    }

    pub fn get(&self, path: &str) -> Option<&Image> {
        self.images.get(path)
    }
}
