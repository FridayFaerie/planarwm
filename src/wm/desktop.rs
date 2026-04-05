use crate::wm::workspace::Workspace;

#[derive(Debug, Default)]
pub struct Desktop {
    pub workspaces: Vec<Workspace>,
    pub active_workspace: usize,
}
