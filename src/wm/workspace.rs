use crate::wm::slide::Slide;

pub struct Workspace {
    pub id: String,
    pub coord: (i32, i32),
    pub sildes: Vec<Slide>,
    pub focused_slide: usize,
}
