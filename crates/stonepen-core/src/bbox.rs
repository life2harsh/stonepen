use serde::{Deserialize, Serialize};

use crate::point::Point2;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl BBox {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn from_point(p: Point2) -> Self {
        Self {
            min_x: p.x,
            min_y: p.y,
            max_x: p.x,
            max_y: p.y,
        }
    }

    pub fn expand_by(self, r: f32) -> Self {
        Self {
            min_x: self.min_x - r,
            min_y: self.min_y - r,
            max_x: self.max_x + r,
            max_y: self.max_y + r,
        }
    }

    pub fn union(self, other: BBox) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    pub fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(self) -> f32 {
        self.max_y - self.min_y
    }

    pub fn center(self) -> Point2 {
        Point2 {
            x: (self.min_x + self.max_x) * 0.5,
            y: (self.min_y + self.max_y) * 0.5,
        }
    }

    pub fn to_aabb(self) -> rstar::AABB<[f32; 2]> {
        rstar::AABB::from_corners([self.min_x, self.min_y], [self.max_x, self.max_y])
    }
}
