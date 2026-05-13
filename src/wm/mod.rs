pub mod desktop;
pub mod dispatch;
pub mod libinputdevice;
pub mod lifecycle;
pub mod output;
pub mod seat;
pub mod slide;
pub mod task;
pub mod utils;
pub mod window;
pub mod workspace;

use crate::wm::desktop::Desktop;
use crate::wm::task::Task;
use crate::wm::utils::Position;
use crate::wm::window::WindowLocation;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use wayland_backend::client::ObjectId;

use crate::actions::Action;
use crate::river::{
    river_layer_shell_output_v1::RiverLayerShellOutputV1,
    river_layer_shell_seat_v1::RiverLayerShellSeatV1,
    river_libinput_device_v1::RiverLibinputDeviceV1,
    river_node_v1::RiverNodeV1,
    river_output_v1::RiverOutputV1,
    river_pointer_binding_v1::RiverPointerBindingV1,
    river_seat_v1::RiverSeatV1,
    river_window_v1::{Edges, RiverWindowV1},
    river_xkb_binding_v1::RiverXkbBindingV1,
};

#[derive(Debug)]
pub enum LayerFocus {
    None,
    Exclusive,
    NonExclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeatOp {
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

#[derive(Debug)]
pub struct WindowManager {
    pub desktop: Desktop,
    pub windows: HashMap<RiverWindowV1, Window>,
    pub outputs: HashMap<ObjectId, Output>,
    pub seats: HashMap<ObjectId, Seat>,
    pub libinput_devices: HashMap<ObjectId, LibinputDevice>,
    queue_rx: Receiver<Task>,
    pub queue_tx: Sender<Task>,

    // TODO: combine camera_x/y into one camera_pos tuple
    // Also, this should very much come with dimensions (I think)
    // maybe move into Output?
    pub camera_x: i32,
    pub camera_y: i32,
}

#[derive(Debug)]
pub struct Window {
    pub proxy: RiverWindowV1,
    pub node: RiverNodeV1,
    pub title: String,

    pub location: Option<WindowLocation>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    pub original_position: Position,
    pub render_position: Option<Position>,
    pub current_position: Position,

    pub new: bool,
    pub closed: bool,
    pub maximized: bool,

    pub pointer_move_requested: Option<RiverSeatV1>,
    pub pointer_resize_requested: Option<RiverSeatV1>,
    pub pointer_resize_requested_edges: Edges,
}

#[derive(Debug)]
pub struct LibinputDevice {
    proxy: RiverLibinputDeviceV1,
    tap_support: Option<i32>,
}

#[derive(Debug)]
pub struct Output {
    pub proxy: RiverOutputV1,
    pub removed: bool,
    pub layer: Option<RiverLayerShellOutputV1>,
    // node
    pub position: Option<(i32, i32)>,
    pub dimensions: Option<(i32, i32)>,
    // logical
}

#[derive(Debug)]
pub struct Seat {
    pub proxy: RiverSeatV1,
    queue_tx: Sender<Task>,
    pub new: bool,
    pub removed: bool,
    pub focused: Option<RiverWindowV1>,
    pub hovered: Option<RiverWindowV1>,
    pub interacted: Option<RiverWindowV1>,
    pub xkb_bindings: HashMap<ObjectId, XkbBinding>,
    pub pointer_bindings: HashMap<ObjectId, PointerBinding>,
    pub pending_action: Action,
    pub op: SeatOp,
    pub op_dx: i32,
    pub op_dy: i32,
    pub op_release: bool,
    pub layer: Option<RiverLayerShellSeatV1>,
    pub layer_focus: LayerFocus,
}

#[derive(Debug)]
pub struct XkbBinding {
    pub proxy: RiverXkbBindingV1,
    pub action: Action,
}

#[derive(Debug)]
pub struct PointerBinding {
    pub proxy: RiverPointerBindingV1,
    pub action: Action,
}
