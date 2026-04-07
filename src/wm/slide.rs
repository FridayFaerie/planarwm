use crate::wm::{ObjectId, VecDeque};

pub enum SlideKind {
    Tiling,
    VerticalScroll,
    HorizontalScroll,
}

pub struct Slide {
    pub kind: SlideKind,
    pub windows: VecDeque<ObjectId>,
    pub focused_window: Option<ObjectId>,
}
