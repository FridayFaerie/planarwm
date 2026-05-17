// SPDX-FileCopyrightText: © 2026 FridayFaerie
// SPDX-License-Identifier: 0BSD

mod actions;
mod config;
mod ipc;
mod process;
mod protocol;
mod requests;
mod wm;

use crate::config::{Config, load_config};
use crate::ipc::{IpcState, MainRequest, MainResponse};
use crate::river::{
    river_input_manager_v1::RiverInputManagerV1, river_layer_shell_v1::RiverLayerShellV1,
    river_libinput_config_v1::RiverLibinputConfigV1, river_window_manager_v1::RiverWindowManagerV1,
    river_xkb_bindings_v1::RiverXkbBindingsV1,
};
use crate::wm::task::Task;
use crate::wm::{Output, Window, WindowManager};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::eventfd::{EfdFlags, EventFd};
use process::spawn_shell;
use std::fmt::Debug;
use std::io::Error;
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use wayland_client::Connection;

pub use protocol::river;

#[derive(Debug)]
struct AppData {
    config: Config,
    river_wm: Option<RiverWindowManagerV1>,
    river_xkb: Option<RiverXkbBindingsV1>,
    river_ls: Option<RiverLayerShellV1>,
    river_im: Option<RiverInputManagerV1>,
    river_lc: Option<RiverLibinputConfigV1>,
    wm: WindowManager,

    ipc_tx: Sender<MainResponse>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Queue up a get_registry event.
    let conn = Connection::connect_to_env()?;
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let _registry = display.get_registry(&event_queue.handle(), ());

    let config = load_config();

    // IPC things
    let (to_main_tx, to_main_rx) = mpsc::channel::<MainRequest>();
    let (from_main_tx, from_main_rx) = mpsc::channel::<MainResponse>();
    let waker = Arc::new(EventFd::from_value_and_flags(0, EfdFlags::EFD_NONBLOCK)?);

    let socket_path;
    if cfg!(debug_assertions) {
        socket_path = PathBuf::from(format!(
            "{}/planarwm-debug.sock",
            std::env::var("XDG_RUNTIME_DIR")?
        ));
    } else {
        socket_path = PathBuf::from(format!(
            "{}/planarwm.sock",
            std::env::var("XDG_RUNTIME_DIR")?
        ));
    }

    let _ipc_thread =
        ipc::spawn_ipc_thread(socket_path, to_main_tx, from_main_rx, Arc::clone(&waker));

    // Initial state
    // TODO: I can probably split off this config section and not use clone someday
    let mut app_data = AppData {
        config,
        river_wm: None,
        river_xkb: None,
        river_ls: None,
        river_im: None,
        river_lc: None,
        wm: WindowManager::new(from_main_tx.clone()),

        ipc_tx: from_main_tx,
    };

    // Roundtrip to process the get_registry event and bind interfaces.
    event_queue.roundtrip(&mut app_data)?;
    if app_data.river_wm.is_none() {
        eprintln!("river_window_manager_v1 global not found! Is river running?");
        std::process::exit(1);
    }
    if app_data.river_xkb.is_none() {
        eprintln!("river_xkb_bindings_v1 global not found! Is river running with xkb support?");
        std::process::exit(1);
    }

    for program in &app_data.config.startup {
        spawn_shell(program)
    }

    loop {
        event_queue.dispatch_pending(&mut app_data)?;

        conn.flush()?;

        let read_guard = conn
            .prepare_read()
            .ok_or_else(|| Error::other("prepare_read returned None"))?;

        let (river_ready, wake_ready) = {
            let mut fds = [
                PollFd::new(read_guard.connection_fd(), PollFlags::POLLIN),
                PollFd::new(waker.as_fd(), PollFlags::POLLIN),
            ];

            poll(&mut fds, PollTimeout::NONE)?;
            // poll(&mut fds, PollTimeout::from(1000u16))?;

            (
                fds[0]
                    .revents()
                    .unwrap_or(PollFlags::empty())
                    .contains(PollFlags::POLLIN),
                fds[1]
                    .revents()
                    .unwrap_or(PollFlags::empty())
                    .contains(PollFlags::POLLIN),
            )
        };

        if river_ready {
            read_guard.read()?;
            event_queue.dispatch_pending(&mut app_data)?;
        }

        if wake_ready {
            let _ = waker.read();
            requests::drain_main_requests(&mut app_data, &to_main_rx)?;
        }
    }
}
