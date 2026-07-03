use crate::bbox::BBox;
use crate::ids::{LayerId, StrokeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrokeAddress {
    pub layer_idx: usize,
    pub stroke_idx: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexedStroke {
    pub layer_id: LayerId,
    pub stroke_id: StrokeId,
    pub bbox: rstar::AABB<[f32; 2]>,
}

impl rstar::RTreeObject for IndexedStroke {
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
    pub stroke_pos: std::collections::HashMap<StrokeId, StrokeAddress>,
    pub stroke_idx: rstar::RTree<IndexedStroke>,
    pub sel_strokes: std::collections::HashSet<StrokeId>,
    pub render_cache: RenderCache,
    pub dirty_regions: Vec<BBox>,
}
