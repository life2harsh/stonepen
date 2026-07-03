use crate::brush::Brush;
use crate::ids::{LayerId, StrokeId};
use crate::layer::InkLayer;
use crate::stroke::InkStroke;
use crate::xform::Xform2D;

#[derive(Debug, Clone)]
pub enum InkOp {
    AddStroke {
        layer_id: LayerId,
        stroke: InkStroke,
    },
    DeleteStrokes {
        strokes: Vec<(LayerId, InkStroke)>,
    },
    TransformStrokes {
        stroke_ids: Vec<StrokeId>,
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
        prev_strokes: Vec<InkStroke>,
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
