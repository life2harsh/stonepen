use crate::bbox::BBox;
use crate::ids::StrokeId;
use crate::point::Point2;
use crate::runtime::IndexedStroke;

pub fn rtree_query_bbox(idx: &rstar::RTree<IndexedStroke>, bbox: BBox) -> Vec<StrokeId> {
    let aabb = bbox.to_aabb();
    idx.locate_in_envelope_intersecting(&aabb)
        .map(|e| e.stroke_id)
        .collect()
}

pub fn rtree_query_point(
    idx: &rstar::RTree<IndexedStroke>,
    pos: Point2,
    radius: f32,
) -> Vec<StrokeId> {
    let bbox = BBox::new(
        pos.x - radius,
        pos.y - radius,
        pos.x + radius,
        pos.y + radius,
    );
    rtree_query_bbox(idx, bbox)
}

pub fn rtree_remove(idx: &mut rstar::RTree<IndexedStroke>, stroke_id: StrokeId) {
    let to_remove: Vec<IndexedStroke> = idx
        .iter()
        .filter(|e| e.stroke_id == stroke_id)
        .cloned()
        .collect();
    for entry in to_remove {
        idx.remove(&entry);
    }
}
