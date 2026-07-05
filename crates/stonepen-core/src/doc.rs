use serde::{Deserialize, Serialize};

use crate::bbox::BBox;
use crate::ids::{AssetId, DocId, ItemId, LayerId, StrokeId};
use crate::item::{ImageAsset, InkItem};
use crate::layer::InkLayer;
use crate::point::Point2;
use crate::runtime::{IndexedItem, InkRuntime, ItemAddress};
use crate::stroke::InkStroke;
use crate::xform::Xform2D;

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
            schema_version: 3,
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

        let mut image_xforms = std::collections::HashMap::new();
        for layer in &self.layers {
            for item in &layer.items {
                if let InkItem::Image(img) = item {
                    image_xforms.insert(img.id, img.xform);
                }
            }
        }

        let mut entries = Vec::new();
        for (li, layer) in self.layers.iter_mut().enumerate() {
            self.runtime.layer_pos.insert(layer.id, li);
            for (ii, item) in layer.items.iter_mut().enumerate() {
                match item {
                    InkItem::Stroke(s) => {
                        let eff_xf = if let Some(pid) = s.parent_id {
                            if let Some(&pxf) = image_xforms.get(&pid) {
                                pxf.concat(s.xform)
                            } else {
                                s.xform
                            }
                        } else {
                            s.xform
                        };
                        s.world_bbox = eff_xf.apply_bbox(s.local_bbox);
                    }
                    InkItem::Image(img) => {
                        img.world_bbox = img.xform.apply_bbox(img.local_bbox);
                    }
                }
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
        self.runtime
            .sel_items
            .retain(|id| self.runtime.item_pos.contains_key(id));
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

    pub fn effective_xform(&self, id: ItemId) -> Xform2D {
        if let Some(item) = self.get_item(id) {
            match item {
                InkItem::Stroke(s) => {
                    if let Some(parent_id) = s.parent_id {
                        if let Some(parent_item) = self.get_item(parent_id) {
                            parent_item.xform().concat(s.xform)
                        } else {
                            s.xform
                        }
                    } else {
                        s.xform
                    }
                }
                InkItem::Image(img) => img.xform,
            }
        } else {
            Xform2D::identity()
        }
    }

    pub fn attached_strokes(&self, image_id: ItemId) -> Vec<ItemId> {
        let mut kids = Vec::new();
        for layer in &self.layers {
            for item in &layer.items {
                if let InkItem::Stroke(s) = item {
                    if s.parent_id == Some(image_id) {
                        kids.push(s.id);
                    }
                }
            }
        }
        kids
    }
    pub fn transform_roots(&self) -> std::collections::HashSet<ItemId> {
        let sel = &self.runtime.sel_items;
        let mut roots = std::collections::HashSet::new();
        for &id in sel {
            let mut is_root = true;
            if let Some(item) = self.get_item(id) {
                if let InkItem::Stroke(s) = item {
                    if let Some(pid) = s.parent_id {
                        if sel.contains(&pid) {
                            is_root = false;
                        }
                    }
                }
            }
            if is_root {
                roots.insert(id);
            }
        }
        roots
    }

    pub fn selection_closure(&self) -> std::collections::HashSet<ItemId> {
        let mut closure = self.runtime.sel_items.clone();
        for &id in &self.runtime.sel_items {
            if let Some(item) = self.get_item(id) {
                if let InkItem::Image(img) = item {
                    for kid in self.attached_strokes(img.id) {
                        closure.insert(kid);
                    }
                }
            }
        }
        closure
    }

    pub fn annotation_target_image(&self) -> Option<ItemId> {
        let roots = self.transform_roots();
        if roots.len() == 1 {
            let root_id = *roots.iter().next().unwrap();
            if let Some(InkItem::Image(_)) = self.get_item(root_id) {
                return Some(root_id);
            }
        }
        None
    }

    pub fn apply_world_xform_to_item(
        &mut self,
        id: ItemId,
        world_xf: Xform2D,
        orig_local_xf: Xform2D,
    ) {
        let parent_id = if let Some(InkItem::Stroke(s)) = self.get_item(id) {
            s.parent_id
        } else {
            None
        };

        if let Some(pid) = parent_id {
            let parent_xform = if let Some(parent_item) = self.get_item(pid) {
                parent_item.xform()
            } else {
                Xform2D::identity()
            };
            if let Some(inv) = parent_xform.inverse() {
                if let Some(InkItem::Stroke(s)) = self.get_item_mut(id) {
                    s.xform = inv
                        .concat(world_xf)
                        .concat(parent_xform)
                        .concat(orig_local_xf);
                    s.recompute_world_bbox();
                }
            }
        } else {
            if let Some(item) = self.get_item_mut(id) {
                match item {
                    InkItem::Image(img) => {
                        img.xform = world_xf.concat(orig_local_xf);
                        img.recompute_world_bbox();
                    }
                    InkItem::Stroke(s) => {
                        s.xform = world_xf.concat(orig_local_xf);
                        s.recompute_world_bbox();
                    }
                }
            }
        }
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
        let mut id_set: std::collections::HashSet<ItemId> = ids.iter().copied().collect();
        let mut to_add = Vec::new();
        for &id in &id_set {
            if let Some(item) = self.get_item(id) {
                if let InkItem::Image(img) = item {
                    for kid in self.attached_strokes(img.id) {
                        to_add.push(kid);
                    }
                }
            }
        }
        for kid in to_add {
            id_set.insert(kid);
        }

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
        let include_item = |b: BBox, cur_bbox: &mut Option<BBox>| {
            if let Some(mut cur) = *cur_bbox {
                cur.min_x = cur.min_x.min(b.min_x);
                cur.min_y = cur.min_y.min(b.min_y);
                cur.max_x = cur.max_x.max(b.max_x);
                cur.max_y = cur.max_y.max(b.max_y);
                *cur_bbox = Some(cur);
            } else {
                *cur_bbox = Some(b);
            }
        };

        for &id in &self.runtime.sel_items {
            if let Some(item) = self.get_item(id) {
                include_item(item.world_bbox(), &mut bbox);
                if let InkItem::Image(img) = item {
                    for kid_id in self.attached_strokes(img.id) {
                        if let Some(kid) = self.get_item(kid_id) {
                            include_item(kid.world_bbox(), &mut bbox);
                        }
                    }
                }
            }
        }
        bbox
    }

    pub fn stroke_hit(&self, stroke: &InkStroke, pos: Point2, radius: f32) -> bool {
        let eff_xf = self.effective_xform(stroke.id);
        crate::hit::stroke_hit_with_xform(stroke, eff_xf, pos, radius)
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
                    InkItem::Stroke(s) => self.stroke_hit(s, pos, world_tol),
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

    /// Reorder items within a single layer to match the given ID sequence.
    /// The `new_order` must contain exactly the same set of ItemIds as the
    /// current layer without duplicates or omissions.
    pub fn reorder_items_in_layer(
        &mut self,
        layer_id: LayerId,
        new_order: &[ItemId],
    ) -> Result<(), crate::session::InkError> {
        let li = match self.runtime.layer_pos.get(&layer_id) {
            Some(&idx) => idx,
            None => return Err(crate::session::InkError::LayerNotFound),
        };
        let layer = &self.layers[li];

        // 1. Length check
        if new_order.len() != layer.items.len() {
            return Err(crate::session::InkError::InvalidReorder(format!(
                "Length mismatch: new order has {}, layer has {}",
                new_order.len(),
                layer.items.len()
            )));
        }

        // 2. Presence & duplicate check
        let mut existing_ids = std::collections::HashSet::new();
        for item in &layer.items {
            existing_ids.insert(item.id());
        }

        let mut seen = std::collections::HashSet::new();
        for id in new_order {
            if !existing_ids.contains(id) {
                return Err(crate::session::InkError::InvalidReorder(format!(
                    "Item ID {} not found in target layer",
                    id.0
                )));
            }
            if !seen.insert(*id) {
                return Err(crate::session::InkError::InvalidReorder(format!(
                    "Duplicate Item ID {} in new order",
                    id.0
                )));
            }
        }

        // Safe mutation phase
        let mut id_to_item: std::collections::HashMap<ItemId, InkItem> = self.layers[li]
            .items
            .drain(..)
            .map(|item| (item.id(), item))
            .collect();

        for &id in new_order {
            if let Some(item) = id_to_item.remove(&id) {
                self.layers[li].items.push(item);
            }
        }

        self.rebuild_runtime();
        Ok(())
    }
}
