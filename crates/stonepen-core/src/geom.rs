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

pub fn generate_stroke_outline(
    pts: &[InkPoint],
    brush: &crate::brush::Brush,
) -> Option<Vec<Point2>> {
    if pts.is_empty() {
        return None;
    }
    if pts.len() == 1 {
        let center = pts[0].pos();
        let r = crate::brush::stroke_w(brush, pts[0].press) * 0.5;
        let mut out = Vec::new();
        const SEGMENTS: usize = 16;
        for i in 0..SEGMENTS {
            let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
            out.push(Point2::new(
                center.x + angle.cos() * r,
                center.y + angle.sin() * r,
            ));
        }
        return Some(out);
    }
    let n = pts.len();
    let mut dists = vec![0.0; n];
    let mut total_len = 0.0;
    for i in 1..n {
        let dx = pts[i].x - pts[i - 1].x;
        let dy = pts[i].y - pts[i - 1].y;
        total_len += (dx * dx + dy * dy).sqrt();
        dists[i] = total_len;
    }
    let mut tangents = vec![Point2::new(0.0, 0.0); n];
    for i in 0..n {
        let t_vec = if i == 0 {
            Point2::new(pts[1].x - pts[0].x, pts[1].y - pts[0].y)
        } else if i == n - 1 {
            Point2::new(pts[n - 1].x - pts[n - 2].x, pts[n - 1].y - pts[n - 2].y)
        } else {
            Point2::new(pts[i + 1].x - pts[i - 1].x, pts[i + 1].y - pts[i - 1].y)
        };
        let len = (t_vec.x * t_vec.x + t_vec.y * t_vec.y).sqrt();
        if len > 1e-5 {
            tangents[i] = Point2::new(t_vec.x / len, t_vec.y / len);
        } else if i > 0 {
            tangents[i] = tangents[i - 1];
        } else {
            tangents[i] = Point2::new(1.0, 0.0);
        }
    }
    let mut left_pts = Vec::with_capacity(n);
    let mut right_pts = Vec::with_capacity(n);
    for i in 0..n {
        let pt = pts[i];
        let t_vec = tangents[i];
        let normal = Point2::new(-t_vec.y, t_vec.x);
        let mut factor = 1.0f32;
        if total_len > 0.001 {
            if brush.taper_start > 0.0 {
                let taper_start_len = brush.taper_start * total_len;
                if dists[i] < taper_start_len {
                    factor = factor.min(dists[i] / taper_start_len);
                }
            }
            if brush.taper_end > 0.0 {
                let taper_end_len = brush.taper_end * total_len;
                let dist_from_end = total_len - dists[i];
                if dist_from_end < taper_end_len {
                    factor = factor.min(dist_from_end / taper_end_len);
                }
            }
        }
        let half_w = crate::brush::stroke_w(brush, pt.press) * factor * 0.5;
        left_pts.push(Point2::new(
            pt.x + normal.x * half_w,
            pt.y + normal.y * half_w,
        ));
        right_pts.push(Point2::new(
            pt.x - normal.x * half_w,
            pt.y - normal.y * half_w,
        ));
    }
    let mut outline = Vec::new();
    for p in &left_pts {
        outline.push(*p);
    }
    let end_center = pts[n - 1].pos();
    let end_tangent = tangents[n - 1];
    let end_angle = end_tangent.y.atan2(end_tangent.x);
    let end_r = crate::brush::stroke_w(brush, pts[n - 1].press) * 0.5;
    let mut end_factor = 1.0f32;
    if total_len > 0.001 && brush.taper_end > 0.0 {
        end_factor = 0.0;
    }
    let end_r = end_r * end_factor;
    if end_r > 0.01 {
        const CAP_SEGMENTS: usize = 8;
        for i in 1..CAP_SEGMENTS {
            let t = i as f32 / CAP_SEGMENTS as f32;
            let angle = (end_angle + std::f32::consts::FRAC_PI_2) - t * std::f32::consts::PI;
            outline.push(Point2::new(
                end_center.x + angle.cos() * end_r,
                end_center.y + angle.sin() * end_r,
            ));
        }
    }
    for i in (0..n).rev() {
        outline.push(right_pts[i]);
    }
    let start_center = pts[0].pos();
    let start_tangent = tangents[0];
    let start_angle = start_tangent.y.atan2(start_tangent.x);
    let start_r = crate::brush::stroke_w(brush, pts[0].press) * 0.5;
    let mut start_factor = 1.0f32;
    if total_len > 0.001 && brush.taper_start > 0.0 {
        start_factor = 0.0;
    }
    let start_r = start_r * start_factor;
    if start_r > 0.01 {
        const CAP_SEGMENTS: usize = 8;
        for i in 1..CAP_SEGMENTS {
            let t = i as f32 / CAP_SEGMENTS as f32;
            let angle = (start_angle - std::f32::consts::FRAC_PI_2) - t * std::f32::consts::PI;
            outline.push(Point2::new(
                start_center.x + angle.cos() * start_r,
                start_center.y + angle.sin() * start_r,
            ));
        }
    }
    Some(outline)
}

pub fn compute_outline_bbox(pts: &[Point2]) -> Option<BBox> {
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
        min_x,
        min_y,
        max_x,
        max_y,
    })
}
