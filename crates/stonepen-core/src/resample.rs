use crate::point::InkPoint;

pub fn resample_by_distance(pts: &[InkPoint], min_dist: f32) -> Vec<InkPoint> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(pts.len());
    out.push(pts[0]);
    let mut accum = 0.0f32;
    for i in 1..pts.len() {
        let prev = pts[i - 1];
        let curr = pts[i];
        let dx = curr.x - prev.x;
        let dy = curr.y - prev.y;
        let seg_len = (dx * dx + dy * dy).sqrt();
        accum += seg_len;
        if accum >= min_dist {
            out.push(curr);
            accum = 0.0;
        }
    }
    let last = pts[pts.len() - 1];
    if let Some(l) = out.last() {
        if (l.x - last.x).abs() > 0.001 || (l.y - last.y).abs() > 0.001 {
            out.push(last);
        }
    }
    if out.len() < 2 {
        out.push(last);
    }
    out
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
