use crate::point::InkPoint;

pub fn resample_by_distance(pts: &[InkPoint], spacing: f32) -> Vec<InkPoint> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    if spacing <= 0.001 {
        return pts.to_vec();
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
    let mut out = Vec::new();
    out.push(pts[0]);
    let mut target = spacing;
    let mut i = 0;
    while target <= total_len {
        while i < n - 1 && dists[i + 1] < target {
            i += 1;
        }
        if i >= n - 1 {
            break;
        }
        let d_seg = dists[i + 1] - dists[i];
        if d_seg > 1e-5 {
            let t = (target - dists[i]) / d_seg;
            out.push(interpolate_point(pts[i], pts[i + 1], t));
        }
        target += spacing;
    }
    let last = pts[n - 1];
    if let Some(&l) = out.last() {
        let dx = last.x - l.x;
        let dy = last.y - l.y;
        if (dx * dx + dy * dy).sqrt() > 0.1 {
            out.push(last);
        }
    } else {
        out.push(last);
    }
    out
}

fn interpolate_point(p0: InkPoint, p1: InkPoint, t: f32) -> InkPoint {
    let t_ms = p0.t_ms + (p1.t_ms - p0.t_ms) * t as f64;
    InkPoint {
        x: p0.x + (p1.x - p0.x) * t,
        y: p0.y + (p1.y - p0.y) * t,
        t_ms,
        press: p0.press + (p1.press - p0.press) * t,
        tilt_x: p0.tilt_x + (p1.tilt_x - p0.tilt_x) * t,
        tilt_y: p0.tilt_y + (p1.tilt_y - p0.tilt_y) * t,
        twist: p0.twist + (p1.twist - p0.twist) * t,
        pointer_type: p0.pointer_type,
    }
}

pub fn dedup_pts(pts: &[InkPoint], min_dist: f32) -> Vec<InkPoint> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(pts.len());
    out.push(pts[0]);
    let min_dist_sq = min_dist * min_dist;
    for &pt in pts.iter().skip(1) {
        if let Some(last) = out.last() {
            let dx = pt.x - last.x;
            let dy = pt.y - last.y;
            if dx * dx + dy * dy >= min_dist_sq {
                out.push(pt);
            }
        }
    }
    if let Some(&last_raw) = pts.last() {
        if let Some(&last_added) = out.last() {
            let dx = last_raw.x - last_added.x;
            let dy = last_raw.y - last_added.y;
            if dx * dx + dy * dy >= 0.001 {
                out.push(last_raw);
            }
        }
    }
    out
}
