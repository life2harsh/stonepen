use crate::point::InkPoint;

pub fn smooth_pts(pts: &[InkPoint], _factor: f32) -> Vec<InkPoint> {
    catmull_rom_spline(pts, 4)
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

pub fn catmull_rom_spline(pts: &[InkPoint], subdivisions: usize) -> Vec<InkPoint> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    let mut out = Vec::new();
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
        for j in 0..subdivisions {
            let t_frac = j as f32 / subdivisions as f32;
            let val = eval_catmull_rom(p0, p1, p2, p3, t_frac);
            out.push(val);
        }
    }
    out.push(*pts.last().unwrap());
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

fn eval_catmull_rom(
    p0: InkPoint,
    p1: InkPoint,
    p2: InkPoint,
    p3: InkPoint,
    t_frac: f32,
) -> InkPoint {
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
        return InkPoint {
            x: p1.x + (p2.x - p1.x) * t_frac,
            y: p1.y + (p2.y - p1.y) * t_frac,
            t_ms: p1.t_ms + (p2.t_ms - p1.t_ms) * t_frac as f64,
            press: p1.press + (p2.press - p1.press) * t_frac,
            tilt_x: p1.tilt_x + (p2.tilt_x - p1.tilt_x) * t_frac,
            tilt_y: p1.tilt_y + (p2.tilt_y - p1.tilt_y) * t_frac,
            twist: p1.twist + (p2.twist - p1.twist) * t_frac,
            pointer_type: p1.pointer_type,
        };
    }
    let t = t1 + (t2 - t1) * t_frac;
    let eval_field = |v0: f32, v1: f32, v2: f32, v3: f32| -> f32 {
        let a1 = ((t1 - t) * v0 + (t - t0) * v1) / (t1 - t0);
        let a2 = ((t2 - t) * v1 + (t - t1) * v2) / (t2 - t1);
        let a3 = ((t3 - t) * v2 + (t - t2) * v3) / (t3 - t2);
        let b1 = ((t2 - t) * a1 + (t - t0) * a2) / (t2 - t0);
        let b2 = ((t3 - t) * a2 + (t - t1) * a3) / (t3 - t1);
        ((t2 - t) * b1 + (t - t1) * b2) / (t2 - t1)
    };
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
