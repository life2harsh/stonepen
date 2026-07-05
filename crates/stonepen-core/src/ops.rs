use crate::brush::Brush;
use crate::ids::{ItemId, LayerId, StrokeId};
use crate::item::{ImageAsset, InkItem};
use crate::layer::InkLayer;
use crate::xform::Xform2D;

#[derive(Debug, Clone)]
pub enum InkOp {
    AddItems {
        layer_id: LayerId,
        items: Vec<(usize, InkItem)>,
    },
    DeleteItems {
        items: Vec<(LayerId, usize, InkItem)>,
    },
    TransformItems {
        item_ids: Vec<ItemId>,
        before: Vec<Xform2D>,
        after: Vec<Xform2D>,
    },
    SetStrokeBrush {
        stroke_ids: Vec<StrokeId>,
        before: Vec<Brush>,
        after: Brush,
    },
    ClearLayer {
        layer_id: LayerId,
        prev_items: Vec<InkItem>,
    },
    AddLayer {
        layer: InkLayer,
        idx: usize,
    },
    DeleteLayer {
        layer: InkLayer,
        idx: usize,
    },
    ReorderLayer {
        layer_id: LayerId,
        old_idx: usize,
        new_idx: usize,
    },
    SetActiveLayer {
        prev: LayerId,
        next: LayerId,
    },
    AddAsset {
        asset: ImageAsset,
    },
    DeleteAsset {
        asset: ImageAsset,
    },
    /// Reorder items within one layer. Stores the full item-id order before and after.
    ReorderItems {
        layer_id: LayerId,
        before_order: Vec<ItemId>,
        after_order: Vec<ItemId>,
    },
}

#[derive(Debug, Clone)]
pub struct InkTx {
    pub label: String,
    pub ops: Vec<InkOp>,
}

impl InkTx {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ops: Vec::new(),
        }
    }

    pub fn push(mut self, op: InkOp) -> Self {
        self.ops.push(op);
        self
    }
}

pub struct UndoRedo {
    pub undo_stack: Vec<InkTx>,
    pub redo_stack: Vec<InkTx>,
    pub max_depth: usize,
}

impl UndoRedo {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    pub fn push(&mut self, tx: InkTx) {
        self.redo_stack.clear();
        self.undo_stack.push(tx);
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
    }

    pub fn pop_undo(&mut self) -> Option<InkTx> {
        self.undo_stack.pop()
    }

    pub fn pop_redo(&mut self) -> Option<InkTx> {
        self.redo_stack.pop()
    }

    pub fn push_redo(&mut self, tx: InkTx) {
        self.redo_stack.push(tx);
    }

    pub fn push_undo_after_redo(&mut self, tx: InkTx) {
        self.undo_stack.push(tx);
    }
}

impl Default for UndoRedo {
    fn default() -> Self {
        Self::new(128)
    }
}
