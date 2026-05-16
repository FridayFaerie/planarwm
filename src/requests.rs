use crate::{AppData, ipc, wm::task::Task};
use std::fmt::Display;

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
    for set in state.ipc.watchers.values_mut() {
        set.remove(&client_id);
    }
    state.ipc.watchers.retain(|_, set| !set.is_empty());
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
                println!(
                    "processing watch request {} from client {}, asking about {}!",
                    request_id, client_id, app_id
                );
                state
                    .ipc
                    .watchers
                    .entry(app_id.clone())
                    .or_default()
                    .insert(client_id);

                state.ipc_tx.send(ipc::MainResponse::Ok {
                    client_id,
                    request_id,
                })?;
            }

            ipc::MainRequest::Unwatch {
                client_id,
                request_id,
                app_id,
            } => {
                if let Some(set) = state.ipc.watchers.get_mut(&app_id) {
                    set.remove(&client_id);
                    if set.is_empty() {
                        state.ipc.watchers.remove(&app_id);
                    }
                }

                state.ipc_tx.send(ipc::MainResponse::Ok {
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
