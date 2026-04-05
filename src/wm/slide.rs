use crate::wm::ObjectId;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub enum SlideKind {
    #[default]
    Tiling,
    VerticalScroll,
    HorizontalScroll,
}

#[derive(Debug, Default)]
pub struct Slide {
    pub kind: SlideKind,
    pub windows: Vec<ObjectId>,
    pub focused_window: usize,
}
