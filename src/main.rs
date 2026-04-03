// SPDX-FileCopyrightText: © 2026 Julian Andrews
// SPDX-License-Identifier: 0BSD

use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::path::PathBuf;
use wayland_backend::client::ObjectId;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, protocol::wl_registry};
use xkbcommon::xkb::{self, KEYSYM_CASE_INSENSITIVE};

use crate::river::{
    river_layer_shell_output_v1::RiverLayerShellOutputV1,
    river_layer_shell_seat_v1::RiverLayerShellSeatV1,
    river_layer_shell_v1::RiverLayerShellV1,
    river_node_v1::RiverNodeV1,
    river_output_v1::RiverOutputV1,
    river_pointer_binding_v1::RiverPointerBindingV1,
    river_seat_v1::{Modifiers, RiverSeatV1},
    river_window_manager_v1::RiverWindowManagerV1,
    river_window_v1::{Edges, RiverWindowV1},
    river_xkb_binding_v1::RiverXkbBindingV1,
    river_xkb_bindings_v1::RiverXkbBindingsV1,
};

mod river {
    pub extern crate wayland_client;
    pub use wayland_client::protocol::*;

    mod interfaces {
        pub(super) mod rwm {
            pub use wayland_client::protocol::__interfaces::*;
            wayland_scanner::generate_interfaces!("./protocol/river-window-management-v1.xml");
        }

        pub(super) mod rxkb {
            use super::rwm::*;
            wayland_scanner::generate_interfaces!("./protocol/river-xkb-bindings-v1.xml");
        }

        pub(super) mod rls {
            use super::rwm::*;
            wayland_scanner::generate_interfaces!("./protocol/river-layer-shell-v1.xml");
        }
    }

    use self::interfaces::rls::*;
    use self::interfaces::rwm::*;
    use self::interfaces::rxkb::*;
    wayland_scanner::generate_client_code!("./protocol/river-window-management-v1.xml");
    wayland_scanner::generate_client_code!("./protocol/river-xkb-bindings-v1.xml");
    wayland_scanner::generate_client_code!("./protocol/river-layer-shell-v1.xml");
}

#[derive(Debug, Clone, Deserialize, Default)]
struct Config {
    #[serde(default)]
    bindings: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    startup: Vec<String>,
}

#[derive(Debug, Clone)]
enum Action {
    None,
    Pan,
    View { x: i32, y: i32 },
    Spawn { program: String, args: Vec<String> },
    SpawnShell { command: String },
    Close,
    Focus,
    FocusNext,
    Move,
    Resize,
    Fullscreen,
    Exit,
}

#[derive(Debug)]
enum LayerFocus {
    None,
    Exclusive,
    NonExclusive,
}

#[derive(Debug, Clone)]
enum SeatOp {
    None,
    Pan {
        start_x: i32,
        start_y: i32,
    },
    Move {
        window_proxy: RiverWindowV1,
        start_x: i32,
        start_y: i32,
    },
    Resize {
        window_proxy: RiverWindowV1,
        start_x: i32,
        start_y: i32,
        start_width: i32,
        start_height: i32,
        edges: Edges,
    },
}

#[derive(Debug, Default)]
struct AppData {
    config: Config,
    river_wm: Option<RiverWindowManagerV1>,
    river_xkb: Option<RiverXkbBindingsV1>,
    river_ls: Option<RiverLayerShellV1>,
    wm: WindowManager,
}

#[derive(Debug, Default)]
struct WindowManager {
    windows: VecDeque<Window>,
    outputs: HashMap<ObjectId, Output>,
    seats: HashMap<ObjectId, Seat>,
    camera_x: i32,
    camera_y: i32,
}

#[derive(Debug)]
struct Window {
    proxy: RiverWindowV1,
    node: RiverNodeV1,
    new: bool,
    closed: bool,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    hidden: Option<bool>,
    pointer_move_requested: Option<RiverSeatV1>,
    pointer_resize_requested: Option<RiverSeatV1>,
    pointer_resize_requested_edges: Edges,
}

#[derive(Debug)]
struct Output {
    proxy: RiverOutputV1,
    removed: bool,
    layer: Option<RiverLayerShellOutputV1>,
    position: Option<(i32, i32)>,
    dimensions: Option<(i32, i32)>,
    usable: Option<(i32, i32, i32, i32)>,
}

