// SPDX-FileCopyrightText: © 2026 FridayFaerie
// SPDX-License-Identifier: 0BSD

mod actions;
mod app;
mod config;
mod process;
mod protocol;
mod wm;

use std::fmt::Debug;
use wayland_client::Connection;

use crate::config::{Config, load_config};
use crate::river::{
    river_input_manager_v1::RiverInputManagerV1, river_layer_shell_v1::RiverLayerShellV1,
    river_libinput_config_v1::RiverLibinputConfigV1, river_window_manager_v1::RiverWindowManagerV1,
    river_xkb_bindings_v1::RiverXkbBindingsV1,
};
use crate::wm::{Output, Window, WindowManager};
use process::spawn_shell;

pub use protocol::river;

#[derive(Debug, Default)]
struct AppData {
    config: Config,
    river_wm: Option<RiverWindowManagerV1>,
    river_xkb: Option<RiverXkbBindingsV1>,
    river_ls: Option<RiverLayerShellV1>,
    river_im: Option<RiverInputManagerV1>,
    river_lc: Option<RiverLibinputConfigV1>,
    wm: WindowManager,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Queue up a get_registry event.
    let conn = Connection::connect_to_env()?;
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let _registry = display.get_registry(&event_queue.handle(), ());

    let config = load_config();
    // Initial state
    // TODO: I can probably split off this config section and not use clone someday
    let mut app_data = AppData {
        config: config.clone(),
        ..Default::default()
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

    for program in &config.startup {
        spawn_shell(program)
    }

    loop {
        event_queue.blocking_dispatch(&mut app_data)?;
    }
}
