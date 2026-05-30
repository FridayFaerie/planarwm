use wayland_backend::client::ObjectId;

use crate::{
    AppData, MainResponse,
    ipc::{ClientId, MainRequest},
    wm::{Window, task::Task, utils::Dimension},
};
use std::{collections::HashMap, fmt::Display};

fn response_from_result<E: Display>(
    client_id: ClientId,
    request_id: u64,
    result: Result<(), E>,
) -> MainResponse {
    match result {
        Ok(()) => MainResponse::Ok {
            client_id,
            request_id,
        },
        Err(err) => MainResponse::Error {
            client_id,
            request_id,
            message: err.to_string(),
        },
    }
}

fn remove_client_from_watchers(state: &mut AppData, client_id: ClientId) {
    for set in state.wm.ipc.watchers.values_mut() {
        set.remove(&client_id);
    }
    state.wm.ipc.watchers.retain(|_, set| !set.is_empty());
}

pub fn drain_main_requests(
    state: &mut AppData,
    to_main_rx: &std::sync::mpsc::Receiver<MainRequest>,
) -> Result<(), Box<dyn std::error::Error>> {
    while let Ok(msg) = to_main_rx.try_recv() {
        match msg {
            MainRequest::TrackCamera {
                client_id,
                request_id,
            } => {
                // TODO: I don't ever remove watchers...
                state.wm.ipc.camera_watchers.insert(client_id);

                state.ipc_tx.send(MainResponse::Ok {
                    client_id,
                    request_id,
                })?;

                state.ipc_tx.send(MainResponse::CameraPosition {
                    client_id,
                    pos: state.wm.render_camera_pos,
                })?;
            }
            MainRequest::Watch {
                client_id,
                request_id,
                app_name,
            } => {
                let found_id = find_window_id(app_name.to_lowercase(), &mut state.wm.windows);
                if let Some(id) = found_id {
                    // TODO: I don't ever remove watchers...
                    state
                        .wm
                        .ipc
                        .watchers
                        .entry(id.clone())
                        .or_default()
                        .insert(client_id);

                    if let Some(window) = state.wm.windows.get_mut(&id) {
                        state.ipc_tx.send(MainResponse::OkWatch {
                            client_id,
                            window_id: id.to_string(),
                            request_id,
                        })?;
                        state.ipc_tx.send(MainResponse::Geometry {
                            client_id,
                            window_id: id.to_string(),
                            pos: window.render_position - state.wm.render_camera_pos,
                            // TODO: fix into window.dim
                            dim: Dimension {
                                width: window.width,
                                height: window.height,
                            },
                        })?;
                    }
                } else {
                    state.ipc_tx.send(MainResponse::Error {
                        client_id,
                        request_id,
                        message: format!("couldn't find requested window {}", app_name),
                    })?;
                }
            }

            MainRequest::Unwatch {
                client_id,
                request_id,
                window_id,
            } => {
                let remove_id = if let Some((id, set)) = state
                    .wm
                    .ipc
                    .watchers
                    .iter_mut()
                    .find(|(id, _)| id.to_string() == window_id)
                {
                    set.remove(&client_id);
                    if set.is_empty() {
                        Some(id.clone())
                    } else {
                        None
                    }
                } else {
                    println!("couldn't unwatch {}: not in watched list", window_id);
                    None
                };

                if let Some(id) = remove_id {
                    state.wm.ipc.watchers.remove(&id);
                }

                state.ipc_tx.send(MainResponse::Ok {
                    client_id,
                    request_id,
                })?;
            }

            MainRequest::Focus {
                client_id,
                request_id,
                window_id,
            } => {
                if let Some(proxy) = state.river_wm.as_ref() {
                    proxy.manage_dirty();
                } else {
                    eprintln!("couldn't send manage dirty request!");
                }

                let result = state.wm.queue_tx.send(Task::FocusWindow { window_id });

                let response = response_from_result(client_id, request_id, result);
                state.ipc_tx.send(response)?;
            }

            MainRequest::Disconnected { client_id } => {
                remove_client_from_watchers(state, client_id);
            }
        }
    }

    Ok(())
}

fn find_window_id(
    requested_string: String,
    windows: &mut HashMap<ObjectId, Window>,
) -> Option<ObjectId> {
    for (window_id, window) in windows.iter() {
        let window_app_id = &window.app_id.to_lowercase();
        let window_title = &window.title.to_lowercase();

        if window_app_id.contains(&requested_string)
            || window_title.contains(&requested_string)
            || requested_string.contains(window_app_id)
            || requested_string.contains(window_title)
        {
            return Some(window_id.clone());
        }
    }
    return None;
}
