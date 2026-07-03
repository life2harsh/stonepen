use crate::bbox::BBox;
use crate::point::{InkPoint, Point2};

pub fn compute_bbox(pts: &[InkPoint], extra_radius: f32) -> Option<BBox> {
    if pts.is_empty() {
        return None;
    }
    let mut min_x = pts[0].x;
    let mut min_y = pts[0].y;
    let mut max_x = pts[0].x;
    let mut max_y = pts[0].y;
    for p in pts.iter().skip(1) {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }
    Some(BBox {
        min_x: min_x - extra_radius,
        min_y: min_y - extra_radius,
        max_x: max_x + extra_radius,
        max_y: max_y + extra_radius,
    })
}

pub fn bbox_intersects(a: BBox, b: BBox) -> bool {
    a.min_x <= b.max_x && a.max_x >= b.min_x && a.min_y <= b.max_y && a.max_y >= b.min_y
}

pub fn bbox_contains_point(bbox: BBox, pos: Point2) -> bool {
    pos.x >= bbox.min_x && pos.x <= bbox.max_x && pos.y >= bbox.min_y && pos.y <= bbox.max_y
}

pub fn distance_to_segment(pos: Point2, a: Point2, b: Point2) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-10 {
        return pos.distance_to(a);
    }
    let t = ((pos.x - a.x) * dx + (pos.y - a.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj = Point2::new(a.x + t * dx, a.y + t * dy);
    pos.distance_to(proj)
}

pub fn polyline_hit(pts: &[InkPoint], pos: Point2, radius: f32) -> bool {
    if pts.is_empty() {
        return false;
    }
    if pts.len() == 1 {
        return pts[0].pos().distance_to(pos) <= radius;
    }
    for w in pts.windows(2) {
        if distance_to_segment(pos, w[0].pos(), w[1].pos()) <= radius {
            return true;
        }
    }
    false
}

pub fn point_in_polygon(pos: Point2, polygon: &[Point2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let pi = polygon[i];
        let pj = polygon[j];
        if ((pi.y > pos.y) != (pj.y > pos.y))
            && (pos.x < (pj.x - pi.x) * (pos.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn polyline_intersects_polygon(pts: &[InkPoint], polygon: &[Point2]) -> bool {
    if pts.is_empty() || polygon.len() < 3 {
        return false;
    }
    for p in pts {
        if point_in_polygon(p.pos(), polygon) {
            return true;
        }
    }
    let n = polygon.len();
    for w in pts.windows(2) {
        let a = w[0].pos();
        let b = w[1].pos();
        for i in 0..n {
            let pa = polygon[i];
            let pb = polygon[(i + 1) % n];
            if segments_intersect(a, b, pa, pb) {
                return true;
            }
        }
    }
    false
}

fn cross2d(o: Point2, a: Point2, b: Point2) -> f32 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn segments_intersect(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let d1 = cross2d(c, d, a);
    let d2 = cross2d(c, d, b);
    let d3 = cross2d(a, b, c);
    let d4 = cross2d(a, b, d);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    false
}

pub fn xform_point(xform: crate::xform::Xform2D, pos: Point2) -> Point2 {
    xform.apply(pos)
}

pub fn xform_bbox(xform: crate::xform::Xform2D, bbox: BBox) -> BBox {
    xform.apply_bbox(bbox)
}
