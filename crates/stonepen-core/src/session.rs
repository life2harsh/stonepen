use std::collections::HashSet;
use thiserror::Error;

use crate::brush::Brush;
use crate::clipboard::ClipboardBundle;
use crate::doc::InkDoc;
use crate::export_json;
use crate::export_svg;
use crate::ids::{AssetId, ItemId, LayerId};
use crate::item::{ImageAsset, InkItem};
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
    #[error("invalid reorder: {0}")]
    InvalidReorder(String),
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

    pub fn select_all(&mut self) {
        self.doc.runtime.sel_items.clear();
        for layer in &self.doc.layers {
            for item in &layer.items {
                self.doc.runtime.sel_items.insert(item.id());
            }
        }
    }

    pub fn copy_sel(&self) -> Option<ClipboardBundle> {
        let sel = &self.doc.runtime.sel_items;
        if sel.is_empty() {
            return None;
        }

        let mut to_copy_ids = HashSet::new();
        for &id in sel {
            to_copy_ids.insert(id);
            if let Some(InkItem::Image(img)) = self.doc.get_item(id) {
                for kid in self.doc.attached_strokes(img.id) {
                    to_copy_ids.insert(kid);
                }
            }
        }

        let mut copied_items = Vec::new();
        let mut required_assets = Vec::new();

        // Preserve original draw order by traversing layers and items in order
        for layer in &self.doc.layers {
            for (idx, item) in layer.items.iter().enumerate() {
                if to_copy_ids.contains(&item.id()) {
                    let mut cloned = item.clone();
                    match &mut cloned {
                        InkItem::Image(img) => {
                            if let Some(asset) = self.doc.get_asset(img.asset_id) {
                                if !required_assets
                                    .iter()
                                    .any(|a: &ImageAsset| a.id == asset.id)
                                {
                                    required_assets.push(asset.clone());
                                }
                            }
                        }
                        InkItem::Stroke(s) => {
                            if let Some(pid) = s.parent_id {
                                if !to_copy_ids.contains(&pid) {
                                    // Child-only copy: make it standalone
                                    s.parent_id = None;
                                    s.xform = self.doc.effective_xform(s.id);
                                    s.recompute_world_bbox();
                                }
                            }
                        }
                    }
                    copied_items.push((idx, cloned));
                }
            }
        }

        let selection_bbox = self.doc.selection_bbox();
        let source_origin = selection_bbox
            .map(|b| Point2::new(b.min_x, b.min_y))
            .unwrap_or_else(|| Point2::new(0.0, 0.0));
        let layer_id = self.doc.active_layer_id;

        Some(ClipboardBundle {
            layer_id,
            items: copied_items,
            assets: required_assets,
            source_origin,
        })
    }

    pub fn cut_sel(&mut self) -> Option<ClipboardBundle> {
        let bundle = self.copy_sel()?;
        let sel: Vec<ItemId> = self.doc.runtime.sel_items.iter().copied().collect();
        let removed = self.doc.delete_items(&sel);
        self.doc.clear_sel();
        let mut tx = InkTx::new("cut").push(InkOp::DeleteItems {
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
        Some(bundle)
    }

    pub fn paste_sel(&mut self, bundle: &ClipboardBundle, offset: Xform2D) -> Vec<ItemId> {
        let mut asset_id_map = std::collections::HashMap::new();
        let mut assets_to_add = Vec::new();

        for asset in &bundle.assets {
            if let Some(existing) = self.doc.get_asset(asset.id) {
                let matches = existing.mime == asset.mime
                    && existing.width_px == asset.width_px
                    && existing.height_px == asset.height_px
                    && existing.bytes == asset.bytes;
                if matches {
                    asset_id_map.insert(asset.id, asset.id);
                } else {
                    let new_id = AssetId::new();
                    asset_id_map.insert(asset.id, new_id);
                    let mut new_asset = asset.clone();
                    new_asset.id = new_id;
                    assets_to_add.push(new_asset);
                }
            } else {
                asset_id_map.insert(asset.id, asset.id);
                assets_to_add.push(asset.clone());
            }
        }

        let (mut pasted_items_with_idx, id_map) = bundle.build_paste_items(offset);

        // Remap asset IDs for any image items
        for (_, item) in &mut pasted_items_with_idx {
            if let InkItem::Image(img) = item {
                if let Some(&new_aid) = asset_id_map.get(&img.asset_id) {
                    img.asset_id = new_aid;
                }
            }
        }

        let mut pasted_roots = HashSet::new();
        let mut all_pasted_ids = Vec::new();
        for (_, item) in &pasted_items_with_idx {
            let item_id = item.id();
            all_pasted_ids.push(item_id);
            let is_root = match item {
                InkItem::Image(_) => true,
                InkItem::Stroke(s) => {
                    if let Some(pid) = s.parent_id {
                        !id_map.values().any(|&new_id| new_id == pid)
                    } else {
                        true
                    }
                }
            };
            if is_root {
                pasted_roots.insert(item_id);
            }
        }

        let layer_id = self.doc.active_layer_id;
        let mut tx = InkTx::new("paste");
        for asset in assets_to_add {
            tx = tx.push(InkOp::AddAsset { asset });
        }

        let active_len = self.doc.active_layer().map(|l| l.items.len()).unwrap_or(0);
        let items_to_add: Vec<(usize, InkItem)> = pasted_items_with_idx
            .into_iter()
            .enumerate()
            .map(|(i, (_, item))| (active_len + i, item))
            .collect();

        tx = tx.push(InkOp::AddItems {
            layer_id,
            items: items_to_add,
        });

        self.do_tx(tx);

        self.doc.clear_sel();
        self.doc.runtime.sel_items = pasted_roots.clone();

        all_pasted_ids
    }

    pub fn z_order_sel(&mut self, cmd: ZOrderCmd) {
        let mut tx = InkTx::new(match cmd {
            ZOrderCmd::BringForward => "bring forward",
            ZOrderCmd::SendBackward => "send backward",
            ZOrderCmd::BringToFront => "bring to front",
            ZOrderCmd::SendToBack => "send to back",
        });

        let mut has_changes = false;

        for li in 0..self.doc.layers.len() {
            let layer = &self.doc.layers[li];
            let layer_id = layer.id;
            let mut selected_ids = Vec::new();
            let mut unselected_ids = Vec::new();
            let mut first_sel_idx = None;
            let mut last_sel_idx = None;

            for (i, item) in layer.items.iter().enumerate() {
                if is_item_selected_logical(&self.doc, item) {
                    selected_ids.push(item.id());
                    if first_sel_idx.is_none() {
                        first_sel_idx = Some(i);
                    }
                    last_sel_idx = Some(i);
                } else {
                    unselected_ids.push(item.id());
                }
            }

            if selected_ids.is_empty() {
                continue;
            }

            let first_sel_idx = first_sel_idx.unwrap();
            let last_sel_idx = last_sel_idx.unwrap();

            let mut k = 0;
            for i in last_sel_idx + 1..layer.items.len() {
                if !is_item_selected_logical(&self.doc, &layer.items[i]) {
                    k += 1;
                }
            }

            let mut j = 0;
            for i in 0..first_sel_idx {
                if !is_item_selected_logical(&self.doc, &layer.items[i]) {
                    j += 1;
                }
            }

            let n = unselected_ids.len();

            let after_order = match cmd {
                ZOrderCmd::BringToFront => {
                    if k == 0 {
                        continue;
                    }
                    let mut order = unselected_ids.clone();
                    order.extend(selected_ids);
                    order
                }
                ZOrderCmd::SendToBack => {
                    if j == 0 {
                        continue;
                    }
                    let mut order = selected_ids.clone();
                    order.extend(unselected_ids);
                    order
                }
                ZOrderCmd::BringForward => {
                    if k == 0 {
                        continue;
                    }
                    let insert_pos = n - k + 1;
                    let mut order = Vec::new();
                    order.extend_from_slice(&unselected_ids[0..insert_pos]);
                    order.extend(selected_ids);
                    order.extend_from_slice(&unselected_ids[insert_pos..n]);
                    order
                }
                ZOrderCmd::SendBackward => {
                    if j == 0 {
                        continue;
                    }
                    let insert_pos = j - 1;
                    let mut order = Vec::new();
                    order.extend_from_slice(&unselected_ids[0..insert_pos]);
                    order.extend(selected_ids);
                    order.extend_from_slice(&unselected_ids[insert_pos..n]);
                    order
                }
            };

            let before_order: Vec<ItemId> = layer.items.iter().map(|item| item.id()).collect();
            if before_order != after_order {
                tx = tx.push(InkOp::ReorderItems {
                    layer_id,
                    before_order,
                    after_order,
                });
                has_changes = true;
            }
        }

        if has_changes {
            self.do_tx(tx);
        }
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
            InkOp::ReorderItems {
                layer_id,
                after_order,
                ..
            } => {
                let _ = self.doc.reorder_items_in_layer(*layer_id, after_order);
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
                InkOp::ReorderItems {
                    layer_id,
                    before_order,
                    after_order,
                } => {
                    inv_ops.push(InkOp::ReorderItems {
                        layer_id: *layer_id,
                        before_order: after_order.clone(),
                        after_order: before_order.clone(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZOrderCmd {
    BringForward,
    SendBackward,
    BringToFront,
    SendToBack,
}

fn is_item_selected_logical(doc: &InkDoc, item: &InkItem) -> bool {
    let sel = &doc.runtime.sel_items;
    if sel.contains(&item.id()) {
        return true;
    }
    if let InkItem::Stroke(s) = item {
        if let Some(pid) = s.parent_id {
            if sel.contains(&pid) {
                return true;
            }
        }
    }
    false
}
