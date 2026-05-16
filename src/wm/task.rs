use crate::WindowManager;
use crate::wm::utils::{Dimension, Position};
use crate::wm::{RiverWindowV1, SeatOp};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

impl Task {
    pub fn step(&mut self, wm: &mut WindowManager, phase: Phase, queue_tx: Sender<Task>) -> bool {
        // println!("------");
        // println!("performing {:?}", self);
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
                // TODO: SetWindowGeometry is sometimes called with a window that doesn't exist in
                // wm.windows - check what's causing that, and if I can ignore it like I am doing
                // right now
                if let Some(window) = wm.windows.get_mut(window_id) {
                    let diff_pos = *pos - window.target_position;
                    window.target_position = *pos;
                    if (diff_pos != Position { x: 0, y: 0 }) {
                        queue_tx
                            .send(Task::MoveWindow {
                                window_id: window_id.clone(),
                                diff_pos,
                                timer: *timer,
                                duration: Duration::from_millis(200),
                            })
                            .expect("couldn't send movewindow");
                    }
                    // TODO: add animations
                    if dim.width != window.width || dim.height != window.height {
                        queue_tx
                            .send(Task::ResizeWindow {
                                window_id: window_id.clone(),
                                dim: *dim,
                                timer: *timer,
                                duration: Duration::from_secs(0),
                            })
                            .expect("couldn't send resizewindow");
                    }
                }
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
                        if let Some(render_position) = window.render_position.as_mut() {
                            *render_position += diff_pos;
                        } else {
                            window.render_position = Some(window.original_position);
                        }
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
                        if let Some(window) = wm.windows.get_mut(window_id)
                            && let Some(loc) = &window.location
                            && let Some(workspace) =
                                wm.desktop.workspaces.get_mut(&loc.workspace_id)
                            && let Some(slide) =
                                workspace.slides.iter_mut().find(|s| s.id == loc.slide_id)
                        {
                            slide.rearrange();
                        }
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
            Task::SetCamera { pos, timer } => {
                let diff_pos = *pos - wm.target_camera_pos;
                wm.target_camera_pos = *pos;
                if (diff_pos != Position { x: 0, y: 0 }) {
                    queue_tx
                        .send(Task::MoveCamera {
                            diff_pos,
                            timer: *timer,
                            duration: Duration::from_millis(200),
                        })
                        .expect("couldn't send movewindow");
                }
                true
            }
            Task::MoveCamera {
                diff_pos,
                timer,
                duration,
            } => {
                let elapsed = timer.elapsed();

                if elapsed > *duration {
                    wm.camera_pos += *diff_pos;
                    if let Some(render_position) = wm.render_camera_pos.as_mut() {
                        *render_position += diff_pos;
                    } else {
                        wm.render_camera_pos = Some(wm.camera_pos);
                    }
                    return true;
                }

                let t = elapsed.as_millis() as f32 / duration.as_millis() as f32;
                // let smooth_t = t;
                let smooth_t = t * t * (3.0 - 2.0 * t);
                let partial_diff_pos = *diff_pos * smooth_t;

                if let Some(mut render_position) = wm.render_camera_pos {
                    render_position += partial_diff_pos;
                    wm.render_camera_pos = Some(render_position);
                } else {
                    wm.render_camera_pos = Some(wm.camera_pos + partial_diff_pos);
                }

                false
            }
            // TODO: maybe remove this?
            // Task::FocusActive {} => {
            //     let slide = wm.desktop.active_workspace_mut().active_slide_mut();
            //     if !slide.windows.is_empty() {
            //         for seat in wm.seats.values_mut() {
            //             seat.focus_window(&slide.windows[slide.active_window]);
            //         }
            //     }
            //     return true;
            // }
            Task::SetDefaultLayerShellOutput {} => {
                for output in wm.outputs.values_mut() {
                    if let Some(layer_shell_output) = output.layer.as_mut() {
                        layer_shell_output.set_default();
                    }
                }
                true
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
    SetCamera {
        pos: Position,
        timer: Instant,
    },
    MoveCamera {
        diff_pos: Position,
        timer: Instant,
        duration: Duration,
    },
    // FocusActive {},
    SetDefaultLayerShellOutput {},
}
