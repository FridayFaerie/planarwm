use crate::Window;
use crate::wm::HashMap;
use crate::wm::RiverWindowV1;
use crate::wm::WindowLocation;
use crate::wm::slide::{Slide, SlideType};
use crate::wm::workspace::Workspace;

#[derive(Debug)]
pub struct Desktop {
    pub workspaces: HashMap<String, Workspace>,
    pub active_workspace: String,
}

impl Desktop {
    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        // TODO: here, I'm just hoping that workspaces have an active workspace :) if this doesn't
        // work, I might need to create a workspace if it doesn't exist
        self.workspaces.get_mut(&self.active_workspace).unwrap()
    }

    pub fn attach_window(
        &mut self,
        window_id: RiverWindowV1,
        windows: &mut HashMap<RiverWindowV1, Window>,
    ) {
        let ws = self.active_workspace_mut();
        if ws.slides.is_empty() {
            ws.slides.push(Slide::new(0, ws.dimensions));
            ws.active_slide = 0;
        }
        ws.child_rearrange_required = true;
        ws.rearrange_required = true;

        let slide = &mut ws.slides[ws.active_slide];
        slide.attach_window(window_id.clone());

        if let Some(window) = windows.get_mut(&window_id) {
            window.location = Some(WindowLocation {
                workspace_id: ws.id.clone(),
                slide_id: slide.id.clone(),
            })
        }
    }
}

impl Default for Desktop {
    fn default() -> Desktop {
        let mut workspaces = HashMap::new();

        let default_id = "default".to_string();

        workspaces.insert(default_id.clone(), Workspace::new(&default_id));

        Self {
            workspaces,
            active_workspace: default_id,
        }
    }
}
