/// Rust-owned clipboard bundle.
///
/// Produced by copy/cut and consumed by paste.
/// The browser shell owns no clipboard state — this is entirely Rust.
use crate::ids::{AssetId, ItemId, LayerId};
use crate::item::{ImageAsset, InkItem};
use crate::point::Point2;
use crate::xform::Xform2D;

/// A copied bundle of items and required assets.
#[derive(Debug, Clone)]
pub struct ClipboardBundle {
    /// Source layer (paste targets the same layer if possible).
    pub layer_id: LayerId,
    /// Items in their original draw order. Index = original position within layer.
    pub items: Vec<(usize, InkItem)>,
    /// Image assets required by any copied image items.
    pub assets: Vec<ImageAsset>,
    /// Top-left world position of the selection bounds (for relative offset on paste).
    pub source_origin: Point2,
}

impl ClipboardBundle {
    /// Whether the bundle contains any items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Collect all required asset IDs referenced in this bundle.
    pub fn required_asset_ids(&self) -> Vec<AssetId> {
        self.items
            .iter()
            .filter_map(|(_, item)| {
                if let InkItem::Image(img) = item {
                    Some(img.asset_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Build a paste clone of the bundle: assign fresh ItemIds, remap parent_ids,
    /// and apply a world-space translation offset.
    ///
    /// Returns (new_items_per_layer, remapped_id_map)
    pub fn build_paste_items(
        &self,
        offset: Xform2D,
    ) -> (
        Vec<(usize, InkItem)>,
        std::collections::HashMap<ItemId, ItemId>,
    ) {
        let mut id_map: std::collections::HashMap<ItemId, ItemId> =
            std::collections::HashMap::new();

        // First pass: assign new IDs for images (so strokes can remap parent_id)
        for (_, item) in &self.items {
            if let InkItem::Image(img) = item {
                id_map.insert(img.id, ItemId::new());
            }
        }

        let mut result = Vec::new();
        let mut base_idx = 0usize;

        for (_orig_idx, item) in &self.items {
            match item {
                InkItem::Image(img) => {
                    let new_id = *id_map.get(&img.id).unwrap();
                    let mut cloned = img.clone();
                    cloned.id = new_id;
                    cloned.xform = offset.concat(cloned.xform);
                    cloned.recompute_world_bbox();
                    result.push((base_idx, InkItem::Image(cloned)));
                    base_idx += 1;
                }
                InkItem::Stroke(s) => {
                    let new_stroke_id = ItemId::new();
                    id_map.insert(s.id, new_stroke_id);
                    let mut cloned = s.clone();
                    cloned.id = new_stroke_id;
                    // Remap parent_id if parent was also copied
                    if let Some(pid) = s.parent_id {
                        if let Some(&new_pid) = id_map.get(&pid) {
                            cloned.parent_id = Some(new_pid);
                        } else {
                            // Parent not in bundle — detach (standalone stroke)
                            cloned.parent_id = None;
                            // Bake the effective world transform into the stroke's xform
                            // (world position is already in the stroke's points for
                            //  standalone strokes; for parent-local strokes we keep
                            //  the local-space points since we're detaching)
                            cloned.xform = offset.concat(cloned.xform);
                        }
                    } else {
                        cloned.xform = offset.concat(cloned.xform);
                    }
                    cloned.recompute_world_bbox();
                    result.push((base_idx, InkItem::Stroke(cloned)));
                    base_idx += 1;
                }
            }
        }

        (result, id_map)
    }
}
