use crate::wm::{ObjectId, VecDeque};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub enum SlideKind {
    #[default]
    Tiling,
    VerticalScroll,
    HorizontalScroll,
}

pub struct Slide {
    pub kind: SlideKind,
    pub windows: VecDeque<ObjectId>,
    pub focused_window: Option<ObjectId>,
}
