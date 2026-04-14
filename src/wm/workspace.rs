use crate::Window;
use crate::wm::HashMap;
use crate::wm::RiverWindowV1;
use crate::wm::slide::Slide;

#[derive(Debug, Default)]
pub struct Workspace {
    pub id: String,
    pub coord: (i32, i32),
    // TODO: remove dimensions, workspace shouldn't have dimensions
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
    // TODO: split this into rearrange child's windows within the slide, and to rearrange the
    // slides
    // TODO: Or just remove this entirely, it's bad
    pub fn child_rearrange(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        for (index, slide) in self.slides.iter_mut().enumerate() {
            if slide.focus_nearest_required {
                slide.focus_nearest();
                slide.focus_nearest_required = false;
            }
            if slide.rearrange_required {
                slide.position = (
                    self.coord.0,
                    self.coord.1 + self.dimensions.1 * (index as i32),
                );
                slide.rearrange(windows);
                slide.rearrange_required = false;
            }
        }
        self.child_rearrange_required = false;
    }

    pub fn active_slide_mut(&mut self) -> &mut Slide {
        self.slides.get_mut(self.active_slide).unwrap()
    }

    // TODO: when next_slide without any windows, remove the slide. Alternatively, remove slide on
    // window delete
    // TODO: what is this mess of if else
    // TODO: surely these functions don't need the global window?
    pub fn next_slide(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        let new_slide_index = self.active_slide + 1;
        if new_slide_index == self.slides.len()
            && let Some(last_slide) = self.slides.last()
        {
            if !last_slide.windows.is_empty() {
                self.slides
                    .push(Slide::new(self.new_slide_id, self.dimensions));
                self.new_slide_id += 1;
            } else {
                return;
            }
        }
        if let Some(previous_slide) = self.slides.get(self.active_slide) {
            if previous_slide.windows.is_empty() {
                self.slides.remove(self.active_slide);
                // TODO: refactor this away
                for slide in self.slides.iter_mut() {
                    slide.rearrange_required = true;
                }
            } else {
                self.active_slide += 1;
            }
        }
        self.child_rearrange(windows);
        self.focus_active_requested = true;
    }

    pub fn prev_slide(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        if self.active_slide == 0 {
            if let Some(first_slide) = self.slides.first() {
                if !first_slide.windows.is_empty() {
                    self.slides
                        .insert(0, Slide::new(self.new_slide_id, self.dimensions));
                    self.new_slide_id += 1;
                    // TODO: refactor this away
                    for slide in self.slides.iter_mut() {
                        slide.rearrange_required = true;
                    }
                } else {
                    return;
                }
            } else {
                eprintln!("can't find first slide!");
            }
        } else {
            // TODO: not strictly needed - should I remove?
            if let Some(original_slide) = self.slides.get(self.active_slide)
                && original_slide.windows.is_empty()
            {
                self.slides.remove(self.active_slide);
                // TODO: refactor this away
                for slide in self.slides.iter_mut() {
                    slide.rearrange_required = true;
                }
            }
            self.active_slide -= 1;
        }
        self.child_rearrange(windows);
        self.focus_active_requested = true;
    }
}
