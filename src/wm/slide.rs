use std::sync::mpsc::Sender;

use crate::wm::task::Task;
use crate::wm::utils::{Dimension, Position};
use crate::wm::{RiverWindowV1, utils::Rect};
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

#[derive(Debug)]
pub struct Slide {
    pub id: u16,
    pub slide_type: SlideType,
    pub position: (i32, i32),
    pub dimensions: (i32, i32),
    pub windows: Vec<RiverWindowV1>,
    pub active_window: usize,
    pub rearrange_required: bool,
    queue_tx: Sender<Task>,
}

impl Slide {
    pub fn new(id: u16, dimensions: (i32, i32), queue_tx: Sender<Task>) -> Self {
        Self {
            id,
            slide_type: SlideType::VerticalScroll,
            position: (0, 0),
            dimensions,
            windows: Vec::new(),
            active_window: 0,
            rearrange_required: true,
            queue_tx,
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

    // TODO: add config to loop?
    pub fn next_window(&mut self) {
        if self.active_window + 1 < self.windows.len() {
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

    pub fn rearrange(&mut self) {
        if self.active_window >= self.windows.len() {
            self.active_window = self.windows.len() - 1;
        }

        let bounds = Rect {
            x: self.position.0,
            y: self.position.1,
            width: self.dimensions.0,
            height: self.dimensions.1,
        };
        match self.slide_type {
            SlideType::Master => self.master_rearrange(bounds),
            SlideType::VerticalScroll => self.vertscroll_rearrange(bounds),
            _ => {}
        }
    }

    fn vertscroll_rearrange(&self, bounds: Rect) {
        let slide_size = self.windows.len();
        let outer_gaps = 20;
        let inner_gaps = 10;
        let window_width = bounds.width - 2 * outer_gaps;
        let window_height = bounds.height - 2 * outer_gaps;

        if slide_size == 0 {
            return;
        };

        let active_index = self.active_window;

        for index in 0..active_index {
            let x = (bounds.x + outer_gaps)
                + (window_width + inner_gaps) * (index as i32 - active_index as i32);
            let y = bounds.y + outer_gaps;
            self.queue_tx
                .send(Task::SetWindowGeometry {
                    window_id: self.windows[index].clone(),
                    pos: Position { x: x, y: y },
                    dim: Dimension {
                        width: window_width,
                        height: window_height,
                    },
                })
                .expect("couldn't send window geometry...");
        }

        for index in active_index..self.windows.len() {
            let x = (bounds.x + outer_gaps)
                + (window_width + inner_gaps) * (index as i32 - active_index as i32);
            let y = bounds.y + outer_gaps;
            self.queue_tx
                .send(Task::SetWindowGeometry {
                    window_id: self.windows[index].clone(),
                    pos: Position { x: x, y: y },
                    dim: Dimension {
                        width: window_width,
                        height: window_height,
                    },
                })
                .expect("couldn't send window geometry...");
        }
    }

    fn master_rearrange(&self, bounds: Rect) {
        let slide_size = self.windows.len();

        if slide_size == 0 {
            return;
        }

        let master_w = if slide_size > 1 {
            bounds.width / 2
        } else {
            bounds.width
        };

        for (index, _) in self.windows.iter().enumerate() {
            if index == 0 {
                self.queue_tx
                    .send(Task::SetWindowGeometry {
                        window_id: self.windows[index].clone(),
                        pos: Position {
                            x: bounds.x,
                            y: bounds.y,
                        },
                        dim: Dimension {
                            width: master_w,
                            height: bounds.height,
                        },
                    })
                    .expect("couldn't send window geometry...");
            } else {
                let stack_size = (slide_size - 1) as i32;
                let stack_h = bounds.height / stack_size;

                self.queue_tx
                    .send(Task::SetWindowGeometry {
                        window_id: self.windows[index].clone(),
                        pos: Position {
                            x: bounds.x + master_w,
                            y: bounds.y + ((index as i32 - 1) * stack_h),
                        },
                        dim: Dimension {
                            width: bounds.width - master_w,
                            height: stack_h,
                        },
                    })
                    .expect("couldn't send window geometry...");
            }
        }
    }
}
