use std::sync::mpsc::Sender;

use wayland_backend::client::ObjectId;

use crate::Window;
use crate::wm::HashMap;
use crate::wm::RiverWindowV1;
use crate::wm::WindowLocation;
use crate::wm::slide::Slide;
use crate::wm::task::Task;
use crate::wm::workspace::Workspace;

#[derive(Debug)]
pub struct Desktop {
    pub workspaces: HashMap<String, Workspace>,
    pub active_workspace: String,

    queue_tx: Sender<Task>,
}

impl Desktop {
    pub fn new(queue_tx: Sender<Task>) -> Desktop {
        let mut workspaces = HashMap::new();

        let default_id = "default".to_string();

        workspaces.insert(
            default_id.clone(),
            Workspace::new(&default_id, queue_tx.clone()),
        );

        Self {
            workspaces,
            active_workspace: default_id,
            queue_tx,
        }
    }
    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        // TODO: here, I'm just hoping that workspaces have an active workspace :) if this doesn't
        // work, I might need to create a workspace if it doesn't exist
        self.workspaces.get_mut(&self.active_workspace).unwrap()
    }

    pub fn attach_window(&mut self, window_id: ObjectId, windows: &mut HashMap<ObjectId, Window>) {
        let queue_tx = self.queue_tx.clone();
        let ws = self.active_workspace_mut();
        if ws.slides.is_empty() {
            // TODO: make a "new_slide() function maybe?"
            ws.slides.push(Slide::new(0, ws.dimensions, queue_tx));
            ws.active_slide = 0;
            ws.child_rearrange_required = true;
            ws.rearrange();
        }

        let slide = &mut ws.slides[ws.active_slide];
        slide.attach_window(window_id.clone());

        if let Some(window) = windows.get_mut(&window_id) {
            window.location = Some(WindowLocation {
                workspace_id: ws.id.clone(),
                slide_id: slide.id,
            })
        }
    }
}

// impl Default for Desktop {
//     fn default() -> Desktop {
//         let mut workspaces = HashMap::new();
//
//         let default_id = "default".to_string();
//
//         workspaces.insert(
//             default_id.clone(),
//             Workspace::new(&default_id),
//         );
//
//         Self {
//             workspaces,
//             active_workspace: default_id,
//         }
//     }
// }
