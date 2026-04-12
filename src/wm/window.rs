use crate::AppData;
use crate::Window;
use crate::river::river_window_v1::{Edges, RiverWindowV1};
use crate::wm::utils::Rect;
use wayland_client::QueueHandle;

impl Window {
    pub fn new(proxy: RiverWindowV1, qh: &QueueHandle<AppData>) -> Self {
        let node = proxy.get_node(qh, ());
        Window {
            proxy,
            node,
            title: "unknown".to_string(),
            location: None,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            target_dimensions: Some((0, 0)),
            target_position: None,
            unmaximized_geometry: None,
            new: true,
            closed: false,
            pointer_move_requested: None,
            pointer_resize_requested: None,
            pointer_resize_requested_edges: Edges::None,
            relayout_requested: true,
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

    pub fn set_target_geometry(&mut self, rect: Rect) {
        self.target_position = Some((rect.x, rect.y));
        self.target_dimensions = Some((rect.width, rect.height));
    }
}

#[derive(Debug, Clone)]
pub struct WindowLocation {
    pub workspace_id: String,
    pub slide_id: u16,
}
