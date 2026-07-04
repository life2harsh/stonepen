use thiserror::Error;

use crate::brush::Brush;
use crate::doc::InkDoc;
use crate::export_json;
use crate::export_svg;
use crate::ids::StrokeId;
use crate::ops::{InkOp, InkTx, UndoRedo};
use crate::point::{Point2, Vec2};
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
        let tx = InkTx::new("add stroke").push(InkOp::AddStroke { layer_id, stroke });
        self.do_tx(tx);
    }

    pub fn erase_at(&mut self, pos: Point2, radius: f32) {
        let candidates = self.doc.hit_eraser(pos, radius);
        if candidates.is_empty() {
            return;
        }
        let mut strokes_to_delete = Vec::new();
        for sid in candidates {
            if let Some(s) = self.doc.get_stroke(sid) {
                if crate::hit::stroke_hit(s, pos, radius) {
                    strokes_to_delete.push(sid);
                }
            }
        }
        if strokes_to_delete.is_empty() {
            return;
        }
        let removed = self.doc.delete_strokes(&strokes_to_delete);
        let tx = InkTx::new("erase").push(InkOp::DeleteStrokes { strokes: removed });
        self.undo_redo.push(tx);
        self.rev += 1;
        self.dirty = true;
    }

    pub fn delete_sel(&mut self) {
        let sel: Vec<StrokeId> = self.doc.runtime.sel_strokes.iter().copied().collect();
        if sel.is_empty() {
            return;
        }
        let removed = self.doc.delete_strokes(&sel);
        self.doc.clear_sel();
        let tx = InkTx::new("delete selection").push(InkOp::DeleteStrokes { strokes: removed });
        self.undo_redo.push(tx);
        self.rev += 1;
        self.dirty = true;
    }

    pub fn clear_active_layer(&mut self) {
        let layer_id = self.doc.active_layer_id;
        let prev_strokes = self.doc.clear_layer(layer_id);
        let tx = InkTx::new("clear layer").push(InkOp::ClearLayer {
            layer_id,
            prev_strokes,
        });
        self.undo_redo.push(tx);
        self.rev += 1;
        self.dirty = true;
    }

    pub fn select_lasso(&mut self, polygon: &[Point2]) {
        self.doc.select_lasso(polygon);
    }

    pub fn move_sel(&mut self, delta: Vec2) {
        let sel: Vec<StrokeId> = self.doc.runtime.sel_strokes.iter().copied().collect();
        if sel.is_empty() {
            return;
        }
        let mut before = Vec::new();
        let mut after = Vec::new();
        for &sid in &sel {
            if let Some(stroke) = self.doc.get_stroke(sid) {
                let old_xf = stroke.xform;
                before.push(old_xf);
                after.push(Xform2D::translate(delta.x, delta.y).concat(old_xf));
            }
        }
        for (i, &sid) in sel.iter().enumerate() {
            if let Some(stroke) = self.doc.get_stroke_mut(sid) {
                stroke.xform = after[i];
                stroke.recompute_world_bbox();
            }
        }
        self.doc.rebuild_runtime();
        let tx = InkTx::new("move selection").push(InkOp::TransformStrokes {
            stroke_ids: sel,
            before,
            after,
        });
        self.undo_redo.push(tx);
        self.rev += 1;
        self.dirty = true;
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
            InkOp::AddStroke { layer_id, stroke } => {
                self.doc.add_stroke(*layer_id, stroke.clone());
            }
            InkOp::DeleteStrokes { strokes } => {
                let ids: Vec<StrokeId> = strokes.iter().map(|(_, s)| s.id).collect();
                self.doc.delete_strokes(&ids);
            }
            InkOp::TransformStrokes {
                stroke_ids, after, ..
            } => {
                for (i, &sid) in stroke_ids.iter().enumerate() {
                    if let Some(stroke) = self.doc.get_stroke_mut(sid) {
                        stroke.xform = after[i];
                        stroke.recompute_world_bbox();
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
        }
    }

    fn invert_tx(&self, tx: &InkTx) -> InkTx {
        let mut inv_ops = Vec::new();
        for op in tx.ops.iter().rev() {
            match op {
                InkOp::AddStroke { layer_id, stroke } => {
                    inv_ops.push(InkOp::DeleteStrokes {
                        strokes: vec![(*layer_id, stroke.clone())],
                    });
                }
                InkOp::DeleteStrokes { strokes } => {
                    for (layer_id, stroke) in strokes {
                        inv_ops.push(InkOp::AddStroke {
                            layer_id: *layer_id,
                            stroke: stroke.clone(),
                        });
                    }
                }
                InkOp::TransformStrokes {
                    stroke_ids,
                    before,
                    after,
                } => {
                    inv_ops.push(InkOp::TransformStrokes {
                        stroke_ids: stroke_ids.clone(),
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
                    prev_strokes,
                } => {
                    for stroke in prev_strokes.iter().rev() {
                        inv_ops.push(InkOp::AddStroke {
                            layer_id: *layer_id,
                            stroke: stroke.clone(),
                        });
                    }
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
            }
        }
        InkTx {
            label: format!("undo: {}", tx.label),
            ops: inv_ops,
        }
    }
}
