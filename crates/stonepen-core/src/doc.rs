use serde::{Deserialize, Serialize};

use crate::bbox::BBox;
use crate::ids::{AssetId, DocId, ItemId, LayerId, StrokeId};
use crate::item::{ImageAsset, InkItem};
use crate::layer::InkLayer;
use crate::point::Point2;
use crate::runtime::{IndexedItem, InkRuntime, ItemAddress};
use crate::stroke::InkStroke;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InkBackground {
    Plain,
    Dots,
    Grid,
    Ruled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InkDoc {
    pub schema_version: u32,
    pub id: DocId,
    pub width: f32,
    pub height: f32,
    pub background: InkBackground,
    pub active_layer_id: LayerId,
    pub layers: Vec<InkLayer>,
    #[serde(default)]
    pub assets: Vec<ImageAsset>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,

    #[serde(skip)]
    pub runtime: InkRuntime,
}

impl Clone for InkDoc {
    fn clone(&self) -> Self {
        let mut doc = Self {
            schema_version: self.schema_version,
            id: self.id,
            width: self.width,
            height: self.height,
            background: self.background,
            active_layer_id: self.active_layer_id,
            layers: self.layers.clone(),
            assets: self.assets.clone(),
            created_at_ms: self.created_at_ms,
            updated_at_ms: self.updated_at_ms,
            runtime: InkRuntime::default(),
        };
        doc.rebuild_runtime();
        doc
    }
}

impl InkDoc {
    pub fn new(width: f32, height: f32) -> Self {
        let layer = InkLayer::new("Layer 1");
        let active_layer_id = layer.id;
        let mut doc = Self {
            schema_version: 2,
            id: DocId::new(),
            width,
            height,
            background: InkBackground::Dots,
            active_layer_id,
            layers: vec![layer],
            assets: Vec::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
            runtime: InkRuntime::default(),
        };
        doc.rebuild_runtime();
        doc
    }

    pub fn rebuild_runtime(&mut self) {
        self.runtime.layer_pos.clear();
        self.runtime.item_pos.clear();
        let mut entries = Vec::new();
        for (li, layer) in self.layers.iter().enumerate() {
            self.runtime.layer_pos.insert(layer.id, li);
            for (ii, item) in layer.items.iter().enumerate() {
                self.runtime.item_pos.insert(
                    item.id(),
                    ItemAddress {
                        layer_idx: li,
                        item_idx: ii,
                    },
                );
                entries.push(IndexedItem {
                    layer_id: layer.id,
                    item_id: item.id(),
                    bbox: item.world_bbox().to_aabb(),
                });
            }
        }
        self.runtime.item_idx = rstar::RTree::bulk_load(entries);
    }

    pub fn active_layer(&self) -> Option<&InkLayer> {
        let id = self.active_layer_id;
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut InkLayer> {
        let id = self.active_layer_id;
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn add_item(&mut self, layer_id: LayerId, item: InkItem) {
        let li = match self.runtime.layer_pos.get(&layer_id) {
            Some(&idx) => idx,
            None => return,
        };
        self.layers[li].items.push(item);
        self.rebuild_runtime();
    }

    pub fn add_items(&mut self, layer_id: LayerId, mut items: Vec<(usize, InkItem)>) {
        let li = match self.runtime.layer_pos.get(&layer_id) {
            Some(&idx) => idx,
            None => return,
        };
        items.sort_by_key(|(idx, _)| *idx);
        for (idx, item) in items {
            let insert_idx = idx.min(self.layers[li].items.len());
            self.layers[li].items.insert(insert_idx, item);
        }
        self.rebuild_runtime();
    }

    pub fn add_stroke(&mut self, layer_id: LayerId, stroke: InkStroke) {
        self.add_item(layer_id, InkItem::Stroke(stroke));
    }

    pub fn get_item(&self, id: ItemId) -> Option<&InkItem> {
        let addr = self.runtime.item_pos.get(&id)?;
        self.layers.get(addr.layer_idx)?.items.get(addr.item_idx)
    }

    pub fn get_item_mut(&mut self, id: ItemId) -> Option<&mut InkItem> {
        let addr = *self.runtime.item_pos.get(&id)?;
        self.layers
            .get_mut(addr.layer_idx)?
            .items
            .get_mut(addr.item_idx)
    }

    pub fn get_stroke(&self, stroke_id: StrokeId) -> Option<&InkStroke> {
        match self.get_item(stroke_id)? {
            InkItem::Stroke(s) => Some(s),
            _ => None,
        }
    }

    pub fn get_stroke_mut(&mut self, stroke_id: StrokeId) -> Option<&mut InkStroke> {
        match self.get_item_mut(stroke_id)? {
            InkItem::Stroke(s) => Some(s),
            _ => None,
        }
    }

    pub fn delete_items(&mut self, ids: &[ItemId]) -> Vec<(LayerId, usize, InkItem)> {
        let id_set: std::collections::HashSet<ItemId> = ids.iter().copied().collect();
        let mut removed = Vec::new();
        for layer in &mut self.layers {
            let layer_id = layer.id;
            for i in (0..layer.items.len()).rev() {
                if id_set.contains(&layer.items[i].id()) {
                    let item = layer.items.remove(i);
                    removed.push((layer_id, i, item));
                }
            }
        }
        self.rebuild_runtime();
        removed
    }

    pub fn clear_layer(&mut self, layer_id: LayerId) -> Vec<InkItem> {
        let li = match self.runtime.layer_pos.get(&layer_id) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };
        let items = std::mem::take(&mut self.layers[li].items);
        self.rebuild_runtime();
        items
    }

    pub fn query_bbox(&self, bbox: BBox) -> Vec<ItemId> {
        let aabb = bbox.to_aabb();
        self.runtime
            .item_idx
            .locate_in_envelope_intersecting(&aabb)
            .map(|e| e.item_id)
            .collect()
    }

    pub fn hit_eraser(&self, pos: Point2, radius: f32) -> Vec<ItemId> {
        let bbox = BBox::new(
            pos.x - radius,
            pos.y - radius,
            pos.x + radius,
            pos.y + radius,
        );
        self.query_bbox(bbox)
    }

    pub fn select_lasso(&mut self, polygon: &[Point2]) -> Vec<ItemId> {
        crate::sel::lasso_select(self, polygon)
    }

    pub fn clear_sel(&mut self) {
        self.runtime.sel_items.clear();
    }

    pub fn add_asset(&mut self, asset: ImageAsset) {
        self.assets.retain(|a| a.id != asset.id);
        self.assets.push(asset);
    }

    pub fn delete_asset(&mut self, id: AssetId) -> Option<ImageAsset> {
        if let Some(pos) = self.assets.iter().position(|a| a.id == id) {
            Some(self.assets.remove(pos))
        } else {
            None
        }
    }

    pub fn get_asset(&self, id: AssetId) -> Option<&ImageAsset> {
        self.assets.iter().find(|a| a.id == id)
    }

    pub fn has_asset_references(&self, id: AssetId) -> bool {
        for layer in &self.layers {
            for item in &layer.items {
                if let InkItem::Image(img) = item {
                    if img.asset_id == id {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn selection_bbox(&self) -> Option<BBox> {
        let mut bbox: Option<BBox> = None;
        for &id in &self.runtime.sel_items {
            if let Some(item) = self.get_item(id) {
                let b = item.world_bbox();
                if let Some(mut cur) = bbox {
                    cur.min_x = cur.min_x.min(b.min_x);
                    cur.min_y = cur.min_y.min(b.min_y);
                    cur.max_x = cur.max_x.max(b.max_x);
                    cur.max_y = cur.max_y.max(b.max_y);
                    bbox = Some(cur);
                } else {
                    bbox = Some(b);
                }
            }
        }
        bbox
    }

    pub fn hit_test_item(&self, pos: Point2, screen_tol: f32, zoom: f32) -> Option<ItemId> {
        let world_tol = screen_tol / zoom;
        let bbox = BBox::new(
            pos.x - world_tol,
            pos.y - world_tol,
            pos.x + world_tol,
            pos.y + world_tol,
        );
        let candidates = self.query_bbox(bbox);
        let mut best_id: Option<ItemId> = None;
        let mut best_addr: Option<ItemAddress> = None;
        for id in candidates {
            if let Some(item) = self.get_item(id) {
                let hit = match item {
                    InkItem::Stroke(s) => crate::hit::stroke_hit(s, pos, world_tol),
                    InkItem::Image(img) => {
                        if let Some(inv) = img.xform.inverse() {
                            let lp = inv.apply(pos);
                            lp.x >= 0.0 && lp.x <= img.width && lp.y >= 0.0 && lp.y <= img.height
                        } else {
                            false
                        }
                    }
                };
                if hit {
                    if let Some(addr) = self.runtime.item_pos.get(&id) {
                        let is_better = match best_addr {
                            None => true,
                            Some(b) => {
                                if addr.layer_idx != b.layer_idx {
                                    addr.layer_idx > b.layer_idx
                                } else {
                                    addr.item_idx > b.item_idx
                                }
                            }
                        };
                        if is_better {
                            best_id = Some(id);
                            best_addr = Some(*addr);
                        }
                    }
                }
            }
        }
        best_id
    }
}