#[derive(Debug)]
struct Seat {
    proxy: RiverSeatV1,
    new: bool,
    removed: bool,
    focused: Option<RiverWindowV1>,
    hovered: Option<RiverWindowV1>,
    interacted: Option<RiverWindowV1>,
    xkb_bindings: HashMap<ObjectId, XkbBinding>,
    pointer_bindings: HashMap<ObjectId, PointerBinding>,
    pending_action: Action,
    op: SeatOp,
    op_dx: i32,
    op_dy: i32,
    op_release: bool,
    layer: Option<RiverLayerShellSeatV1>,
    layer_focus: LayerFocus,
}

#[derive(Debug)]
struct XkbBinding {
    proxy: RiverXkbBindingV1,
    action: Action,
}

#[derive(Debug)]
struct PointerBinding {
    proxy: RiverPointerBindingV1,
    action: Action,
}

impl WindowManager {
    fn handle_manage_start(
        &mut self,
        proxy: &RiverWindowManagerV1,
        river_xkb: &RiverXkbBindingsV1,
        qh: &QueueHandle<AppData>,
        config: &Config,
    ) {
        self.remove_outputs();
        self.remove_windows();
        self.remove_seats();
        self.init_new_windows();
        self.init_new_seats(river_xkb, qh, config);
        self.manage_windows();
        self.manage_seats(proxy);
        proxy.manage_finish();
    }

    fn handle_render_start(&mut self, proxy: &RiverWindowManagerV1) {
        for seat in &mut self.seats.values_mut() {
            match &seat.op {
                SeatOp::None => {}
                SeatOp::Pan { start_x, start_y } => {
                    self.camera_x = start_x - seat.op_dx;
                    self.camera_y = start_y - seat.op_dy;
                }
                SeatOp::Move {
                    window_proxy,
                    start_x,
                    start_y,
                } => {
                    if let Some(window) = self
                        .windows
                        .iter_mut()
                        .find(|window| &window.proxy == window_proxy)
                    {
                        window.set_position(start_x + seat.op_dx, start_y + seat.op_dy);
                    }
                }
                SeatOp::Resize {
                    window_proxy,
                    start_x,
                    start_y,
                    start_width,
                    start_height,
                    edges,
                } => {
                    if let Some(window) = self
                        .windows
                        .iter_mut()
                        .find(|window| &window.proxy == window_proxy)
                    {
                        let (mut x, mut y) = (*start_x, *start_y);
                        if edges.contains(Edges::Left) {
                            x += start_width - window.width;
                        }
                        if edges.contains(Edges::Top) {
                            y += start_height - window.height;
                        }
                        window.set_position(x, y);
                    }
                }
            }
        }

        let center = self
            .outputs
            .values()
            .find_map(|o| o.usable)
            .map(|(x, y, w, h)| (x + w / 2, y + h / 2));

        for window in self.windows.iter_mut() {
            if let Some(false) = window.hidden {
                if let Some((cx, cy)) = center {
                    window.set_position(
                        cx + self.camera_x - window.width / 2,
                        cy + self.camera_y - window.height / 2,
                    );
                }
                window.proxy.show();
                window.hidden = None;
                window.new = false
            }
            window.set_node_position(self.camera_x, self.camera_y);
        }

        proxy.render_finish();
    }

    fn remove_outputs(&mut self) {
        self.outputs.retain(|_, output| {
            if output.removed {
                if let Some(layer) = output.layer.take() {
                    layer.destroy();
                }
                output.proxy.destroy();
                return false;
            }
            true
        });
    }

    fn remove_windows(&mut self) {
        let old_windows = std::mem::take(&mut self.windows);
        self.windows = old_windows
            .into_iter()
            .filter(|window| {
                if window.closed {
                    for seat in self.seats.values_mut() {
                        if let SeatOp::Move { window_proxy, .. }
                        | SeatOp::Resize { window_proxy, .. } = &seat.op
                        {
                            if window_proxy == &window.proxy {
                                seat.op_end();
                            }
                        }
                    }
                    return false;
                }
                true
            })
            .collect();
    }

