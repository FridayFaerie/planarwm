use crate::Window;
use crate::wm::{HashMap, RiverWindowV1, utils::Rect};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub enum SlideType {
    #[default]
    Master,
    Dwindle,
    Floating,
    VerticalScroll,
    HorizontalScroll,
}

#[derive(Debug, Default)]
pub struct Slide {
    pub id: u16,
    pub slide_type: SlideType,
    pub coord: (i32, i32),
    pub windows: Vec<RiverWindowV1>,
    pub active_window: usize,
    pub rearrange_required: bool,
}

impl Slide {
    pub fn new(id: u16) -> Self {
        Self {
            id: id,
            slide_type: SlideType::Master,
            coord: (0, 0),
            windows: Vec::new(),
            active_window: 0,
            rearrange_required: true,
        }
    }

    pub fn attach_window(&mut self, window_id: RiverWindowV1) {
        if !self.windows.contains(&window_id) {
            self.windows.push(window_id);
            self.rearrange_required = true;
        }
    }

    pub fn compute_targets(&self, bounds: Rect, windows: &mut HashMap<RiverWindowV1, Window>) {
        match self.slide_type {
            SlideType::Master => self.compute_master_targets(bounds, windows),
            _ => {}
        }
    }

    fn compute_master_targets(&self, bounds: Rect, windows: &mut HashMap<RiverWindowV1, Window>) {
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
