use crate::doc::InkDoc;
use crate::geom::polyline_intersects_polygon;
use crate::ids::{ItemId, LayerId};
use crate::item::InkItem;
use crate::point::{InkPoint, Point2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SelectionIntent {
    Replace,
    Add,
    Toggle,
}

pub fn lasso_query(doc: &InkDoc, polygon: &[Point2]) -> Vec<ItemId> {
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
    let mut hits = Vec::new();
    for id in candidates {
        if let Some(item) = doc.get_item(id) {
            match item {
                InkItem::Stroke(stroke) => {
                    let eff_xf = doc.effective_xform(stroke.id);
                    let world_pts: Vec<InkPoint> = stroke
                        .pts
                        .iter()
                        .map(|p| {
                            let mut wp = *p;
                            let p2 = eff_xf.apply(Point2::new(p.x, p.y));
                            wp.x = p2.x;
                            wp.y = p2.y;
                            wp
                        })
                        .collect();
                    if polyline_intersects_polygon(&world_pts, polygon) {
                        hits.push(id);
                    }
                }
                InkItem::Image(img) => {
                    let corners = [
                        img.xform.apply(Point2::new(0.0, 0.0)),
                        img.xform.apply(Point2::new(img.width, 0.0)),
                        img.xform.apply(Point2::new(img.width, img.height)),
                        img.xform.apply(Point2::new(0.0, img.height)),
                    ];
                    if crate::geom::polygon_intersects_polygon(&corners, polygon) {
                        hits.push(id);
                    }
                }
            }
        }
    }
    hits
}

pub fn rect_query(doc: &InkDoc, rect: crate::bbox::BBox) -> Vec<ItemId> {
    let candidates = doc.query_bbox(rect);
    let mut hits = Vec::new();
    let min_x = rect.min_x;
    let min_y = rect.min_y;
    let max_x = rect.max_x;
    let max_y = rect.max_y;
    let marquee_corners = [
        Point2::new(min_x, min_y),
        Point2::new(max_x, min_y),
        Point2::new(max_x, max_y),
        Point2::new(min_x, max_y),
    ];
    for id in candidates {
        if let Some(item) = doc.get_item(id) {
            match item {
                InkItem::Stroke(stroke) => {
                    let eff_xf = doc.effective_xform(stroke.id);
                    let world_pts: Vec<InkPoint> = stroke
                        .pts
                        .iter()
                        .map(|p| {
                            let mut wp = *p;
                            let p2 = eff_xf.apply(Point2::new(p.x, p.y));
                            wp.x = p2.x;
                            wp.y = p2.y;
                            wp
                        })
                        .collect();
                    if polyline_intersects_polygon(&world_pts, &marquee_corners) {
                        hits.push(id);
                    }
                }
                InkItem::Image(img) => {
                    let corners = [
                        img.xform.apply(Point2::new(0.0, 0.0)),
                        img.xform.apply(Point2::new(img.width, 0.0)),
                        img.xform.apply(Point2::new(img.width, img.height)),
                        img.xform.apply(Point2::new(0.0, img.height)),
                    ];
                    if crate::geom::polygon_intersects_polygon(&corners, &marquee_corners) {
                        hits.push(id);
                    }
                }
            }
        }
    }
    hits
}

pub fn apply_selection_hits(doc: &mut InkDoc, hits: &[ItemId], intent: SelectionIntent) {
    match intent {
        SelectionIntent::Replace => {
            doc.runtime.sel_items.clear();
            for &id in hits {
                doc.runtime.sel_items.insert(id);
            }
        }
        SelectionIntent::Add => {
            for &id in hits {
                doc.runtime.sel_items.insert(id);
            }
        }
        SelectionIntent::Toggle => {
            for &id in hits {
                if doc.runtime.sel_items.contains(&id) {
                    doc.runtime.sel_items.remove(&id);
                } else {
                    doc.runtime.sel_items.insert(id);
                }
            }
        }
    }
}

pub fn lasso_select(doc: &mut InkDoc, polygon: &[Point2]) -> Vec<ItemId> {
    let hits = lasso_query(doc, polygon);
    apply_selection_hits(doc, &hits, SelectionIntent::Replace);
    hits
}

pub fn select_rect(doc: &mut InkDoc, rect: crate::bbox::BBox) -> Vec<ItemId> {
    let hits = rect_query(doc, rect);
    apply_selection_hits(doc, &hits, SelectionIntent::Replace);
    hits
}

pub fn eraser_candidates(doc: &InkDoc, pos: Point2, radius: f32) -> Vec<ItemId> {
    let bbox = crate::bbox::BBox::new(
        pos.x - radius,
        pos.y - radius,
        pos.x + radius,
        pos.y + radius,
    );
    let candidates = doc.query_bbox(bbox);
    candidates
        .into_iter()
        .filter(|&id| {
            if let Some(InkItem::Stroke(s)) = doc.get_item(id) {
                doc.stroke_hit(s, pos, radius)
            } else {
                false
            }
        })
        .collect()
}

pub fn layer_of_item(doc: &InkDoc, id: ItemId) -> Option<LayerId> {
    doc.runtime
        .item_pos
        .get(&id)
        .and_then(|addr| doc.layers.get(addr.layer_idx))
        .map(|l| l.id)
}
