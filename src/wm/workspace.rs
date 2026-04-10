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
    pub rearrange_required: bool,
    pub focus_active_requested: bool,
    pub new_slide_id: u16,
}

impl Workspace {
    pub fn rearrange(&mut self) {
        for (index, slide) in self.slides.iter_mut().enumerate() {
            slide.coord = (
                self.coord.0,
                self.coord.1 + (index as i32) * self.dimensions.1,
            )
            // hi
        }
    }
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

    // TODO: what is this mess of if else
    // TODO: surely these functions don't need the global window?
    pub fn next_slide(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        let new_slide_index = self.active_slide + 1;
        if new_slide_index == self.slides.len() {
            if let Some(last_slide) = self.slides.get(self.slides.len() - 1) {
                if last_slide.windows.len() > 0 {
                    self.slides.push(Slide::new(self.new_slide_id));
                    self.new_slide_id += 1;
                } else {
                    return;
                }
            } else {
            }
        }
        self.active_slide += 1;
        self.child_rearrange(windows);
        self.focus_active_requested = true;
    }

    pub fn prev_slide(&mut self, windows: &mut HashMap<RiverWindowV1, Window>) {
        if self.active_slide == 0 {
            if let Some(first_slide) = self.slides.get(0) {
                if first_slide.windows.len() > 0 {
                    self.slides.insert(0, Slide::new(self.new_slide_id));
                    self.new_slide_id += 1;
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
            self.active_slide -= 1;
        }
        self.child_rearrange(windows);
        self.focus_active_requested = true;
    }
}
