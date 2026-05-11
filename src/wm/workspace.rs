use crate::Window;
use crate::wm::HashMap;
use crate::wm::RiverWindowV1;
use crate::wm::slide::Slide;

#[derive(Debug, Default)]
pub struct Workspace {
    pub id: String,
    pub coord: (i32, i32),
    // TODO: remove dimensions, workspace shouldn't have dimensions - need to have a "center"
    // position instead?
    pub dimensions: (i32, i32),
    pub slides: Vec<Slide>,
    pub active_slide: usize,
    pub child_rearrange_required: bool,
    pub rearrange_required: bool,
    pub focus_active_requested: bool,
    pub new_slide_id: u16,
}

impl Workspace {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            coord: (0, 0),
            dimensions: (0, 0),
            slides: vec![Slide::new(0, (0, 0))],
            active_slide: 0,
            child_rearrange_required: true,
            rearrange_required: true,
            focus_active_requested: false,
            new_slide_id: 1,
        }
    }
    pub fn rearrange(&mut self) {
        for (index, slide) in self.slides.iter_mut().enumerate() {
            slide.position = (
                self.coord.0,
                self.coord.1 + (index as i32) * self.dimensions.1,
            );
        }
    }

    pub fn active_slide_mut(&mut self) -> &mut Slide {
        self.slides.get_mut(self.active_slide).unwrap()
    }

    pub fn moveto_next_slide(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        let active_slide = self
            .slides
            .get_mut(self.active_slide)
            .unwrap_or_else(|| panic!("can't find active slide!!"));
        let window_id = active_slide.windows.remove(active_slide.active_window);
        active_slide.rearrange_required = true;
        self.child_rearrange_required = true;
        self.next_slide();
        let active_slide = self
            .slides
            .get_mut(self.active_slide)
            .unwrap_or_else(|| panic!("can't find the active slide"));
        active_slide.attach_window(window_id.clone());

        if let Some(window) = windows.get_mut(&window_id) {
            window.location.as_mut().unwrap().slide_id = active_slide.id;
        }
    }

    pub fn moveto_prev_slide(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        let active_slide = self
            .slides
            .get_mut(self.active_slide)
            .unwrap_or_else(|| panic!("can't find active slide!!"));
        let window_id = active_slide.windows.remove(active_slide.active_window);
        active_slide.rearrange_required = true;
        self.child_rearrange_required = true;
        self.prev_slide();
        let active_slide = self
            .slides
            .get_mut(self.active_slide)
            .unwrap_or_else(|| panic!("can't find the active slide"));
        active_slide.attach_window(window_id.clone());

        if let Some(window) = windows.get_mut(&window_id) {
            window.location.as_mut().unwrap().slide_id = active_slide.id;
        }
    }

    pub fn next_slide(&mut self) {
        let current_slide = self.slides.get(self.active_slide).unwrap();
        let new_slide_index = self.active_slide + 1;
        if new_slide_index == self.slides.len() {
            if current_slide.windows.is_empty() {
                return;
            } else {
                self.slides
                    .push(Slide::new(self.new_slide_id, self.dimensions));
                self.new_slide_id += 1;
                self.rearrange();
            }
        } else if current_slide.windows.is_empty() {
            self.slides.remove(self.active_slide);
            self.rearrange();
        }
        self.active_slide += 1;
    }

    pub fn prev_slide(&mut self) {
        if self.active_slide == 0 {
            let first_slide = self.slides.first().unwrap();
            if !first_slide.windows.is_empty() {
                self.slides
                    .insert(0, Slide::new(self.new_slide_id, self.dimensions));
                self.new_slide_id += 1;
            } else {
                return;
            }
        } else {
            // TODO: not strictly needed - should I remove?
            if let Some(original_slide) = self.slides.get(self.active_slide)
                && original_slide.windows.is_empty()
            {
                self.slides.remove(self.active_slide);
            }
            self.active_slide -= 1;
        }
        self.rearrange();
    }
}
