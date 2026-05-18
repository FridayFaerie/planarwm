use crate::AppData;
use crate::Window;
use crate::river::river_window_v1::{Edges, RiverWindowV1};
use crate::wm::utils::Position;
use wayland_client::QueueHandle;

impl Window {
    pub fn new(proxy: RiverWindowV1, qh: &QueueHandle<AppData>) -> Self {
        let node = proxy.get_node(qh, ());
        Window {
            proxy,
            node,
            title: "unknown".to_string(),
            app_id: "unknown".to_string(),

            location: None,
            // TODO: remove the default x,y,w,h
            x: 0,
            y: 0,
            width: 0,
            height: 0,

            // TODO: maybe not default to 0?
            original_position: Position { x: 0, y: 0 },
            render_position: Position { x: 0, y: 0 },
            rendered_position: Position { x: 0, y: 0 },
            target_position: Position { x: 0, y: 0 },

            new: true,
            maximized: false,
            closed: false,

            pointer_move_requested: None,
            pointer_resize_requested: None,
            pointer_resize_requested_edges: Edges::None,
        }
    }

    // TODO: remove?
    pub fn set_node_position(&mut self, camera_pos: Position) {
        self.node.set_position(
            self.render_position.x - camera_pos.x,
            self.render_position.y - camera_pos.y,
        );
    }
}

#[derive(Debug, Clone)]
pub struct WindowLocation {
    pub workspace_id: String,
    pub slide_id: u16,
}
