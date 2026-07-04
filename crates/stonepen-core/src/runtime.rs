use crate::bbox::BBox;
use crate::ids::{ItemId, LayerId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemAddress {
    pub layer_idx: usize,
    pub item_idx: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexedItem {
    pub layer_id: LayerId,
    pub item_id: ItemId,
    pub bbox: rstar::AABB<[f32; 2]>,
}

impl rstar::RTreeObject for IndexedItem {
    type Envelope = rstar::AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.bbox
    }
}

#[derive(Debug, Default, Clone)]
pub struct RenderCache {}

#[derive(Debug, Default)]
pub struct InkRuntime {
    pub layer_pos: std::collections::HashMap<LayerId, usize>,
    pub item_pos: std::collections::HashMap<ItemId, ItemAddress>,
    pub item_idx: rstar::RTree<IndexedItem>,
    pub sel_items: std::collections::HashSet<ItemId>,
    pub render_cache: RenderCache,
    pub dirty_regions: Vec<BBox>,
}
