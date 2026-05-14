use crate::WindowManager;
use crate::wm::RiverWindowV1;
use crate::wm::utils::{Dimension, Position};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

impl Task {
    pub fn step(&mut self, wm: &mut WindowManager, phase: Phase, queue_tx: Sender<Task>) -> bool {
        // if cfg!(debug_assertions) {
        //     println!("------");
        //     println!("performing {:?}", self);
        // }
        match self {
            Task::CloseWindow { window_id } => {
                if phase == Phase::Manage {
                    // TODO: include everything in lifecycle.rs under if window.closed
                    // TODO: make this use the window's slide, instead of the active slide
                    let slide = wm.desktop.active_workspace_mut().active_slide_mut();
                    window_id.close();
                    slide.windows.remove(slide.active_window);
                    slide.rearrange();
                    if !slide.windows.is_empty() {
                        for seat in wm.seats.values_mut() {
                            seat.focus_window(&slide.windows[slide.active_window])
                        }
                    };
                    return true;
                }
                return false;
            }
            Task::SetWindowGeometry {
                window_id,
                pos,
                dim,
                timer,
            } => {
                let window = wm.windows.get_mut(window_id).expect("window not found!!");
                let diff_pos = *pos - window.current_position;
                let diff_width = dim.width - window.width;
                let diff_height = dim.height - window.height;
                window.current_position = *pos;
                if (diff_pos != Position { x: 0, y: 0 }) {
                    queue_tx
                        .send(Task::MoveWindow {
                            window_id: window_id.clone(),
                            diff_pos,
                            timer: timer.clone(),
                            duration: Duration::from_secs_f32(0.4),
                        })
                        .expect("couldn't send movewindow");
                }
                // TODO: fix
                if diff_width != 0 || diff_height != 0 {
                    queue_tx
                        .send(Task::ResizeWindow {
                            window_id: window_id.clone(),
                            diff_dim: Dimension {
                                width: diff_width,
                                height: diff_height,
                            },
                            timer: timer.clone(),
                            duration: Duration::from_secs(0),
                        })
                        .expect("couldn't send resizewindow");
                }
                return true;
                // if phase == Phase::Manage {
                //     let width = dim.width;
                //     let height = dim.height;
                //     (window.width, window.height) = (width, height);
                //     window.proxy.propose_dimensions(width, height);
                //     return true;
                // } else {
                //     return false;
                // }
            }
            // animation to move window by diff_pos
            Task::MoveWindow {
                window_id,
                diff_pos,
                timer,
                duration,
            } => {
                if let Some(window) = wm.windows.get_mut(window_id) {
                    let elapsed = timer.elapsed();

                    if elapsed > *duration {
                        // TODO: why do I need a clone here
                        window.original_position += diff_pos.clone();
                        // window.set_node_position(wm.camera_x, wm.camera_y);
                        if let Some(mut render_position) = window.render_position {
                            render_position += diff_pos;
                        }
                        window.set_node_position(wm.camera_x, wm.camera_y);
                        return true;
                    }

                    let t = elapsed.as_millis() as f32 / duration.as_millis() as f32;
                    // let smooth_t = t;
                    let smooth_t = t * t * (3.0 - 2.0 * t);
                    let partial_diff_pos = *diff_pos * smooth_t;

                    if let Some(mut render_position) = window.render_position {
                        render_position += partial_diff_pos;
                        window.render_position = Some(render_position);
                    } else {
                        window.render_position = Some(window.original_position + partial_diff_pos);
                    }

                    return false;
                } else {
                    return true;
                }
            }
            Task::ResizeWindow {
                window_id,
                diff_dim,
                timer,
                duration,
            } => {
                if phase == Phase::Manage && timer.elapsed() > *duration {
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
                // TODO: remove all code that uses camera_x and camera_y?
                (wm.camera_x, wm.camera_y) = (position.x, position.y);

                // TODO: change position to position....
                for window in wm.windows.values_mut() {
                    window.set_node_position(position.x, position.y);
                }
                return true;
            } // TODO: maybe remove this?

              // Task::FocusActive {} => {
              //     let slide = wm.desktop.active_workspace_mut().active_slide_mut();
              //     if !slide.windows.is_empty() {
              //         for seat in wm.seats.values_mut() {
              //             seat.focus_window(&slide.windows[slide.active_window]);
              //         }
              //     }
              //     return true;
              // }
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
    CloseWindow {
        window_id: RiverWindowV1,
    },
    SetWindowGeometry {
        window_id: RiverWindowV1,
        pos: Position,
        dim: Dimension,
        timer: Instant,
    },
    MoveWindow {
        window_id: RiverWindowV1,
        diff_pos: Position,
        timer: Instant,
        duration: Duration,
    },
    ResizeWindow {
        window_id: RiverWindowV1,
        diff_dim: Dimension,
        timer: Instant,
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
    // FocusActive {},
}
