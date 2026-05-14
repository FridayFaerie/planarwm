use crate::WindowManager;
use crate::wm::utils::{Dimension, Position};
use crate::wm::{RiverWindowV1, SeatOp};
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
                    for seat in wm.seats.values_mut() {
                        if let SeatOp::Move { window_proxy, .. }
                        | SeatOp::Resize { window_proxy, .. } = &seat.op
                            && window_proxy == window_id
                        {
                            seat.op_end();
                        }
                    }
                    if let Some(window) = wm.windows.get_mut(window_id)
                        && let Some(loc) = &window.location
                        && let Some(workspace) = wm.desktop.workspaces.get_mut(&loc.workspace_id)
                        && let Some(slide) =
                            workspace.slides.iter_mut().find(|s| s.id == loc.slide_id)
                    {
                        slide.windows.remove(slide.active_window);
                        slide.rearrange();
                        if !slide.windows.is_empty() {
                            for seat in wm.seats.values_mut() {
                                seat.focus_window(&slide.windows[slide.active_window])
                            }
                        };
                    }
                    wm.windows.remove(window_id);
                    window_id.close();
                    return true;
                }
                false
            }
            Task::SetWindowGeometry {
                window_id,
                pos,
                dim,
                timer,
            } => {
                let window = wm.windows.get_mut(window_id).expect("window not found!!");
                let diff_pos = *pos - window.current_position;
                window.current_position = *pos;
                if (diff_pos != Position { x: 0, y: 0 }) {
                    queue_tx
                        .send(Task::MoveWindow {
                            window_id: window_id.clone(),
                            diff_pos,
                            timer: *timer,
                            duration: Duration::from_millis(300),
                        })
                        .expect("couldn't send movewindow");
                }
                // TODO: add animations
                queue_tx
                    .send(Task::ResizeWindow {
                        window_id: window_id.clone(),
                        dim: *dim,
                        timer: *timer,
                        duration: Duration::from_secs(0),
                    })
                    .expect("couldn't send resizewindow");
                true
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
                        window.original_position += *diff_pos;
                        // window.set_node_position(wm.camera_x, wm.camera_y);
                        if let Some(mut render_position) = window.render_position {
                            render_position += diff_pos;
                        }
                        window.set_node_position(wm.camera_pos);
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

                    false
                } else {
                    true
                }
            }
            Task::ResizeWindow {
                window_id,
                dim,
                timer,
                duration,
            } => {
                if phase == Phase::Manage && timer.elapsed() > *duration {
                    if let Some(window) = wm.windows.get_mut(window_id) {
                        let width = dim.width;
                        let height = dim.height;
                        (window.width, window.height) = (width, height);
                        window.proxy.propose_dimensions(width, height);
                    }
                    return true;
                }
                false
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
                            (window.x, window.y) = (wm.camera_pos.x, wm.camera_pos.y);
                            (window.width, window.height) = (width, height);
                            window.proxy.propose_dimensions(width, height);
                            window.node.set_position(wm.camera_pos.x, wm.camera_pos.y);
                            // NOTE: not informing because they're already maximized :)
                            // window.proxy.inform_maximized()
                        }
                    }
                    return true;
                }
                false
            }
            Task::MoveCamera { position } => {
                wm.camera_pos = *position;

                for window in wm.windows.values_mut() {
                    window.set_node_position(*position);
                }
                true
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
        dim: Dimension,
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
