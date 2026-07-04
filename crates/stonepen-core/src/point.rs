use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    pub x: f32,
    pub y: f32,
}

impl Point2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Point2) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointerKind {
    Pen,
    Touch,
    Mouse,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InkPoint {
    pub x: f32,
    pub y: f32,
    pub t_ms: f64,
    pub press: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub twist: f32,
    pub pointer_type: PointerKind,
}

impl InkPoint {
    pub fn pos(&self) -> Point2 {
        Point2 {
            x: self.x,
            y: self.y,
        }
    }
}

impl std::ops::Sub<Point2> for Point2 {
    type Output = Vec2;
    fn sub(self, other: Point2) -> Vec2 {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Add<Vec2> for Point2 {
    type Output = Point2;
    fn add(self, other: Vec2) -> Point2 {
        Point2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Vec2 {
    pub fn len(self) -> f32 {
        self.length()
    }
}