    fn remove_seats(&mut self) {
        self.seats.retain(|_, seat| {
            if seat.removed {
                if let Some(layer) = seat.layer.take() {
                    layer.destroy();
                }
                seat.xkb_bindings
                    .values_mut()
                    .for_each(|binding| binding.proxy.destroy());
                seat.pointer_bindings
                    .values_mut()
                    .for_each(|binding| binding.proxy.destroy());
                seat.proxy.destroy();
                return false;
            }
            true
        });
    }

    fn init_new_windows(&mut self) {
        for window in self.windows.iter_mut().filter(|w| w.new) {
            window.proxy.propose_dimensions(window.width, window.height);
            window.proxy.hide();
        }
    }

    fn init_new_seats(
        &mut self,
        river_xkb: &RiverXkbBindingsV1,
        qh: &QueueHandle<AppData>,
        config: &Config,
    ) {
        for seat in self.seats.values_mut().filter(|seat| seat.new) {
            for (mods_name, keymap) in &config.bindings {
                let Some(mods) = parse_modifiers(mods_name) else {
                    eprintln!("Unknown modifier group: {mods_name}");
                    continue;
                };

                for (key_name, action_text) in keymap {
                    let Some(keysym) = parse_keysym(key_name) else {
                        eprintln!("Unknown key: {key_name}");
                        continue;
                    };
                    let Some(action) = parse_action(action_text) else {
                        eprintln!("Unknown action: {action_text}");
                        continue;
                    };

                    seat.create_xkb_binding(river_xkb, qh, mods, keysym, action);
                }
            }

            const BTN_LEFT: u32 = 0x110;
            const BTN_RIGHT: u32 = 0x111;
            const BTN_MIDDLE: u32 = 0x112;
            let mods = Modifiers::Mod1;
            seat.create_pointer_binding(qh, mods, BTN_LEFT, Action::Move);
            seat.create_pointer_binding(qh, mods, BTN_RIGHT, Action::Resize);
            seat.create_pointer_binding(qh, Modifiers::None, BTN_MIDDLE, Action::Pan);

            seat.new = false;
        }
    }

    fn manage_windows(&mut self) {
        for window in self.windows.iter_mut() {
            if let Some(seat_proxy) = window.pointer_move_requested.take() {
                let seat = self
                    .seats
                    .get_mut(&seat_proxy.id())
                    .expect("Seat {seat_proxy.id()} not found");
                seat.pointer_move(window);
            }
            if let Some(seat_proxy) = window.pointer_resize_requested.take() {
                let seat = self
                    .seats
                    .get_mut(&seat_proxy.id())
                    .expect("Seat {seat_proxy.id()} not found");
                seat.pointer_resize(window, window.pointer_resize_requested_edges);
            }
        }
    }

    fn manage_seats(&mut self, wm_proxy: &RiverWindowManagerV1) {
        let windows = &mut self.windows;
        let camera_x = &mut self.camera_x;
        let camera_y = &mut self.camera_y;

        for seat in self.seats.values_mut() {
            if let Some(window_proxy) = seat.interacted.take() {
                let i = windows
                    .iter()
                    .position(|window| window.proxy == window_proxy)
                    .expect("Interacted window {window.proxy.id()} not found");
                let window = windows.remove(i).unwrap();
                windows.push_back(window);
            }
            seat.focus_top(windows);
            seat.do_action(windows, &self.outputs, wm_proxy, camera_x, camera_y);
            if seat.op_release {
                seat.op_end();
                seat.op_release = false;
            } else {
                seat.op_manage();
            }
        }
    }
}

