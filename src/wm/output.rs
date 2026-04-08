use crate::Output;
use crate::river::river_output_v1::RiverOutputV1;
use crate::wm::utils::Rect;

impl Output {
    pub fn new(proxy: RiverOutputV1) -> Self {
        Self {
            proxy,
            removed: false,
            layer: None,
            position: None,
            dimensions: None,
            usable: None,
        }
    }

    // TODO: just set usable to position & dimensions by default? then we can read directly from usable
    pub fn bounds(&self) -> Option<Rect> {
        if let Some((x, y, width, height)) = self.usable {
            Some(Rect {
                x,
                y,
                width,
                height,
            })
        } else {
            let (x, y) = self.position.unwrap();
            let (width, height) = self.dimensions.unwrap();
            Some(Rect {
                x,
                y,
                width,
                height,
            })
        }
    }
}
