use wayland_backend::client::ObjectId;
use wayland_client::Proxy;

use crate::WindowManager;
use crate::ipc::ClientId;
use crate::wm::SeatOp;
use crate::wm::utils::{Dimension, Position};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

impl Task {
    pub fn step(&mut self, wm: &mut WindowManager, phase: Phase, queue_tx: Sender<Task>) -> bool {
        // println!("------\nperforming {:?}", self);
        match self {
            Task::CloseWindow { window_id } => {
                if phase == Phase::Manage {
                    for seat in wm.seats.values_mut() {
                        if let SeatOp::Move {
                            window_id: op_id, ..
                        }
                        | SeatOp::Resize {
                            window_id: op_id, ..
                        } = &seat.op
                            && op_id == window_id
                        {
                            seat.op_end(&mut wm.windows);
                        }
                    }
                    if let Some(window) = wm.windows.get_mut(window_id)
                        && let Some(loc) = &window.location
                        && let Some(workspace) = wm.desktop.workspaces.get_mut(&loc.workspace_id)
                        && let Some(slide) =
                            workspace.slides.iter_mut().find(|s| s.id == loc.slide_id)
                        && let Some(window_index) =
                            slide.windows.iter_mut().position(|w| w == window_id)
                    {
                        slide.windows.remove(window_index);
                        if !slide.windows.is_empty() {
                            slide.rearrange();
                            for seat in wm.seats.values_mut() {
                                seat.focus_window(
                                    &slide.windows[slide.active_window],
                                    &mut wm.windows,
                                )
                            }
                        } else if let Some(slide_pos) = workspace
                            .slides
                            .iter_mut()
                            .position(|s| s.id == loc.slide_id)
                            && !workspace.slides.is_empty()
                        {
                            workspace.slides.remove(slide_pos);
                            if workspace.active_slide == workspace.slides.len()
                                && workspace.active_slide != 0
                            {
                                workspace.active_slide -= 1;
                                if let Some(slide) =
                                    workspace.slides.get_mut(workspace.active_slide)
                                {
                                    for seat in wm.seats.values_mut() {
                                        seat.focus_window(
                                            &slide.windows[slide.active_window],
                                            &mut wm.windows,
                                        )
                                    }
                                    queue_tx
                                        .send(Task::SetCamera {
                                            pos: slide.position,
                                            timer: Instant::now(),
                                        })
                                        .expect("couldn't send setcamera");
                                }
                            }
                            workspace.rearrange();
                        }
                    }
                    wm.windows[window_id].proxy.close();
                    wm.windows.remove(window_id);
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
                    // TODO: allow config?
                    let duration = (((diff_pos.x.pow(2) + diff_pos.y.pow(2)) as f32).powf(0.25)
                        * 3.0
                        + 50.0) as u64;
                    window.target_position = *pos;
                    if (diff_pos != Position { x: 0, y: 0 }) {
                        queue_tx
                            .send(Task::MoveWindow {
                                window_id: window_id.clone(),
                                diff_pos,
                                timer: *timer,
                                duration: Duration::from_millis(duration),
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
                        window.render_position += diff_pos;
                        return true;
                    }

                    let t = elapsed.as_millis() as f32 / duration.as_millis() as f32;
                    // let smooth_t = t;
                    let smooth_t = t * t * (3.0 - 2.0 * t);
                    let partial_diff_pos = *diff_pos * smooth_t;

                    window.render_position += partial_diff_pos;

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
                        if width == 0 || height == 0 {
                            // TODO: fix
                            eprintln!("width or height tried to be 0! that's wrong");
                            return false;
                        }
                        window.proxy.propose_dimensions(width, height);
                    }
                    return true;
                }
                false
            }
            // TODO: block other changes if fullscreened
            Task::MaximizeWindow { window_id } => {
                if phase != Phase::Manage {
                    return false;
                }
                if let Some(window) = wm.windows.get_mut(window_id) {
                    if window.maximized {
                        if let Some(loc) = &window.location
                            && let Some(workspace) =
                                wm.desktop.workspaces.get_mut(&loc.workspace_id)
                            && let Some(slide) =
                                workspace.slides.iter_mut().find(|s| s.id == loc.slide_id)
                        {
                            slide.rearrange();
                        }
                        window.proxy.exit_fullscreen();
                        window.maximized = false;
                    } else {
                        if let Some(output) = wm.outputs.values().last() {
                            window.proxy.fullscreen(&output.proxy);
                            window.maximized = true;
                            // TODO: switch to dimensions
                            window.width = output.dimensions.unwrap().0;
                            window.height = output.dimensions.unwrap().1;
                        }
                    }
                }
                true
            }
            Task::SetCameraOffset { pos } => {
                wm.camera_offset = *pos;
                // TODO: this is a nasty hack..
                wm.rendered_camera_pos = Position { x: -1, y: -1 };
                true
            }
            Task::SetCamera { pos, timer } => {
                let diff_pos = *pos - wm.target_camera_pos;
                // TODO: allow it to be configurable?
                let duration = (((diff_pos.x.pow(2) + diff_pos.y.pow(2)) as f32).powf(0.25) * 3.0
                    + 50.0) as u64;
                wm.target_camera_pos = *pos;
                if (diff_pos != Position { x: 0, y: 0 }) {
                    queue_tx
                        .send(Task::MoveCamera {
                            diff_pos,
                            timer: *timer,
                            duration: Duration::from_millis(duration),
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
                    wm.render_camera_pos += diff_pos;
                    return true;
                }

                let t = elapsed.as_millis() as f32 / duration.as_millis() as f32;
                // let smooth_t = t;
                let smooth_t = t * t * (3.0 - 2.0 * t);
                let partial_diff_pos = *diff_pos * smooth_t;

                wm.render_camera_pos += partial_diff_pos;
                false
            }
            Task::SetPointer { pos } => {
                for seat in wm.seats.values_mut() {
                    let old_pos = seat.pointer_position;
                    let pos_diff = *pos - old_pos;

                    // TODO: THIS IS HORRRRRRIBLE
                    if pos_diff.x.abs() + pos_diff.y.abs() < 100 {
                        return true;
                    }

                    seat.proxy.pointer_warp(pos.x, pos.y);
                    if let SeatOp::Pan {
                        start_camera_pos: old_start,
                    } = seat.op
                    {
                        seat.op = SeatOp::Pan {
                            start_camera_pos: old_start + pos_diff * 2.0,
                        };
                        seat.op_diff = pos_diff;
                    }
                }
                true
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
            Task::UnWatchRequest { .. } => {
                eprintln!("TODO: implement UnWatchRequest");
                true
            }
            Task::WatchRequest { .. } => {
                // state
                //     .ipc
                //     .watchers
                //     .entry(app_id.clone())
                //     .or_default()
                //     .insert(client_id);
                //
                // let geom = app.wm.geometry_for_appid(&app_id);
                //
                // ipc.send_reply(
                //     client_id,
                //     IpcResponse::Snapshot {
                //         request_id,
                //         app_id,
                //         window: geom,
                //     },
                // );
                eprintln!("TODO: implement WatchRequest");
                true
            }
            Task::FocusWindow { window_id } => {
                if let Some((_, window)) = wm
                    .windows
                    .iter_mut()
                    .find(|(id, _)| id.to_string() == *window_id)
                    && let Some(loc) = &window.location
                    && let Some(workspace) = wm.desktop.workspaces.get_mut(&loc.workspace_id)
                {
                    workspace.active_slide = workspace
                        .slides
                        .iter()
                        .position(|s| s.id == loc.slide_id)
                        .expect("slide is not in the workspace the window says it is!");
                    workspace.rearrange();
                    let position = workspace.slides[workspace.active_slide].position;
                    queue_tx
                        .send(Task::SetCamera {
                            pos: position,
                            timer: Instant::now(),
                        })
                        .expect("couldn't send prevslide's setcamera");
                    if let Some(slide) = workspace.slides.iter_mut().find(|s| s.id == loc.slide_id)
                    {
                        slide.active_window = slide
                            .windows
                            .iter()
                            .position(|id| id.to_string() == *window_id)
                            .expect("window not in slide it says it is!");
                        slide.rearrange();
                        for seat in wm.seats.values_mut() {
                            seat.focus_window(&slide.windows[slide.active_window], &mut wm.windows)
                        }
                    }
                }
                true
            }
            Task::InitNewOutput { id } => {
                if let Some(output) = wm.outputs.get_mut(id)
                    && let Some(layer_shell_output) = output.layer.as_mut()
                {
                    layer_shell_output.set_default();
                }
                true
            }
            Task::InitNewBackground { id } => {
                if let Some(output) = wm.outputs.get_mut(id)
                    && let Some(background) = output.background.as_mut()
                    && phase == Phase::Render
                {
                    // TODO: I'm place_bottom()ing more than I need to
                    background.node.place_bottom();
                    background.node.set_position(0, 0);
                    background.render(wm.render_camera_pos + wm.camera_offset);
                    background.sync_commit();

                    true
                } else {
                    false
                }
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
        window_id: ObjectId,
    },
    SetWindowGeometry {
        window_id: ObjectId,
        pos: Position,
        dim: Dimension,
        timer: Instant,
    },
    MoveWindow {
        window_id: ObjectId,
        diff_pos: Position,
        timer: Instant,
        duration: Duration,
    },
    ResizeWindow {
        window_id: ObjectId,
        dim: Dimension,
        timer: Instant,
        duration: Duration,
    },
    // RelayoutWorkspace {
    //     workspace: u8,
    // },
    MaximizeWindow {
        window_id: ObjectId,
    },
    SetCamera {
        pos: Position,
        timer: Instant,
    },
    SetCameraOffset {
        pos: Position,
    },
    MoveCamera {
        diff_pos: Position,
        timer: Instant,
        duration: Duration,
    },
    SetPointer {
        pos: Position,
    },
    // FocusActive {},

    // other externals
    UnWatchRequest {
        app_id: String,
    },
    WatchRequest {
        app_id: String,
        client_id: ClientId,
    },
    FocusWindow {
        window_id: String,
    },
    InitNewOutput {
        id: ObjectId,
    },
    InitNewBackground {
        id: ObjectId,
    },
}
