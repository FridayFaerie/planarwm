use crate::Output;
use crate::river::river_output_v1::RiverOutputV1;
use crate::river::river_shell_surface_v1::RiverShellSurfaceV1;

impl Output {
    pub fn new(proxy: RiverOutputV1) -> Self {
        Self {
            proxy,
            removed: false,
            layer: None,
            position: None,
            dimensions: None,
            background: None,
            overview_active: false,
        }
    }
}