impl Window {
    fn new(proxy: RiverWindowV1, qh: &QueueHandle<AppData>) -> Self {
        let node = proxy.get_node(qh, ());
        Window {
            proxy,
            node,
            new: true,
            closed: false,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            hidden: Some(true),
            pointer_move_requested: None,
            pointer_resize_requested: None,
            pointer_resize_requested_edges: Edges::None,
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    fn set_node_position(&mut self, camera_x: i32, camera_y: i32) {
        self.node.set_position(self.x - camera_x, self.y - camera_y);
    }
}

impl Output {
    fn new(proxy: RiverOutputV1) -> Self {
        Self {
            proxy,
            removed: false,
            layer: None,
            position: None,
            dimensions: None,
            usable: None,
        }
    }
}

impl Seat {
    fn new(proxy: RiverSeatV1) -> Self {
        Self {
            proxy,
            new: true,
            removed: false,
            focused: None,
            hovered: None,
            interacted: None,
            xkb_bindings: HashMap::new(),
            pointer_bindings: HashMap::new(),
            pending_action: Action::None,
            op: SeatOp::None,
            op_dx: 0,
            op_dy: 0,
            op_release: false,
            layer: None,
            layer_focus: LayerFocus::None,
        }
    }

    fn create_xkb_binding(
        &mut self,
        river_xkb: &RiverXkbBindingsV1,
        qh: &QueueHandle<AppData>,
        mods: Modifiers,
        keysym: u32,
        action: Action,
    ) {
        let proxy = river_xkb.get_xkb_binding(&self.proxy, keysym, mods, qh, self.proxy.id());
        proxy.enable();
        let binding = XkbBinding { proxy, action };
        self.xkb_bindings.insert(binding.proxy.id(), binding);
    }

    fn create_pointer_binding(
        &mut self,
        qh: &QueueHandle<AppData>,
        mods: Modifiers,
        button: u32,
        action: Action,
    ) {
        let proxy = self
            .proxy
            .get_pointer_binding(button, mods, qh, self.proxy.id());
        proxy.enable();
        let binding = PointerBinding { proxy, action };
        self.pointer_bindings.insert(binding.proxy.id(), binding);
    }

    fn do_action(
        &mut self,
        windows: &mut VecDeque<Window>,
        outputs: &HashMap<ObjectId, Output>,
        wm_proxy: &RiverWindowManagerV1,
        camera_x: &mut i32,
        camera_y: &mut i32,
    ) {
        match &self.pending_action {
            Action::None => {}
            Action::Pan => {
                self.pointer_pan(*camera_x, *camera_y);
            }
            Action::View { x, y } => {
                *camera_x = *x;
                *camera_y = *y;
            }
            Action::Spawn { program, args } => {
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                spawn_program(&program, &arg_refs)
            }
            Action::SpawnShell { command } => spawn_shell(&command),
            Action::Close => {
                if let Some(window_proxy) = self.focused.as_ref() {
                    window_proxy.close();
                }
            }
            Action::Focus => {
                if let Some(window_proxy) = self.focused.as_ref() {
                    let window = windows
                        .iter_mut()
                        .find(|window| &window.proxy == window_proxy)
                        .expect("Focused window {window.proxy.id()} not found");
                    self.focus_window_camera(window, outputs, camera_x, camera_y)
                }
            }
            Action::FocusNext => {
                windows.rotate_left(1);
                self.focus_top(windows);
            }
            Action::Move => {
                if let (Some(window_proxy), SeatOp::None) = (self.hovered.as_ref(), &self.op) {
                    let window = windows
                        .iter()
                        .find(|window| &window.proxy == window_proxy)
                        .expect("Hovered window {window.proxy.id()} not found");
                    self.pointer_move(window);
                }
            }
            Action::Resize => {
                if let (Some(window_proxy), SeatOp::None) = (self.hovered.as_ref(), &self.op) {
                    let window = windows
                        .iter()
                        .find(|window| &window.proxy == window_proxy)
                        .expect("Hovered window {window.proxy.id()} not found");
                    self.pointer_resize(window, Edges::Bottom.union(Edges::Right));
                }
            }
            Action::Fullscreen => {
                if let Some(window_proxy) = self.focused.as_ref() {
                    let window = windows
                        .iter_mut()
                        .find(|window| &window.proxy == window_proxy)
                        .expect("Focused window {window.proxy.id()} not found");
                    self.make_fullscreen(window, outputs, (*camera_x, *camera_y));
                }
            }
            Action::Exit => wm_proxy.exit_session(),
        }
        self.pending_action = Action::None;
    }

    fn op_end(&mut self) {
        if let SeatOp::Resize { window_proxy, .. } = &self.op {
            window_proxy.inform_resize_end();
        }
        self.proxy.op_end();
        self.op = SeatOp::None;
    }

    fn op_manage(&mut self) {
        match &self.op {
            SeatOp::None | SeatOp::Move { .. } => {}
            SeatOp::Pan { .. } => {}
            SeatOp::Resize {
                window_proxy,
                start_width,
                start_height,
                edges,
                ..
            } => {
                let (mut width, mut height) = (*start_width, *start_height);
                if edges.contains(Edges::Left) {
                    width -= self.op_dx;
                }
                if edges.contains(Edges::Right) {
                    width += self.op_dx;
                }
                if edges.contains(Edges::Top) {
                    height -= self.op_dy;
                }
                if edges.contains(Edges::Bottom) {
                    height += self.op_dy;
                }
                window_proxy.propose_dimensions(width, height);
            }
        }
    }

    fn focus_top(&mut self, windows: &VecDeque<Window>) {
        match self.layer_focus {
            LayerFocus::Exclusive => {
                self.proxy.clear_focus();
                self.focused = None;
                return;
            }
            LayerFocus::NonExclusive => {}
            LayerFocus::None => {}
        }
        match windows.back() {
            Some(window) => {
                self.proxy.focus_window(&window.proxy);
                window.node.place_top();
                self.focused = Some(window.proxy.clone());
            }
            None => {
                self.proxy.clear_focus();
                self.focused = None;
            }
        }
    }

    fn pointer_pan(&mut self, camera_x: i32, camera_y: i32) {
        self.proxy.op_start_pointer();
        self.op = SeatOp::Pan {
            start_x: camera_x,
            start_y: camera_y,
        };
        self.op_dx = 0;
        self.op_dy = 0;
    }

    fn pointer_move(&mut self, window: &Window) {
        self.interacted = Some(window.proxy.clone());
        self.proxy.op_start_pointer();
        self.op = SeatOp::Move {
            window_proxy: window.proxy.clone(),
            start_x: window.x,
            start_y: window.y,
        };
        self.op_dx = 0;
        self.op_dy = 0;
    }

    fn pointer_resize(&mut self, window: &Window, edges: Edges) {
        self.interacted = Some(window.proxy.clone());
        self.proxy.op_start_pointer();
        window.proxy.inform_resize_start();
        self.op = SeatOp::Resize {
            window_proxy: window.proxy.clone(),
            start_x: window.x,
            start_y: window.y,
            start_width: window.width,
            start_height: window.height,
            edges,
        };
        self.op_dx = 0;
        self.op_dy = 0;
    }

    fn make_fullscreen(
        &self,
        window: &mut Window,
        outputs: &HashMap<ObjectId, Output>,
        (camera_x, camera_y): (i32, i32),
    ) {
        let Some((x, y, width, height)) = outputs.values().find_map(|output| {
            let (width, height) = output.dimensions?;
            Some((camera_x, camera_y, width, height))
        }) else {
            return;
        };

        window.set_position(x, y);
        window.width = width;
        window.height = height;
        window.proxy.propose_dimensions(width, height);
    }
    fn focus_window_camera(
        &self,
        window: &Window,
        outputs: &HashMap<ObjectId, Output>,
        camera_x: &mut i32,
        camera_y: &mut i32,
    ) {
        let Some((screen_cx, screen_cy)) = outputs.values().find_map(|output| {
            let (x, y) = output.position?;
            let (w, h) = output.dimensions?;
            Some((x + w / 2, y + h / 2))
        }) else {
            return;
        };
        *camera_x = window.x + window.width / 2 - screen_cx;
        *camera_y = window.y + window.height / 2 - screen_cy
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            const RIVER_WINDOW_MANAGER_V1_VERSION: u32 = 4;
            const RIVER_XKB_BINDINGS_V1_VERSION: u32 = 1;
            const RIVER_LAYER_SHELL_V1_VERSION: u32 = 1;
            match interface.as_str() {
                "river_window_manager_v1" => {
                    if version < RIVER_WINDOW_MANAGER_V1_VERSION {
                        eprintln!(
                            "Server river_window_manager_v1 v{version}, but we need at least v{RIVER_WINDOW_MANAGER_V1_VERSION}",
                        );
                        std::process::exit(1);
                    }
                    let wm = registry.bind::<RiverWindowManagerV1, _, _>(
                        name,
                        RIVER_WINDOW_MANAGER_V1_VERSION,
                        qh,
                        (),
                    );
                    state.river_wm = Some(wm);
                }
                "river_xkb_bindings_v1" => {
                    if version < RIVER_XKB_BINDINGS_V1_VERSION {
                        eprintln!(
                            "Server supports river_xkb_bindings_v1 v{version}, but we need at least v{RIVER_XKB_BINDINGS_V1_VERSION}"
                        );
                        std::process::exit(1);
                    }
                    let xkb = registry.bind::<RiverXkbBindingsV1, _, _>(
                        name,
                        RIVER_XKB_BINDINGS_V1_VERSION,
                        qh,
                        (),
                    );
                    state.river_xkb = Some(xkb);
                }
                "river_layer_shell_v1" => {
                    if version < RIVER_LAYER_SHELL_V1_VERSION {
                        eprintln!(
                            "Server supports river_layer_shell_v1 v{version}, but we need at least v{RIVER_LAYER_SHELL_V1_VERSION}"
                        );
                        std::process::exit(1);
                    }
                    let layer_shell = registry.bind::<RiverLayerShellV1, _, _>(
                        name,
                        RIVER_LAYER_SHELL_V1_VERSION,
                        qh,
                        (),
                    );
                    state.river_ls = Some(layer_shell);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<RiverWindowManagerV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverWindowManagerV1,
        event: <RiverWindowManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use river::river_window_manager_v1::Event;
        match event {
            Event::Unavailable => {
                eprintln!("Error: Another WM is already running");
                std::process::exit(1);
            }
            Event::Finished => std::process::exit(0),
            Event::ManageStart => {
                let river_xkb = state
                    .river_xkb
                    .as_ref()
                    .expect("river_xkb_bindings_v1 missing");
                let config = state.config.clone();
                state.wm.handle_manage_start(proxy, river_xkb, qh, &config)
            }
            Event::RenderStart => state.wm.handle_render_start(proxy),
            Event::SessionLocked => {}
            Event::SessionUnlocked => {}
            Event::Window { id } => state.wm.windows.push_back(Window::new(id, qh)),
            Event::Output { id } => {
                let mut output = Output::new(id.clone());
                if let Some(layer_shell) = &state.river_ls {
                    output.layer =
                        Some(layer_shell.get_output(&output.proxy, qh, output.proxy.id()))
                }
                state.wm.outputs.insert(id.id(), output);
            }
            Event::Seat { id } => {
                let mut seat = Seat::new(id.clone());
                if let Some(layer_shell) = &state.river_ls {
                    seat.layer = Some(layer_shell.get_seat(&seat.proxy, qh, seat.proxy.id()))
                }
                state.wm.seats.insert(id.id(), seat);
            }
        }
    }

    wayland_client::event_created_child!(AppData, RiverWindowManagerV1, [
        river::river_window_manager_v1::EVT_WINDOW_OPCODE => (RiverWindowV1, ()),
        river::river_window_manager_v1::EVT_OUTPUT_OPCODE => (RiverOutputV1, ()),
        river::river_window_manager_v1::EVT_SEAT_OPCODE => (RiverSeatV1, ())
    ]);
}

impl Dispatch<RiverWindowV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverWindowV1,
        event: <RiverWindowV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_window_v1::Event;
        let window = match state
            .wm
            .windows
            .iter_mut()
            .find(|output| &output.proxy == proxy)
        {
            Some(window) => window,
            None => return,
        };
        match event {
            Event::Closed => window.closed = true,
            Event::DimensionsHint {
                min_width: _,
                min_height: _,
                max_width: _,
                max_height: _,
            } => {}
            Event::Dimensions { width, height } => {
                window.width = width;
                window.height = height;
                if window.new {
                    window.hidden = Some(false);
                }
            }
            Event::AppId { app_id: _ } => {}
            Event::Title { title: _ } => {}
            Event::Parent { parent: _ } => {}
            Event::DecorationHint { hint: _ } => {}
            Event::PointerMoveRequested { seat } => window.pointer_move_requested = Some(seat),
            Event::PointerResizeRequested { seat, edges } => {
                window.pointer_resize_requested = Some(seat);
                window.pointer_resize_requested_edges = edges
                    .into_result()
                    .expect("Invalid edges for resize: {edges}");
            }
            Event::ShowWindowMenuRequested { x: _, y: _ } => {}
            Event::MaximizeRequested => {}
            Event::UnmaximizeRequested => {}
            Event::FullscreenRequested { output: _ } => {}
            Event::ExitFullscreenRequested => {}
            Event::MinimizeRequested => {}
            Event::UnreliablePid { unreliable_pid: _ } => {}
            Event::PresentationHint { .. } => {}
            Event::Identifier { .. } => {}
        }
    }
}

impl Dispatch<RiverLayerShellV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &RiverLayerShellV1,
        _event: <RiverLayerShellV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<RiverOutputV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverOutputV1,
        event: <RiverOutputV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_output_v1::Event;
        let output = state
            .wm
            .outputs
            .get_mut(&proxy.id())
            .expect("Output {proxy.id()} not found");
        match event {
            Event::Removed => output.removed = true,
            Event::WlOutput { name: _ } => {}
            Event::Position { x, y } => output.position = Some((x, y)),
            Event::Dimensions { width, height } => {
                output.dimensions = Some((width, height));
            }
        }
    }
}

