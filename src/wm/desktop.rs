use crate::wm::HashMap;
use crate::wm::slide::{Slide, SlideType};
use crate::wm::workspace::Workspace;

#[derive(Debug)]
pub struct Desktop {
    pub workspaces: HashMap<String, Workspace>,
    pub active_workspace: String,
}

impl Desktop {
    // pub fn new() -> Self {
    //     let mut workspaces = HashMap::new();
    //
    //     let default_id = "default".to_string();
    //
    //     // TODO: change to floating
    //     let default_ws = Workspace {
    //         id: default_id,
    //         coord: (0, 0),
    //         slides: vec![Slide::new(SlideType::Master)],
    //         active_slide: 0,
    //     };
    //
    //     workspaces.insert(default_id, default_ws);
    //
    //     Self {
    //         workspaces,
    //         active_workspace: default_id,
    //     }
    // }

    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        // TODO: here, I'm just hoping that workspaces have an active workspace :) if this doesn't
        // work, I might need to create a workspace if it doesn't exist
        self.workspaces.get_mut(&self.active_workspace).unwrap()
    }
}

impl Default for Desktop {
    fn default() -> Desktop {
        let mut workspaces = HashMap::new();

        let default_id = "default".to_string();

        // TODO: change to floating
        let default_ws = Workspace {
            id: default_id.clone(),
            coord: (0, 0),
            // TODO: fix
            dimensions: (1920, 1080),
            // dimensions: (1280, 720),
            slides: vec![Slide::new(0)],
            active_slide: 0,
            child_rearrange_required: true,
            rearrange_required: true,
            focus_active_requested: false,
            new_slide_id: 1,
        };

        workspaces.insert(default_id.clone(), default_ws);

        Self {
            workspaces,
            active_workspace: default_id,
        }
    }
}
