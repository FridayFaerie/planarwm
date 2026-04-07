use crate::wm::ObjectId;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub enum SlideType {
    #[default]
    Tiling,
    Floating,
    VerticalScroll,
    HorizontalScroll,
}

#[derive(Debug, Default)]
pub struct Slide {
    pub slide_type: SlideType,
    pub windows: Vec<ObjectId>,
    pub focused_window: usize,
}
