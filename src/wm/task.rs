use crate::WindowManager;
use crate::wm::utils::{Dimension, Position};
use crate::wm::{RiverWindowV1, utils::Rect};
use std::time::{Duration, Instant};

impl Task {
    pub fn step(&mut self, wm: &mut WindowManager, phase: Phase, now: Instant) -> bool {
        match self {
            Task::MoveWindow {
                window_id,
                diff_pos,
                started_at,
                duration,
            } => {
                if let Some(window) = wm.windows.get_mut(window_id) {
                    window.set_position(window.x + diff_pos.x, window.y + diff_pos.y);
                    window.set_node_position(wm.camera_x, wm.camera_y);
                }

                return true;
            }
            Task::ResizeWindow {
                window_id,
                diff_dim,
                started_at,
                duration,
            } => {
                if phase == Phase::Manage {
                    if let Some(window) = wm.windows.get_mut(window_id) {
                        let width = window.width + diff_dim.width;
                        let height = window.height + diff_dim.height;
                        (window.width, window.height) = (width, height);
                        window.proxy.propose_dimensions(width, height);
                    }
                    return true;
                }
                return false;
            }
            Task::MaximizeWindow { window_id } => {
                if phase == Phase::Manage
                    && let Some(window) = wm.windows.get_mut(window_id)
                {
                    if window.maximized {
                        // TODO: write this code
                    } else {
                        if let Some((width, height)) =
                            wm.outputs.values().find_map(|output| output.dimensions)
                        {
                            (window.x, window.y) = (wm.camera_x, wm.camera_y);
                            (window.width, window.height) = (width, height);
                            window.proxy.propose_dimensions(width, height);
                            window.node.set_position(wm.camera_x, wm.camera_y);
                            // NOTE: not informing because they're already maximized :)
                            // window.proxy.inform_maximized()
                        }
                    }
                    return true;
                }
                return false;
            }
            Task::MoveCamera { position } => {
                println!("handling task::MoveCamera! target y: {}", position.y);
                // TODO: remove
                // (wm.camera_x, wm.camera_y) = (position.x, position.y);

                // TODO: change position to position....
                for window in wm.windows.values_mut() {
                    window.set_node_position(position.x, position.y);
                }
                return true;
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Render,
    Manage,
}

#[derive(Debug)]
pub enum Task {
    MoveWindow {
        window_id: RiverWindowV1,
        diff_pos: Position,
        started_at: Instant,
        duration: Duration,
    },

    ResizeWindow {
        window_id: RiverWindowV1,
        diff_dim: Dimension,
        started_at: Instant,
        duration: Duration,
    },
    // RelayoutWorkspace {
    //     workspace: u8,
    // },
    MaximizeWindow {
        window_id: RiverWindowV1,
    },
    MoveCamera {
        position: Position,
    },
}
