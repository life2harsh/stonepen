use crate::bbox::BBox;
use crate::ids::ItemId;
use crate::point::Point2;
use crate::runtime::IndexedItem;

pub fn rtree_query_bbox(idx: &rstar::RTree<IndexedItem>, bbox: BBox) -> Vec<ItemId> {
    let aabb = bbox.to_aabb();
    idx.locate_in_envelope_intersecting(&aabb)
        .map(|e| e.item_id)
        .collect()
}

pub fn rtree_query_point(idx: &rstar::RTree<IndexedItem>, pos: Point2, radius: f32) -> Vec<ItemId> {
    let bbox = BBox::new(
        pos.x - radius,
        pos.y - radius,
        pos.x + radius,
        pos.y + radius,
    );
    rtree_query_bbox(idx, bbox)
}

pub fn rtree_remove(idx: &mut rstar::RTree<IndexedItem>, item_id: ItemId) {
    let to_remove: Vec<IndexedItem> = idx
        .iter()
        .filter(|e| e.item_id == item_id)
        .cloned()
        .collect();
    for entry in to_remove {
        idx.remove(&entry);
    }
}