impl Dispatch<RiverLayerShellOutputV1, ObjectId> for AppData {
    fn event(
        state: &mut Self,
        _proxy: &RiverLayerShellOutputV1,
        event: <RiverLayerShellOutputV1 as Proxy>::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_layer_shell_output_v1::Event;
        let output = state
            .wm
            .outputs
            .get_mut(data)
            .expect("Output {proxy.id()} not found");
        match event {
            Event::NonExclusiveArea {
                x,
                y,
                width,
                height,
            } => output.usable = Some((x, y, width, height)),
        }
    }
}

impl Dispatch<RiverSeatV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverSeatV1,
        event: <RiverSeatV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_seat_v1::Event;
        let seat = state
            .wm
            .seats
            .get_mut(&proxy.id())
            .expect("Seat {proxy.id()} not found");
        match event {
            Event::Removed => seat.removed = true,
            Event::WlSeat { name: _ } => {}
            Event::PointerEnter { window } => seat.hovered = Some(window),
            Event::PointerLeave => seat.hovered = None,
            Event::WindowInteraction { window } => seat.interacted = Some(window),
            Event::ShellSurfaceInteraction {
                shell_surface: _shell_surface,
            } => {}
            Event::OpDelta { dx, dy } => (seat.op_dx, seat.op_dy) = (dx, dy),
            Event::OpRelease => seat.op_release = true,
            Event::PointerPosition { x: _, y: _ } => {}
        }
    }
}

