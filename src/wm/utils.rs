use std::ops::{Add, AddAssign, Mul, Sub};

// TODO: maybe change Rect into {Position,Dimensions}
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

// TODO: maybe rename dimension to size
#[derive(Debug, Clone, Copy)]
pub struct Dimension {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl Sub for Position {
    type Output = Position;
    fn sub(self, rhs: Position) -> Position {
        Position {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Add for Position {
    type Output = Position;
    fn add(self, rhs: Position) -> Position {
        Position {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

// TODO: remove?
impl AddAssign<&mut Position> for Position {
    fn add_assign(&mut self, rhs: &mut Position) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl AddAssign<Position> for Position {
    fn add_assign(&mut self, rhs: Position) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Mul<f32> for Position {
    type Output = Position;
    fn mul(self, rhs: f32) -> Position {
        Position {
            x: (self.x as f32 * rhs) as i32,
            y: (self.y as f32 * rhs) as i32,
        }
    }
}
