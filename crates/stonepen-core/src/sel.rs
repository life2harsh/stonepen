use crate::doc::InkDoc;
use crate::geom::polyline_intersects_polygon;
use crate::hit::stroke_hit;
use crate::ids::{LayerId, StrokeId};
use crate::point::Point2;

pub fn lasso_select(doc: &mut InkDoc, polygon: &[Point2]) -> Vec<StrokeId> {
    if polygon.len() < 3 {
        return Vec::new();
    }
    let min_x = polygon.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let min_y = polygon.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_x = polygon
        .iter()
        .map(|p| p.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_y = polygon
        .iter()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let bbox = crate::bbox::BBox::new(min_x, min_y, max_x, max_y);
    let candidates = doc.query_bbox(bbox);
    let mut sel = Vec::new();
    for sid in candidates {
        if let Some(stroke) = doc.get_stroke(sid) {
            if polyline_intersects_polygon(&stroke.pts, polygon) {
                sel.push(sid);
            }
        }
    }
    doc.runtime.sel_strokes.clear();
    for &sid in &sel {
        doc.runtime.sel_strokes.insert(sid);
    }
    sel
}

pub fn eraser_candidates(doc: &InkDoc, pos: Point2, radius: f32) -> Vec<StrokeId> {
    let bbox = crate::bbox::BBox::new(
        pos.x - radius,
        pos.y - radius,
        pos.x + radius,
        pos.y + radius,
    );
    let candidates = doc.query_bbox(bbox);
    candidates
        .into_iter()
        .filter(|&sid| {
            doc.get_stroke(sid)
                .map(|s| stroke_hit(s, pos, radius))
                .unwrap_or(false)
        })
        .collect()
}

pub fn layer_of_stroke(doc: &InkDoc, stroke_id: StrokeId) -> Option<LayerId> {
    doc.runtime
        .stroke_pos
        .get(&stroke_id)
        .and_then(|addr| doc.layers.get(addr.layer_idx))
        .map(|l| l.id)
}
