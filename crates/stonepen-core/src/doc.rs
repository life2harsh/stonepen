use serde::{Deserialize, Serialize};

use crate::bbox::BBox;
use crate::ids::{DocId, LayerId, StrokeId};
use crate::layer::InkLayer;
use crate::point::Point2;
use crate::runtime::{IndexedStroke, InkRuntime, StrokeAddress};
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
            schema_version: 1,
            id: DocId::new(),
            width,
            height,
            background: InkBackground::Dots,
            active_layer_id,
            layers: vec![layer],
            created_at_ms: 0,
            updated_at_ms: 0,
            runtime: InkRuntime::default(),
        };
        doc.rebuild_runtime();
        doc
    }

    pub fn rebuild_runtime(&mut self) {
        self.runtime = InkRuntime::default();
        let mut entries = Vec::new();
        for (li, layer) in self.layers.iter().enumerate() {
            self.runtime.layer_pos.insert(layer.id, li);
            for (si, stroke) in layer.strokes.iter().enumerate() {
                self.runtime.stroke_pos.insert(
                    stroke.id,
                    StrokeAddress {
                        layer_idx: li,
                        stroke_idx: si,
                    },
                );
                entries.push(IndexedStroke {
                    layer_id: layer.id,
                    stroke_id: stroke.id,
                    bbox: stroke.world_bbox.to_aabb(),
                });
            }
        }
        self.runtime.stroke_idx = rstar::RTree::bulk_load(entries);
    }

    pub fn active_layer(&self) -> Option<&InkLayer> {
        let id = self.active_layer_id;
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut InkLayer> {
        let id = self.active_layer_id;
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn add_stroke(&mut self, layer_id: LayerId, stroke: InkStroke) {
        let li = match self.runtime.layer_pos.get(&layer_id) {
            Some(&idx) => idx,
            None => return,
        };
        let si = self.layers[li].strokes.len();
        self.runtime.stroke_pos.insert(
            stroke.id,
            StrokeAddress {
                layer_idx: li,
                stroke_idx: si,
            },
        );
        self.runtime.stroke_idx.insert(IndexedStroke {
            layer_id,
            stroke_id: stroke.id,
            bbox: stroke.world_bbox.to_aabb(),
        });
        self.layers[li].strokes.push(stroke);
    }

    pub fn get_stroke(&self, stroke_id: StrokeId) -> Option<&InkStroke> {
        let addr = self.runtime.stroke_pos.get(&stroke_id)?;
        self.layers
            .get(addr.layer_idx)?
            .strokes
            .get(addr.stroke_idx)
    }

    pub fn get_stroke_mut(&mut self, stroke_id: StrokeId) -> Option<&mut InkStroke> {
        let addr = *self.runtime.stroke_pos.get(&stroke_id)?;
        self.layers
            .get_mut(addr.layer_idx)?
            .strokes
            .get_mut(addr.stroke_idx)
    }

    pub fn delete_stroke(&mut self, stroke_id: StrokeId) -> Option<(LayerId, InkStroke)> {
        let addr = *self.runtime.stroke_pos.get(&stroke_id)?;
        let layer = self.layers.get_mut(addr.layer_idx)?;
        let stroke = layer.strokes.remove(addr.stroke_idx);
        let layer_id = layer.id;
        self.runtime.stroke_pos.remove(&stroke_id);
        self.rebuild_runtime();
        Some((layer_id, stroke))
    }

    pub fn delete_strokes(&mut self, stroke_ids: &[StrokeId]) -> Vec<(LayerId, InkStroke)> {
        let id_set: std::collections::HashSet<StrokeId> = stroke_ids.iter().copied().collect();
        let mut removed = Vec::new();
        for layer in &mut self.layers {
            let layer_id = layer.id;
            let mut i = 0;
            while i < layer.strokes.len() {
                if id_set.contains(&layer.strokes[i].id) {
                    let stroke = layer.strokes.remove(i);
                    removed.push((layer_id, stroke));
                } else {
                    i += 1;
                }
            }
        }
        self.rebuild_runtime();
        removed
    }

    pub fn clear_layer(&mut self, layer_id: LayerId) -> Vec<InkStroke> {
        let li = match self.runtime.layer_pos.get(&layer_id) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };
        let strokes = std::mem::take(&mut self.layers[li].strokes);
        self.rebuild_runtime();
        strokes
    }

    pub fn query_bbox(&self, bbox: BBox) -> Vec<StrokeId> {
        let aabb = bbox.to_aabb();
        self.runtime
            .stroke_idx
            .locate_in_envelope_intersecting(&aabb)
            .map(|e| e.stroke_id)
            .collect()
    }

    pub fn hit_eraser(&self, pos: Point2, radius: f32) -> Vec<StrokeId> {
        let bbox = BBox::new(
            pos.x - radius,
            pos.y - radius,
            pos.x + radius,
            pos.y + radius,
        );
        self.query_bbox(bbox)
    }

    pub fn select_lasso(&mut self, polygon: &[Point2]) -> Vec<StrokeId> {
        crate::sel::lasso_select(self, polygon)
    }

    pub fn clear_sel(&mut self) {
        self.runtime.sel_strokes.clear();
    }
}
