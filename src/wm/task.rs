use crate::WindowManager;
use crate::wm::RiverWindowV1;
use crate::wm::utils::{Dimension, Position};

impl Task {
    pub fn step(&mut self, wm: &mut WindowManager, phase: Phase) -> bool {
        // if cfg!(debug_assertions) {
        //     println!("performing {:?}", self);
        // }
        match self {
            Task::SetWindowGeometry {
                window_id,
                pos,
                dim,
            } => {
                if phase == Phase::Manage {
                    let window = wm.windows.get_mut(window_id).expect("window not found!!");
                    window.set_position(pos.x, pos.y);
                    window.set_node_position(wm.camera_x, wm.camera_y);
                    let width = dim.width;
                    let height = dim.height;
                    (window.width, window.height) = (width, height);
                    window.proxy.propose_dimensions(width, height);
                    return true;
                } else {
                    return false;
                }
            }
            // Task::MoveWindow {
            //     window_id,
            //     diff_pos,
            //     // TODO: do I need these
            //     // started_at,
            //     // duration,
            //     ..
            // } => {
            //     if let Some(window) = wm.windows.get_mut(window_id) {
            //         window.set_position(window.x + diff_pos.x, window.y + diff_pos.y);
            //         window.set_node_position(wm.camera_x, wm.camera_y);
            //     }
            //
            //     return true;
            // }
            // Task::ResizeWindow {
            //     window_id,
            //     diff_dim,
            //     // TODO: do I need these
            //     // started_at,
            //     // duration,
            //     ..
            // } => {
            //     if phase == Phase::Manage {
            //         if let Some(window) = wm.windows.get_mut(window_id) {
            //             let width = window.width + diff_dim.width;
            //             let height = window.height + diff_dim.height;
            //             (window.width, window.height) = (width, height);
            //             window.proxy.propose_dimensions(width, height);
            //         }
            //         return true;
            //     }
            //     return false;
            // }
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
                // TODO: remove all code that uses camera_x and camera_y?
                (wm.camera_x, wm.camera_y) = (position.x, position.y);

                // TODO: change position to position....
                for window in wm.windows.values_mut() {
                    window.set_node_position(position.x, position.y);
                }
                return true;
            }
            Task::FocusActive {} => {
                let slide = wm.desktop.active_workspace_mut().active_slide_mut();
                if !slide.windows.is_empty() && phase == Phase::Manage {
                    for seat in wm.seats.values_mut() {
                        seat.focus_window(&slide.windows[slide.active_window]);
                    }
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
    SetWindowGeometry {
        window_id: RiverWindowV1,
        pos: Position,
        dim: Dimension,
    },
    // MoveWindow {
    //     window_id: RiverWindowV1,
    //     diff_pos: Position,
    //     started_at: Instant,
    //     duration: Duration,
    // },
    // ResizeWindow {
    //     window_id: RiverWindowV1,
    //     diff_dim: Dimension,
    //     started_at: Instant,
    //     duration: Duration,
    // },
    // RelayoutWorkspace {
    //     workspace: u8,
    // },
    MaximizeWindow {
        window_id: RiverWindowV1,
    },
    MoveCamera {
        position: Position,
    },
    FocusActive {},
}
