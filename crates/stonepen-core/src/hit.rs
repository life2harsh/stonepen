use crate::geom::{bbox_contains_point, polyline_hit};
use crate::point::Point2;
use crate::stroke::InkStroke;
use crate::xform::Xform2D;

pub fn stroke_hit(stroke: &InkStroke, pos: Point2, radius: f32) -> bool {
    let world_bbox = stroke.world_bbox;
    let padded_min_x = world_bbox.min_x - radius;
    let padded_min_y = world_bbox.min_y - radius;
    let padded_max_x = world_bbox.max_x + radius;
    let padded_max_y = world_bbox.max_y + radius;
    if pos.x < padded_min_x || pos.x > padded_max_x || pos.y < padded_min_y || pos.y > padded_max_y
    {
        return false;
    }
    let inv_xform = invert_xform(stroke.xform);
    let local_pos = inv_xform.apply(pos);
    polyline_hit(&stroke.pts, local_pos, radius)
}

fn invert_xform(xf: Xform2D) -> Xform2D {
    let det = xf.a * xf.d - xf.b * xf.c;
    if det.abs() < 1e-10 {
        return Xform2D::identity();
    }
    let inv_det = 1.0 / det;
    Xform2D {
        a: xf.d * inv_det,
        b: -xf.b * inv_det,
        c: -xf.c * inv_det,
        d: xf.a * inv_det,
        tx: (xf.c * xf.ty - xf.d * xf.tx) * inv_det,
        ty: (xf.b * xf.tx - xf.a * xf.ty) * inv_det,
    }
}

pub fn stroke_bbox_contains(stroke: &InkStroke, pos: Point2) -> bool {
    bbox_contains_point(stroke.world_bbox, pos)
}
