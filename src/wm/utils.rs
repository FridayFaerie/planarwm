use std::ops::Sub;

// TODO: maybe change Rect into {Position,Dimensions}
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Dimension {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

// TODO: this is actually a RectDelta, but I'm outputting a Rect.... they're about the same amirite
impl Sub for Rect {
    type Output = Rect;

    fn sub(self, rhs: Rect) -> Rect {
        Rect {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}

impl Sub for Dimension {
    type Output = Dimension;

    fn sub(self, rhs: Dimension) -> Dimension {
        Dimension {
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}
