use crate::wm::workspace::Workspace;
pub struct Desktop {
    pub workspaces: Vec<Workspace>,
    pub active_workspace: usize,
}
