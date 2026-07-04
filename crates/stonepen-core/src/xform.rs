use serde::{Deserialize, Serialize};

use crate::bbox::BBox;
use crate::point::Point2;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Xform2D {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Xform2D {
    pub fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx: 0.0,
            ty: 0.0,
        }
    }

    pub fn translate(dx: f32, dy: f32) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx: dx,
            ty: dy,
        }
    }

    pub fn rotate(angle_rad: f32) -> Self {
        let cos = angle_rad.cos();
        let sin = angle_rad.sin();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            tx: 0.0,
            ty: 0.0,
        }
    }

    pub fn scale(sx: f32, sy: f32) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            tx: 0.0,
            ty: 0.0,
        }
    }

    pub fn scale_about(pivot: Point2, sx: f32, sy: f32) -> Self {
        Self::translate(pivot.x, pivot.y)
            .concat(Self::scale(sx, sy))
            .concat(Self::translate(-pivot.x, -pivot.y))
    }

    pub fn rotate_about(pivot: Point2, angle_rad: f32) -> Self {
        Self::translate(pivot.x, pivot.y)
            .concat(Self::rotate(angle_rad))
            .concat(Self::translate(-pivot.x, -pivot.y))
    }

    pub fn apply(self, p: Point2) -> Point2 {
        Point2 {
            x: self.a * p.x + self.c * p.y + self.tx,
            y: self.b * p.x + self.d * p.y + self.ty,
        }
    }

    pub fn concat(self, other: Xform2D) -> Xform2D {
        Xform2D {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            tx: self.a * other.tx + self.c * other.ty + self.tx,
            ty: self.b * other.tx + self.d * other.ty + self.ty,
        }
    }

    pub fn apply_bbox(self, bbox: BBox) -> BBox {
        let corners = [
            Point2::new(bbox.min_x, bbox.min_y),
            Point2::new(bbox.max_x, bbox.min_y),
            Point2::new(bbox.max_x, bbox.max_y),
            Point2::new(bbox.min_x, bbox.max_y),
        ];
        let pts: Vec<Point2> = corners.iter().map(|&p| self.apply(p)).collect();
        let min_x = pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let min_y = pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_x = pts.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let max_y = pts.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        BBox {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn inverse(self) -> Option<Self> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-6 {
            return None;
        }
        let inv_det = 1.0 / det;
        let a = self.d * inv_det;
        let b = -self.b * inv_det;
        let c = -self.c * inv_det;
        let d = self.a * inv_det;
        let tx = -(a * self.tx + c * self.ty);
        let ty = -(b * self.tx + d * self.ty);
        Some(Self { a, b, c, d, tx, ty })
    }
}

impl Default for Xform2D {
    fn default() -> Self {
        Self::identity()
    }
}

pub fn xform_scale(xf: Xform2D) -> f32 {
    (xf.a * xf.d - xf.b * xf.c).abs().sqrt()
}
