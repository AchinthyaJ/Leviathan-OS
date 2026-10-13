use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerElement {
    pub id: String,
    pub element_type: ElementType,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub image_path: Option<String>,
    pub visibility: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElementType {
    Icon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub elements: Vec<LayerElement>,
}