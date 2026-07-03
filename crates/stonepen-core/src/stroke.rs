use serde::{Deserialize, Serialize};

use crate::bbox::BBox;
use crate::brush::Brush;
use crate::geom::compute_bbox;
use crate::ids::StrokeId;
use crate::point::InkPoint;
use crate::resample::resample_by_distance;
use crate::smooth::smooth_pts;
use crate::xform::Xform2D;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkStroke {
    pub id: StrokeId,
    pub brush: Brush,
    pub raw_pts: Vec<InkPoint>,
    pub pts: Vec<InkPoint>,
    pub local_bbox: BBox,
    pub world_bbox: BBox,
    pub xform: Xform2D,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl InkStroke {
    pub fn recompute_world_bbox(&mut self) {
        self.world_bbox = self.xform.apply_bbox(self.local_bbox);
    }
}

pub struct StrokeBuilder {
    pub brush: Brush,
    pub raw_pts: Vec<InkPoint>,
    pub preview_pts: Vec<InkPoint>,
}

impl StrokeBuilder {
    pub fn new(brush: Brush) -> Self {
        Self {
            brush,
            raw_pts: Vec::new(),
            preview_pts: Vec::new(),
        }
    }

    pub fn push(&mut self, pt: InkPoint) {
        if let Some(last) = self.raw_pts.last() {
            let dx = pt.x - last.x;
            let dy = pt.y - last.y;
            if dx * dx + dy * dy < 0.01 {
                return;
            }
        }
        self.raw_pts.push(pt);
        self.preview_pts = smooth_pts(&self.raw_pts, self.brush.smooth);
    }

    pub fn preview_pts(&self) -> &[InkPoint] {
        &self.preview_pts
    }

    pub fn finish(self, now_ms: i64) -> Option<InkStroke> {
        if self.raw_pts.is_empty() {
            return None;
        }
        let resampled = resample_by_distance(&self.raw_pts, 2.0);
        let pts = smooth_pts(&resampled, self.brush.smooth);
        let half_w = self.brush.base_w * 0.5;
        let local_bbox = compute_bbox(&pts, half_w)?;
        let xform = Xform2D::identity();
        let world_bbox = xform.apply_bbox(local_bbox);
        Some(InkStroke {
            id: StrokeId::new(),
            brush: self.brush,
            raw_pts: self.raw_pts,
            pts,
            local_bbox,
            world_bbox,
            xform,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        })
    }
}
