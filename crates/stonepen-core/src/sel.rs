use crate::doc::InkDoc;
use crate::geom::polyline_intersects_polygon;
use crate::hit::stroke_hit;
use crate::ids::{ItemId, LayerId};
use crate::item::InkItem;
use crate::point::{InkPoint, Point2};

pub fn lasso_select(doc: &mut InkDoc, polygon: &[Point2]) -> Vec<ItemId> {
    if polygon.len() < 3 {
        doc.runtime.sel_items.clear();
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
    for id in candidates {
        if let Some(item) = doc.get_item(id) {
            match item {
                InkItem::Stroke(stroke) => {
                    let world_pts: Vec<InkPoint> = stroke
                        .pts
                        .iter()
                        .map(|p| {
                            let mut wp = *p;
                            let p2 = stroke.xform.apply(Point2::new(p.x, p.y));
                            wp.x = p2.x;
                            wp.y = p2.y;
                            wp
                        })
                        .collect();
                    if polyline_intersects_polygon(&world_pts, polygon) {
                        sel.push(id);
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
                        sel.push(id);
                    }
                }
            }
        }
    }
    doc.runtime.sel_items.clear();
    for &id in &sel {
        doc.runtime.sel_items.insert(id);
    }
    sel
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
                stroke_hit(s, pos, radius)
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
