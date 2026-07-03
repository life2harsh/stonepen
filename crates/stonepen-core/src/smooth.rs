use crate::point::InkPoint;

pub fn smooth_pts(pts: &[InkPoint], factor: f32) -> Vec<InkPoint> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let f = factor.clamp(0.0, 0.95);
    let inv = 1.0 - f;
    let mut out = pts.to_vec();
    for _ in 0..3 {
        let mut next = out.clone();
        for i in 1..out.len() - 1 {
            next[i].x = out[i].x * inv + (out[i - 1].x + out[i + 1].x) * 0.5 * f;
            next[i].y = out[i].y * inv + (out[i - 1].y + out[i + 1].y) * 0.5 * f;
            next[i].press = out[i].press * inv + (out[i - 1].press + out[i + 1].press) * 0.5 * f;
        }
        out = next;
    }
    out
}