impl Dispatch<RiverLayerShellSeatV1, ObjectId> for AppData {
    fn event(
        state: &mut Self,
        _proxy: &RiverLayerShellSeatV1,
        event: <RiverLayerShellSeatV1 as Proxy>::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_layer_shell_seat_v1::Event;
        let seat = state
            .wm
            .seats
            .get_mut(data)
            .expect("Seat {proxy.id()} not found");
        match event {
            Event::FocusExclusive => seat.layer_focus = LayerFocus::Exclusive,
            Event::FocusNonExclusive => seat.layer_focus = LayerFocus::NonExclusive,
            Event::FocusNone => seat.layer_focus = LayerFocus::None,
        }
    }
}

impl Dispatch<RiverXkbBindingV1, ObjectId> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverXkbBindingV1,
        event: <RiverXkbBindingV1 as Proxy>::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_xkb_binding_v1::Event;
        let seat = state.wm.seats.get_mut(data).expect("Seat {data} not found");
        let binding = seat
            .xkb_bindings
            .get(&proxy.id())
            .expect("xkb_binding {proxy.id()} not found");
        match event {
            Event::Pressed => seat.pending_action = binding.action.clone(),
            Event::Released => {}
            Event::StopRepeat => {}
        }
    }
}

impl Dispatch<RiverPointerBindingV1, ObjectId> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverPointerBindingV1,
        event: <RiverPointerBindingV1 as Proxy>::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_pointer_binding_v1::Event;
        let seat = state.wm.seats.get_mut(data).expect("Seat {data} not found");
        let binding = seat
            .pointer_bindings
            .get(&proxy.id())
            .expect("xkb_binding {proxy.id()} not found");
        match event {
            Event::Pressed => seat.pending_action = binding.action.clone(),
            Event::Released => {}
        }
    }
}

