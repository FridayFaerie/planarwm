use wayland_client::QueueHandle;

use crate::AppData;
use crate::Window;
use crate::river::river_window_v1::{Edges, RiverWindowV1};

impl Window {
    pub fn new(proxy: RiverWindowV1, qh: &QueueHandle<AppData>) -> Self {
        let node = proxy.get_node(qh, ());
        Window {
            proxy,
            node,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            unmaximized_geometry: None,
            new: true,
            closed: false,
            hidden: Some(true),
            pointer_move_requested: None,
            pointer_resize_requested: None,
            pointer_resize_requested_edges: Edges::None,
            maximize_requested: None,
        }
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn set_node_position(&mut self, camera_x: i32, camera_y: i32) {
        self.node.set_position(self.x - camera_x, self.y - camera_y);
    }
}
