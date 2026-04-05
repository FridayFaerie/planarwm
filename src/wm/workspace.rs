use crate::wm::slide::Slide;

#[derive(Debug, Default)]
pub struct Workspace {
    pub id: String,
    pub coord: (i32, i32),
    pub slides: Vec<Slide>,
    pub focused_slide: usize,
}
