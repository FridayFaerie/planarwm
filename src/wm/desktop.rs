use crate::wm::HashMap;
use crate::wm::workspace::Workspace;

#[derive(Debug, Default)]
pub struct Desktop {
    pub workspaces: HashMap<String, Workspace>,
    pub active_workspace: String,
}
