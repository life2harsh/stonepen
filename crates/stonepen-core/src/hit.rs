use crate::geom::{bbox_contains_point, polyline_hit};
use crate::point::Point2;
use crate::stroke::InkStroke;
use crate::xform::Xform2D;

pub fn stroke_hit(stroke: &InkStroke, pos: Point2, radius: f32) -> bool {
    stroke_hit_with_xform(stroke, stroke.xform, pos, radius)
}

pub fn stroke_hit_with_xform(
    stroke: &InkStroke,
    eff_xf: Xform2D,
    pos: Point2,
    radius: f32,
) -> bool {
    let world_bbox = stroke.world_bbox;
    let padded_min_x = world_bbox.min_x - radius;
    let padded_min_y = world_bbox.min_y - radius;
    let padded_max_x = world_bbox.max_x + radius;
    let padded_max_y = world_bbox.max_y + radius;
    if pos.x < padded_min_x || pos.x > padded_max_x || pos.y < padded_min_y || pos.y > padded_max_y
    {
        return false;
    }
    if let Some(inv_xform) = eff_xf.inverse() {
        let local_pos = inv_xform.apply(pos);
        let scale = crate::xform::xform_scale(eff_xf).max(0.001);
        let local_radius = radius / scale;
        polyline_hit(&stroke.pts, local_pos, local_radius)
    } else {
        false
    }
}

pub fn stroke_bbox_contains(stroke: &InkStroke, pos: Point2) -> bool {
    bbox_contains_point(stroke.world_bbox, pos)
}
