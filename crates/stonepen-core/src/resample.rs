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