wayland_client::delegate_noop!(AppData: ignore RiverXkbBindingsV1);
wayland_client::delegate_noop!(AppData: ignore RiverNodeV1);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Queue up a get_registry event.
    let conn = Connection::connect_to_env()?;
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let _registry = display.get_registry(&event_queue.handle(), ());

    let config = load_config();
    // Initial state
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

fn spawn_program(program: &str, args: &[&str]) {
    // match
    std::process::Command::new(program)
        .args(args)
        // Don't pass WAYLAND_DEBUG on to children, the added noise makes
        // debugging the window manager itself impractical.
        .env_remove("WAYLAND_DEBUG")
        // .stdin(std::process::Stdio::null())
        // .stdout(std::process::Stdio::inherit())
        // .stderr(std::process::Stdio::inherit())
        .spawn()
        .expect("couldn't spawn program");
    // {
    //     Ok(_) => {}
    //     Err(e) => eprintln!("Failed to spawn {program}: {e}"),
    // }
}

fn spawn_shell(command: &str) {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .env_remove("WAYLAND_DEBUG")
        .spawn()
        .expect("error in running shell command");
}

fn config_path() -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_else(|| ".".into());
    PathBuf::from(home)
        .join(".config")
        .join("river")
        .join("planarwm.hocon")
}

