use crate::point::InkPoint;

pub fn smooth_pts(pts: &[InkPoint], factor: f32) -> Vec<InkPoint> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let f = factor.clamp(0.0, 0.95);
    let inv = 1.0 - f;
    let mut out = pts.to_vec();
    for _ in 0..2 {
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

pub fn filter_pressure(pts: &mut [InkPoint], alpha: f32) {
    if pts.is_empty() {
        return;
    }
    let mut prev_press = pts[0].press;
    for pt in pts.iter_mut().skip(1) {
        let smooth_press = prev_press + alpha * (pt.press - prev_press);
        pt.press = smooth_press;
        prev_press = smooth_press;
    }
}

pub fn adaptive_catmull_rom(pts: &[InkPoint], zoom: f32) -> Vec<InkPoint> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    let max_err_px = 0.35f32;
    let tolerance = max_err_px / zoom;
    let mut out = Vec::new();
    out.push(pts[0]);
    for i in 0..pts.len() - 1 {
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p0 = if i == 0 {
            extrapolate_pt(p1, p2, -1.0)
        } else {
            pts[i - 1]
        };
        let p3 = if i + 1 == pts.len() - 1 {
            extrapolate_pt(p1, p2, 2.0)
        } else {
            pts[i + 2]
        };
        let alpha = 0.5f32;
        let get_t = |prev: InkPoint, curr: InkPoint, t_prev: f32| -> f32 {
            let dx = curr.x - prev.x;
            let dy = curr.y - prev.y;
            let dist_sq = dx * dx + dy * dy;
            t_prev + dist_sq.powf(alpha * 0.5)
        };
        let t0 = 0.0f32;
        let t1 = get_t(p0, p1, t0);
        let t2 = get_t(p1, p2, t1);
        let t3 = get_t(p2, p3, t2);
        if (t1 - t0).abs() < 1e-4 || (t2 - t1).abs() < 1e-4 || (t3 - t2).abs() < 1e-4 {
            out.push(p2);
        } else {
            recursive_subdivide(
                p0, p1, p2, p3, t0, t1, t2, t3, t1, t2, p1, p2, tolerance, 0, &mut out,
            );
        }
    }
    out
}

fn extrapolate_pt(p1: InkPoint, p2: InkPoint, factor: f32) -> InkPoint {
    InkPoint {
        x: p1.x + (p2.x - p1.x) * factor,
        y: p1.y + (p2.y - p1.y) * factor,
        t_ms: p1.t_ms + (p2.t_ms - p1.t_ms) * factor as f64,
        press: (p1.press + (p2.press - p1.press) * factor).clamp(0.0, 1.0),
        tilt_x: p1.tilt_x + (p2.tilt_x - p1.tilt_x) * factor,
        tilt_y: p1.tilt_y + (p2.tilt_y - p1.tilt_y) * factor,
        twist: p1.twist + (p2.twist - p1.twist) * factor,
        pointer_type: p1.pointer_type,
    }
}

fn eval_catmull_rom_knots(
    p0: InkPoint,
    p1: InkPoint,
    p2: InkPoint,
    p3: InkPoint,
    t0: f32,
    t1: f32,
    t2: f32,
    t3: f32,
    t: f32,
) -> InkPoint {
    let eval_field = |v0: f32, v1: f32, v2: f32, v3: f32| -> f32 {
        let a1 = ((t1 - t) * v0 + (t - t0) * v1) / (t1 - t0);
        let a2 = ((t2 - t) * v1 + (t - t1) * v2) / (t2 - t1);
        let a3 = ((t3 - t) * v2 + (t - t2) * v3) / (t3 - t2);
        let b1 = ((t2 - t) * a1 + (t - t0) * a2) / (t2 - t0);
        let b2 = ((t3 - t) * a2 + (t - t1) * a3) / (t3 - t1);
        ((t2 - t) * b1 + (t - t1) * b2) / (t2 - t1)
    };
    let t_frac = (t - t1) / (t2 - t1);
    let t_ms = p1.t_ms + (p2.t_ms - p1.t_ms) * t_frac as f64;
    InkPoint {
        x: eval_field(p0.x, p1.x, p2.x, p3.x),
        y: eval_field(p0.y, p1.y, p2.y, p3.y),
        t_ms,
        press: eval_field(p0.press, p1.press, p2.press, p3.press).clamp(0.0, 1.0),
        tilt_x: eval_field(p0.tilt_x, p1.tilt_x, p2.tilt_x, p3.tilt_x),
        tilt_y: eval_field(p0.tilt_y, p1.tilt_y, p2.tilt_y, p3.tilt_y),
        twist: eval_field(p0.twist, p1.twist, p2.twist, p3.twist),
        pointer_type: p1.pointer_type,
    }
}

fn recursive_subdivide(
    p0: InkPoint,
    p1: InkPoint,
    p2: InkPoint,
    p3: InkPoint,
    t0: f32,
    t1: f32,
    t2: f32,
    t3: f32,
    ta: f32,
    tb: f32,
    pt_a: InkPoint,
    pt_b: InkPoint,
    tolerance: f32,
    depth: usize,
    out: &mut Vec<InkPoint>,
) {
    if depth >= 6 {
        out.push(pt_b);
        return;
    }
    let tm = (ta + tb) * 0.5;
    let pt_m = eval_catmull_rom_knots(p0, p1, p2, p3, t0, t1, t2, t3, tm);
    let dx = pt_b.x - pt_a.x;
    let dy = pt_b.y - pt_a.y;
    let len_sq = dx * dx + dy * dy;
    let dist = if len_sq < 1e-6 {
        let mx = pt_m.x - pt_a.x;
        let my = pt_m.y - pt_a.y;
        (mx * mx + my * my).sqrt()
    } else {
        let t = ((pt_m.x - pt_a.x) * dx + (pt_m.y - pt_a.y) * dy) / len_sq;
        let t_clamped = t.clamp(0.0, 1.0);
        let px = pt_a.x + t_clamped * dx;
        let py = pt_a.y + t_clamped * dy;
        let mx = pt_m.x - px;
        let my = pt_m.y - py;
        (mx * mx + my * my).sqrt()
    };
    if dist > tolerance {
        recursive_subdivide(
            p0, p1, p2, p3, t0, t1, t2, t3, ta, tm, pt_a, pt_m, tolerance, depth + 1, out,
        );
        recursive_subdivide(
            p0, p1, p2, p3, t0, t1, t2, t3, tm, tb, pt_m, pt_b, tolerance, depth + 1, out,
        );
    } else {
        out.push(pt_b);
    }
}
