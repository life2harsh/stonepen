use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::bbox::BBox;
use crate::stroke::InkStroke;
use crate::xform::Xform2D;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub Uuid);

impl ItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ItemId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub Uuid);

impl AssetId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAsset {
    pub id: AssetId,
    pub mime: String,
    pub width_px: u32,
    pub height_px: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkImage {
    pub id: ItemId,
    pub asset_id: AssetId,
    pub width: f32,
    pub height: f32,
    pub opacity: f32,
    pub xform: Xform2D,
    pub local_bbox: BBox,
    pub world_bbox: BBox,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    #[serde(default)]
    pub geom_rev: u64,
}

impl InkImage {
    pub fn recompute_world_bbox(&mut self) {
        self.world_bbox = self.xform.apply_bbox(self.local_bbox);
    }
    pub fn recompute_local_bbox(&mut self) {
        self.local_bbox = BBox::new(0.0, 0.0, self.width, self.height);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InkItem {
    Stroke(InkStroke),
    Image(InkImage),
}

impl InkItem {
    pub fn id(&self) -> ItemId {
        match self {
            InkItem::Stroke(s) => s.id,
            InkItem::Image(img) => img.id,
        }
    }

    pub fn local_bbox(&self) -> BBox {
        match self {
            InkItem::Stroke(s) => s.local_bbox,
            InkItem::Image(img) => img.local_bbox,
        }
    }

    pub fn world_bbox(&self) -> BBox {
        match self {
            InkItem::Stroke(s) => s.world_bbox,
            InkItem::Image(img) => img.world_bbox,
        }
    }

    pub fn xform(&self) -> Xform2D {
        match self {
            InkItem::Stroke(s) => s.xform,
            InkItem::Image(img) => img.xform,
        }
    }

    pub fn set_xform(&mut self, xf: Xform2D) {
        match self {
            InkItem::Stroke(s) => {
                s.xform = xf;
                s.recompute_world_bbox();
            }
            InkItem::Image(img) => {
                img.xform = xf;
                img.recompute_world_bbox();
            }
        }
    }

    pub fn recompute_world_bbox(&mut self) {
        match self {
            InkItem::Stroke(s) => s.recompute_world_bbox(),
            InkItem::Image(img) => img.recompute_world_bbox(),
        }
    }

    pub fn is_stroke(&self) -> bool {
        matches!(self, InkItem::Stroke(_))
    }

    pub fn is_image(&self) -> bool {
        matches!(self, InkItem::Image(_))
    }
}
