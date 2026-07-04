use thiserror::Error;

use crate::brush::Brush;
use crate::doc::InkDoc;
use crate::export_json;
use crate::export_svg;
use crate::ids::{ItemId, LayerId};
use crate::item::InkItem;
use crate::ops::{InkOp, InkTx, UndoRedo};
use crate::point::Point2;
use crate::stroke::InkStroke;
use crate::xform::Xform2D;

#[derive(Debug, Clone, PartialEq)]
pub enum Tool {
    Pen,
    Pencil,
    Highlighter,
    StrokeEraser,
    Lasso,
    Pan,
    Select,
}

#[derive(Debug, Error)]
pub enum InkError {
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("no active layer")]
    NoActiveLayer,
    #[error("layer not found")]
    LayerNotFound,
}

pub struct InkSession {
    pub doc: InkDoc,
    pub active_tool: Tool,
    pub active_brush: Brush,
    pub undo_redo: UndoRedo,
    pub dirty: bool,
    pub last_saved_rev: u64,
    pub rev: u64,
}

impl InkSession {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            doc: InkDoc::new(width, height),
            active_tool: Tool::Pen,
            active_brush: Brush::default_pen(),
            undo_redo: UndoRedo::default(),
            dirty: false,
            last_saved_rev: 0,
            rev: 0,
        }
    }

    pub fn do_tx(&mut self, tx: InkTx) {
        self.apply_ops(&tx.ops);
        self.undo_redo.push(tx);
        self.rev += 1;
        self.dirty = true;
    }

    pub fn undo(&mut self) {
        if let Some(tx) = self.undo_redo.pop_undo() {
            let inverse = self.invert_tx(&tx);
            self.apply_ops(&inverse.ops);
            self.undo_redo.push_redo(tx);
            self.rev += 1;
            self.dirty = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(tx) = self.undo_redo.pop_redo() {
            self.apply_ops(&tx.ops);
            self.undo_redo.push_undo_after_redo(tx);
            self.rev += 1;
            self.dirty = true;
        }
    }

    pub fn add_stroke(&mut self, stroke: InkStroke) {
        let layer_id = self.doc.active_layer_id;
        let tx = InkTx::new("add stroke").push(InkOp::AddItems {
            layer_id,
            items: vec![(
                self.doc.active_layer().map(|l| l.items.len()).unwrap_or(0),
                InkItem::Stroke(stroke),
            )],
        });
        self.do_tx(tx);
    }

    pub fn erase_at(&mut self, pos: Point2, radius: f32) {
        let candidates = self.doc.hit_eraser(pos, radius);
        if candidates.is_empty() {
            return;
        }
        let mut items_to_delete = Vec::new();
        for id in candidates {
            if let Some(InkItem::Stroke(s)) = self.doc.get_item(id) {
                if self.doc.stroke_hit(s, pos, radius) {
                    items_to_delete.push(id);
                }
            }
        }
        if items_to_delete.is_empty() {
            return;
        }
        let removed = self.doc.delete_items(&items_to_delete);
        let tx = InkTx::new("erase").push(InkOp::DeleteItems { items: removed });
        self.do_tx(tx);
    }

    pub fn delete_sel(&mut self) {
        let sel: Vec<ItemId> = self.doc.runtime.sel_items.iter().copied().collect();
        if sel.is_empty() {
            return;
        }
        let removed = self.doc.delete_items(&sel);
        self.doc.clear_sel();
        let mut tx = InkTx::new("delete").push(InkOp::DeleteItems {
            items: removed.clone(),
        });
        for (_, _, item) in &removed {
            if let InkItem::Image(img) = item {
                if !self.doc.has_asset_references(img.asset_id) {
                    if let Some(asset) = self.doc.get_asset(img.asset_id) {
                        tx = tx.push(InkOp::DeleteAsset {
                            asset: asset.clone(),
                        });
                    }
                }
            }
        }
        self.do_tx(tx);
    }

    pub fn clear_active_layer(&mut self) {
        let layer_id = self.doc.active_layer_id;
        let prev_items = self.doc.clear_layer(layer_id);
        let mut tx = InkTx::new("clear layer").push(InkOp::ClearLayer {
            layer_id,
            prev_items: prev_items.clone(),
        });
        for item in &prev_items {
            if let InkItem::Image(img) = item {
                if !self.doc.has_asset_references(img.asset_id) {
                    if let Some(asset) = self.doc.get_asset(img.asset_id) {
                        tx = tx.push(InkOp::DeleteAsset {
                            asset: asset.clone(),
                        });
                    }
                }
            }
        }
        self.do_tx(tx);
    }

    pub fn select_lasso(&mut self, polygon: &[Point2]) {
        self.doc.select_lasso(polygon);
    }

    pub fn duplicate_sel(&mut self) {
        let sel: Vec<ItemId> = self.doc.runtime.sel_items.iter().copied().collect();
        if sel.is_empty() {
            return;
        }

        let mut image_id_map = std::collections::HashMap::new();
        for &id in &sel {
            if let Some(InkItem::Image(_)) = self.doc.get_item(id) {
                image_id_map.insert(id, ItemId::new());
            }
        }

        let mut dup_ops = Vec::new();
        let offset = Xform2D::translate(15.0, 15.0);

        for layer in &self.doc.layers {
            let mut layer_dups = Vec::new();
            for (item_idx, item) in layer.items.iter().enumerate() {
                let id = item.id();
                if let InkItem::Image(img) = item {
                    if sel.contains(&id) {
                        let new_id = *image_id_map.get(&id).unwrap();
                        let mut cloned_img = img.clone();
                        cloned_img.id = new_id;
                        cloned_img.xform = offset.concat(cloned_img.xform);
                        cloned_img.recompute_world_bbox();
                        layer_dups.push((item_idx, InkItem::Image(cloned_img)));
                    }
                }
                if let InkItem::Stroke(s) = item {
                    if let Some(parent_id) = s.parent_id {
                        if let Some(&new_parent_id) = image_id_map.get(&parent_id) {
                            let mut cloned_stroke = s.clone();
                            cloned_stroke.id = ItemId::new();
                            cloned_stroke.parent_id = Some(new_parent_id);
                            cloned_stroke.recompute_world_bbox();
                            layer_dups.push((item_idx, InkItem::Stroke(cloned_stroke)));
                            continue;
                        }
                    }
                    if sel.contains(&id) {
                        let mut cloned_stroke = s.clone();
                        cloned_stroke.id = ItemId::new();
                        cloned_stroke.xform = offset.concat(cloned_stroke.xform);
                        cloned_stroke.recompute_world_bbox();
                        layer_dups.push((item_idx, InkItem::Stroke(cloned_stroke)));
                    }
                }
            }

            if !layer_dups.is_empty() {
                let mut adjusted_dups = Vec::new();
                let mut shift = 0;
                for (orig_idx, dup_item) in layer_dups {
                    adjusted_dups.push((orig_idx + 1 + shift, dup_item));
                    shift += 1;
                }
                dup_ops.push((layer.id, adjusted_dups));
            }
        }

        if dup_ops.is_empty() {
            return;
        }

        let mut tx = InkTx::new("duplicate");
        let mut new_sel = std::collections::HashSet::new();
        for (layer_id, items) in dup_ops {
            for (_, item) in &items {
                new_sel.insert(item.id());
            }
            tx = tx.push(InkOp::AddItems { layer_id, items });
        }

        self.do_tx(tx);
        self.doc.clear_sel();
        self.doc.runtime.sel_items = new_sel;
    }

    pub fn export_json(&self) -> Result<String, InkError> {
        export_json::serialize_doc(&self.doc)
    }

    pub fn import_json(json: &str) -> Result<Self, InkError> {
        let doc = export_json::deserialize_doc(json)?;
        Ok(Self {
            doc,
            active_tool: Tool::Pen,
            active_brush: Brush::default_pen(),
            undo_redo: UndoRedo::default(),
            dirty: false,
            last_saved_rev: 0,
            rev: 0,
        })
    }

    pub fn export_svg(&self) -> Result<String, InkError> {
        export_svg::export_svg(&self.doc)
    }

    fn apply_ops(&mut self, ops: &[InkOp]) {
        for op in ops {
            self.apply_op(op);
        }
    }

    fn apply_op(&mut self, op: &InkOp) {
        match op {
            InkOp::AddItems { layer_id, items } => {
                self.doc.add_items(*layer_id, items.clone());
            }
            InkOp::DeleteItems { items } => {
                let ids: Vec<ItemId> = items.iter().map(|(_, _, item)| item.id()).collect();
                self.doc.delete_items(&ids);
            }
            InkOp::TransformItems {
                item_ids, after, ..
            } => {
                for (i, &id) in item_ids.iter().enumerate() {
                    if let Some(item) = self.doc.get_item_mut(id) {
                        item.set_xform(after[i]);
                    }
                }
                self.doc.rebuild_runtime();
            }
            InkOp::SetStrokeBrush {
                stroke_ids, after, ..
            } => {
                for &sid in stroke_ids {
                    if let Some(stroke) = self.doc.get_stroke_mut(sid) {
                        stroke.brush = after.clone();
                        stroke.geom_rev += 1;
                        stroke.recompute_local_bbox();
                        stroke.recompute_world_bbox();
                    }
                }
                self.doc.rebuild_runtime();
            }
            InkOp::ClearLayer { layer_id, .. } => {
                self.doc.clear_layer(*layer_id);
            }
            InkOp::AddLayer { layer, idx } => {
                self.doc.layers.insert(*idx, layer.clone());
                self.doc.rebuild_runtime();
            }
            InkOp::DeleteLayer { idx, .. } => {
                if *idx < self.doc.layers.len() {
                    self.doc.layers.remove(*idx);
                    self.doc.rebuild_runtime();
                }
            }
            InkOp::ReorderLayer {
                layer_id,
                old_idx: _,
                new_idx,
            } => {
                if let Some(pos) = self.doc.layers.iter().position(|l| l.id == *layer_id) {
                    let layer = self.doc.layers.remove(pos);
                    let insert_at = (*new_idx).min(self.doc.layers.len());
                    self.doc.layers.insert(insert_at, layer);
                    self.doc.rebuild_runtime();
                }
            }
            InkOp::SetActiveLayer { next, .. } => {
                self.doc.active_layer_id = *next;
            }
            InkOp::AddAsset { asset } => {
                self.doc.add_asset(asset.clone());
            }
            InkOp::DeleteAsset { asset } => {
                self.doc.delete_asset(asset.id);
            }
        }
    }

    fn invert_tx(&self, tx: &InkTx) -> InkTx {
        let mut inv_ops = Vec::new();
        for op in tx.ops.iter().rev() {
            match op {
                InkOp::AddItems { layer_id, items } => {
                    inv_ops.push(InkOp::DeleteItems {
                        items: items
                            .iter()
                            .map(|(idx, item)| (*layer_id, *idx, item.clone()))
                            .collect(),
                    });
                }
                InkOp::DeleteItems { items } => {
                    let mut items_by_layer: std::collections::HashMap<
                        LayerId,
                        Vec<(usize, InkItem)>,
                    > = std::collections::HashMap::new();
                    for (layer_id, idx, item) in items {
                        items_by_layer
                            .entry(*layer_id)
                            .or_default()
                            .push((*idx, item.clone()));
                    }
                    for (layer_id, items) in items_by_layer {
                        inv_ops.push(InkOp::AddItems { layer_id, items });
                    }
                }
                InkOp::TransformItems {
                    item_ids,
                    before,
                    after,
                } => {
                    inv_ops.push(InkOp::TransformItems {
                        item_ids: item_ids.clone(),
                        before: after.clone(),
                        after: before.clone(),
                    });
                }
                InkOp::SetStrokeBrush {
                    stroke_ids,
                    before,
                    after,
                } => {
                    inv_ops.push(InkOp::SetStrokeBrush {
                        stroke_ids: stroke_ids.clone(),
                        before: vec![after.clone()],
                        after: before[0].clone(),
                    });
                }
                InkOp::ClearLayer {
                    layer_id,
                    prev_items,
                } => {
                    inv_ops.push(InkOp::AddItems {
                        layer_id: *layer_id,
                        items: prev_items
                            .iter()
                            .enumerate()
                            .map(|(idx, item)| (idx, item.clone()))
                            .collect(),
                    });
                }
                InkOp::AddLayer { layer, idx } => {
                    inv_ops.push(InkOp::DeleteLayer {
                        layer: layer.clone(),
                        idx: *idx,
                    });
                }
                InkOp::DeleteLayer { layer, idx } => {
                    inv_ops.push(InkOp::AddLayer {
                        layer: layer.clone(),
                        idx: *idx,
                    });
                }
                InkOp::ReorderLayer {
                    layer_id,
                    old_idx,
                    new_idx,
                } => {
                    inv_ops.push(InkOp::ReorderLayer {
                        layer_id: *layer_id,
                        old_idx: *new_idx,
                        new_idx: *old_idx,
                    });
                }
                InkOp::SetActiveLayer { prev, next } => {
                    inv_ops.push(InkOp::SetActiveLayer {
                        prev: *next,
                        next: *prev,
                    });
                }
                InkOp::AddAsset { asset } => {
                    inv_ops.push(InkOp::DeleteAsset {
                        asset: asset.clone(),
                    });
                }
                InkOp::DeleteAsset { asset } => {
                    inv_ops.push(InkOp::AddAsset {
                        asset: asset.clone(),
                    });
                }
            }
        }
        InkTx {
            label: format!("undo: {}", tx.label),
            ops: inv_ops,
        }
    }
}
