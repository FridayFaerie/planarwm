use wayland_backend::client::ObjectId;

use crate::{
    AppData,
    ipc::{self, MainResponse},
    wm::{
        Window,
        task::Task,
        utils::{Position, Rect},
    },
};
use std::{
    collections::HashMap,
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
};

fn response_from_result<E: Display>(
    client_id: ipc::ClientId,
    request_id: u64,
    result: Result<(), E>,
) -> ipc::MainResponse {
    match result {
        Ok(()) => ipc::MainResponse::Ok {
            client_id,
            request_id,
        },
        Err(err) => ipc::MainResponse::Error {
            client_id,
            request_id,
            message: err.to_string(),
        },
    }
}

fn remove_client_from_watchers(state: &mut AppData, client_id: ipc::ClientId) {
    for set in state.wm.ipc.watchers.values_mut() {
        set.remove(&client_id);
    }
    state.wm.ipc.watchers.retain(|_, set| !set.is_empty());
}

pub fn drain_main_requests(
    state: &mut AppData,
    to_main_rx: &std::sync::mpsc::Receiver<ipc::MainRequest>,
) -> Result<(), Box<dyn std::error::Error>> {
    while let Ok(msg) = to_main_rx.try_recv() {
        match msg {
            ipc::MainRequest::Watch {
                client_id,
                request_id,
                app_id,
            } => {
                let found_id = find_window_id(app_id.to_lowercase(), &mut state.wm.windows);
                println!(
                    "processing watch request {} from client {}, asking about {}!",
                    request_id, client_id, app_id
                );
                if let Some(id) = found_id {
                    state
                        .wm
                        .ipc
                        .watchers
                        .entry(id.clone())
                        .or_default()
                        .insert(client_id);

                    if let Some(window) = state.wm.windows.get_mut(&id) {
                        state.ipc_tx.send(ipc::MainResponse::OkWatch {
                            client_id,
                            window_id: id.to_string(),
                            request_id,
                        })?;
                        state.ipc_tx.send(ipc::MainResponse::Geometry {
                            client_id,
                            window_id: id.to_string(),
                            center: window.get_vector_from(state.wm.camera_pos),
                        })?;
                    }
                } else {
                    state.ipc_tx.send(ipc::MainResponse::Error {
                        client_id,
                        request_id,
                        message: format!("couldn't find requested window {}", app_id),
                    })?;
                }
            }

            ipc::MainRequest::Unwatch {
                client_id,
                request_id,
                app_id,
            } => {
                let found_id = find_window_id(app_id.to_lowercase(), &mut state.wm.windows);
                if let Some(id) = found_id {
                    if let Some(set) = state.wm.ipc.watchers.get_mut(&id) {
                        set.remove(&client_id);
                        if set.is_empty() {
                            state.wm.ipc.watchers.remove(&id);
                        }
                    } else {
                        println!("couldn't unwatch {}: not in watched list", app_id);
                    }
                } else {
                    println!("couldn't unwatch {}: couldn't map to a window", app_id);
                }

                state.ipc_tx.send(MainResponse::Ok {
                    client_id,
                    request_id,
                })?;
            }

            ipc::MainRequest::Focus {
                client_id,
                request_id,
                app_id,
            } => {
                if let Some(proxy) = state.river_wm.as_ref() {
                    proxy.manage_dirty();
                } else {
                    eprintln!("couldn't send manage dirty request!");
                }

                let result = state.wm.queue_tx.send(Task::FocusWindow { app_id });

                let response = response_from_result(client_id, request_id, result);
                state.ipc_tx.send(response)?;
            }

            ipc::MainRequest::Disconnected { client_id } => {
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
