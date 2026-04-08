use crate::Window;
use crate::wm::HashMap;
use crate::wm::RiverWindowV1;
use crate::wm::slide::Slide;
use crate::wm::utils::Rect;

#[derive(Debug, Default)]
pub struct Workspace {
    pub id: String,
    pub coord: (i32, i32),
    pub dimensions: (i32, i32),
    pub slides: Vec<Slide>,
    pub active_slide: usize,
    pub child_rearrange_required: bool,
    pub new_slide_id: u16,
}

impl Workspace {
    // TODO: split this into rearrange child's windows within the slide, and to rearrange the
    // slides
    pub fn child_rearrange(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        for (index, slide) in self.slides.iter_mut().enumerate() {
            println!(
                "TODO: rearranging slide {}, for myself at {}",
                index, self.coord.1
            );
            if slide.rearrange_required {
                let bounds = Rect {
                    x: self.coord.0,
                    y: self.coord.1 + self.dimensions.1 * (index as i32),
                    width: self.dimensions.0,
                    height: self.dimensions.1,
                };
                slide.compute_targets(bounds, windows);
                slide.rearrange_required = false;
            }
        }
        self.child_rearrange_required = false;
    }

    pub fn active_slide_mut(&mut self) -> &mut Slide {
        self.slides.get_mut(self.active_slide).unwrap()
    }

    // TODO: surely these functions don't need the global window?
    pub fn next_slide(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        let new_slide_index = self.active_slide + 1;
        if new_slide_index >= self.slides.len() {
            self.slides.push(Slide::new(self.new_slide_id));
            self.new_slide_id += 1;
        }
        self.active_slide += 1;
        self.child_rearrange(windows);
    }

    pub fn prev_slide(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        if self.active_slide == 0 {
            self.slides.insert(0, Slide::new(self.new_slide_id));
            self.new_slide_id += 1;
        } else {
            self.active_slide -= 1;
        }
        self.child_rearrange(windows);
    }
}
