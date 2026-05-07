use crate::Window;
use crate::wm::{HashMap, RiverWindowV1, utils::Rect};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq)]
pub enum SlideType {
    #[default]
    Master,
    // Dwindle,
    Floating,
    VerticalScroll,
    // HorizontalScroll,
}

#[derive(Debug, Default)]
pub struct Slide {
    pub id: u16,
    pub slide_type: SlideType,
    pub position: (i32, i32),
    pub dimensions: (i32, i32),
    pub windows: Vec<RiverWindowV1>,
    pub active_window: usize,
    pub rearrange_required: bool,
    pub focus_nearest_required: bool,
}

impl Slide {
    pub fn new(id: u16, dimensions: (i32, i32)) -> Self {
        Self {
            id,
            slide_type: SlideType::VerticalScroll,
            position: (0, 0),
            dimensions,
            windows: Vec::new(),
            active_window: 0,
            rearrange_required: true,
            focus_nearest_required: false,
        }
    }

    pub fn attach_window(&mut self, window_id: RiverWindowV1) {
        if !self.windows.contains(&window_id) {
            self.windows.push(window_id);
            self.rearrange_required = true;
        }
        // TODO: This is not sustainable....
        self.active_window = self.windows.len() - 1;
    }

    pub fn focus_nearest(&mut self) {
        if self.active_window >= self.windows.len() && self.active_window != 0 {
            self.active_window = self.windows.len() - 1;
        }
    }

    // TODO: add config to loop?
    pub fn next_window(&mut self) {
        if self.active_window < self.windows.len() + 1 {
            self.active_window += 1;
        }
    }

    pub fn prev_window(&mut self) {
        if self.active_window > 0 {
            self.active_window -= 1;
        }
    }

    pub fn cycle_tiling(&mut self) {
        if self.slide_type == SlideType::VerticalScroll {
            self.slide_type = SlideType::Master
        } else if self.slide_type == SlideType::Master {
            self.slide_type = SlideType::Floating
        } else if self.slide_type == SlideType::Floating {
            self.slide_type = SlideType::VerticalScroll
        }
    }

    pub fn rearrange(&self, windows: &mut HashMap<RiverWindowV1, Window>) {
        let bounds = Rect {
            x: self.position.0,
            y: self.position.1,
            width: self.dimensions.0,
            height: self.dimensions.1,
        };
        match self.slide_type {
            SlideType::Master => self.master_rearrange(bounds, windows),
            SlideType::VerticalScroll => self.vertscroll_rearrange(bounds, windows),
            _ => {}
        }
    }

    fn vertscroll_rearrange(&self, bounds: Rect, windows: &mut HashMap<RiverWindowV1, Window>) {
        let slide_size = self.windows.len();
        let outer_gaps = 20;
        let inner_gaps = 10;
        let window_width = bounds.width - 2 * outer_gaps;
        let window_height = bounds.height - 2 * outer_gaps;

        if slide_size == 0 {
            return;
        };

        let active_index = self.active_window;

        for i in 0..active_index {
            let window = windows
                .get_mut(&self.windows[i])
                .expect("can't find window");
            window.set_target_geometry(Rect {
                x: (bounds.x + outer_gaps)
                    + (window_width + inner_gaps) * (i as i32 - active_index as i32),
                y: bounds.y + outer_gaps,
                width: window_width,
                height: window_height,
            });
        }

        for i in active_index..self.windows.len() {
            let window = windows
                .get_mut(&self.windows[i])
                .expect("can't find window");
            window.set_target_geometry(Rect {
                x: (bounds.x + outer_gaps)
                    + (window_width + inner_gaps) * (i as i32 - active_index as i32),
                y: bounds.y + outer_gaps,
                width: window_width,
                height: window_height,
            });
        }
    }

    fn master_rearrange(&self, bounds: Rect, windows: &mut HashMap<RiverWindowV1, Window>) {
        let slide_size = self.windows.len();
        if slide_size == 0 {
            return;
        }

        let master_w = if slide_size > 1 {
            bounds.width / 2
        } else {
            bounds.width
        };

        for (i, window_id) in self.windows.iter().enumerate() {
            let Some(window) = windows.get_mut(window_id) else {
                continue;
            };

            if i == 0 {
                window.set_target_geometry(Rect {
                    x: bounds.x,
                    y: bounds.y,
                    width: master_w,
                    height: bounds.height,
                });
            } else {
                let stack_size = (slide_size - 1) as i32;
                let stack_h = bounds.height / stack_size;

                window.set_target_geometry(Rect {
                    x: bounds.x + master_w,
                    y: bounds.y + ((i as i32 - 1) * stack_h),
                    width: bounds.width - master_w,
                    height: stack_h,
                });
            }
        }
    }
}