fn load_config() -> Config {
    let path = config_path();

    if !path.exists() {
        return Config::default();
    }

    match hocon::HoconLoader::new().load_file(path.to_string_lossy().as_ref()) {
        Ok(loader) => match loader.resolve() {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Failed to parse config: {e}");
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("Failed to read config: {e}");
            Config::default()
        }
    }
}

fn parse_modifiers(s: &str) -> Option<Modifiers> {
    let mut mods = Modifiers::None;
    let mut seen_any = false;

    for part in s.split(|c: char| c == '+' || c == '-') {
        let part = part.trim().to_ascii_lowercase();
        if part.is_empty() {
            continue;
        }

        let m = match part.as_str() {
            "none" => Modifiers::None,
            "shift" => Modifiers::Shift,
            "ctrl" => Modifiers::Ctrl,
            "alt" => Modifiers::Mod1,
            "super" => Modifiers::Mod4,
            "mod3" => Modifiers::Mod3,
            "mod5" => Modifiers::Mod5,
            _ => return None,
        };

        mods = mods.union(m);
        seen_any = true;
    }

    if seen_any { Some(mods) } else { None }
}

fn parse_keysym(s: &str) -> Option<u32> {
    let ks = xkb::keysym_from_name(s.trim(), KEYSYM_CASE_INSENSITIVE);
    // it is recommended to first call this function without this flag; and if that fails, only then to try with this flag, while possibly warning the user he had misspelled the name, and might get wrong results.
    // but I shall not :>
    if ks != xkb::keysyms::KEY_NoSymbol.into() {
        return Some(ks.into());
    }
    None
}

fn parse_action(keyword: &str) -> Option<Action> {
    let keyword = keyword.trim();

    match keyword {
        "pan" => Some(Action::Pan),
        "close" => Some(Action::Close),
        "focus" => Some(Action::Focus),
        "focus_next" => Some(Action::FocusNext),
        "move" => Some(Action::Move),
        "resize" => Some(Action::Resize),
        "fullscreen" => Some(Action::Fullscreen),
        "exit" => Some(Action::Exit),
        _ if keyword.starts_with("spawn ") => {
            let rest = &keyword["spawn ".len()..];
            let mut parts = rest.split_whitespace();
            let program = parts.next()?.to_string();
            let args = parts.map(|s| s.to_string()).collect();
            Some(Action::Spawn { program, args })
        }
        _ if keyword.starts_with("shell ") => {
            let command = keyword["shell ".len()..].trim().to_string();
            Some(Action::SpawnShell { command })
        }
        _ if keyword.starts_with("view ") => {
            let mut parts = keyword["view ".len()..].split_whitespace();
            let x = parts.next()?.parse().ok()?;
            let y = parts.next()?.parse().ok()?;
            Some(Action::View { x, y })
        }
        _ => None,
    }
}
