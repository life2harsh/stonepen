use serde::{Deserialize, Serialize};

use crate::bbox::BBox;
use crate::brush::Brush;
use crate::ids::{ItemId, StrokeId};
use crate::point::{InkPoint, PointerKind};
use crate::smooth::smooth_pts;
use crate::xform::Xform2D;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkStroke {
    pub id: StrokeId,
    #[serde(default)]
    pub parent_id: Option<ItemId>,
    pub brush: Brush,
    pub raw_pts: Vec<InkPoint>,
    pub pts: Vec<InkPoint>,
    pub local_bbox: BBox,
    pub world_bbox: BBox,
    pub xform: Xform2D,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    #[serde(default)]
    pub geom_rev: u64,
}

impl InkStroke {
    pub fn recompute_world_bbox(&mut self) {
        self.world_bbox = self.xform.apply_bbox(self.local_bbox);
    }
    pub fn recompute_local_bbox(&mut self) {
        if let Some(bbox) = crate::geom::compute_conservative_stroke_bbox(&self.pts, &self.brush) {
            self.local_bbox = bbox;
        }
    }
}

pub struct StrokeBuilder {
    pub brush: Brush,
    pub raw_pts: Vec<InkPoint>,
    pub pts: Vec<InkPoint>,
    pub last_press: f32,
}

impl StrokeBuilder {
    pub fn new(brush: Brush) -> Self {
        Self {
            brush,
            raw_pts: Vec::new(),
            pts: Vec::new(),
            last_press: 0.5,
        }
    }

    pub fn push(&mut self, mut pt: InkPoint) {
        let normalized = match pt.pointer_type {
            PointerKind::Pen => {
                if pt.press > 0.0 {
                    pt.press
                } else if self.raw_pts.is_empty() {
                    0.5
                } else {
                    self.last_press
                }
            }
            PointerKind::Mouse => 0.5,
            PointerKind::Touch | PointerKind::Unknown => {
                if pt.press > 0.0 {
                    pt.press
                } else {
                    0.5
                }
            }
        };
        pt.press = normalized;
        self.last_press = normalized;

        if let Some(last) = self.raw_pts.last() {
            let dx = pt.x - last.x;
            let dy = pt.y - last.y;
            if dx * dx + dy * dy < 0.01 {
                return;
            }
        }
        self.raw_pts.push(pt);

        let deduped = crate::resample::dedup_pts(&self.raw_pts, 0.5);
        let resampled = crate::resample::resample_by_distance(&deduped, 2.0);
        let mut resampled_mut = resampled;
        let alpha = (1.0 - self.brush.streamline).clamp(0.05, 0.95);
        crate::smooth::filter_pressure(&mut resampled_mut, alpha);
        self.pts = smooth_pts(&resampled_mut, self.brush.smooth);
    }

    pub fn preview_pts(&self) -> &[InkPoint] {
        &self.pts
    }

    pub fn finish(self, now_ms: i64, parent_id: Option<ItemId>) -> Option<InkStroke> {
        if self.pts.is_empty() {
            return None;
        }
        let local_bbox = crate::geom::compute_conservative_stroke_bbox(&self.pts, &self.brush)?;
        let xform = Xform2D::identity();
        let world_bbox = xform.apply_bbox(local_bbox);
        Some(InkStroke {
            id: StrokeId::new(),
            parent_id,
            brush: self.brush,
            raw_pts: self.raw_pts,
            pts: self.pts,
            local_bbox,
            world_bbox,
            xform,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            geom_rev: 0,
        })
    }
}
