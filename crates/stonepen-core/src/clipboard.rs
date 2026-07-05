use crate::ids::{AssetId, ItemId, LayerId};
use crate::item::{ImageAsset, InkItem};
use crate::point::Point2;
use crate::xform::Xform2D;

#[derive(Debug, Clone)]
pub struct ClipboardItemRecord {
    pub source_layer_id: LayerId,
    pub source_layer_rank: usize,
    pub source_idx: usize,
    pub item: InkItem,
}

#[derive(Debug, Clone)]
pub struct ClipboardBundle {
    pub records: Vec<ClipboardItemRecord>,
    pub assets: Vec<ImageAsset>,
    pub source_origin: Point2,
}

impl ClipboardBundle {
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn required_asset_ids(&self) -> Vec<AssetId> {
        self.records
            .iter()
            .filter_map(|rec| {
                if let InkItem::Image(img) = &rec.item {
                    Some(img.asset_id)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn build_paste_items(
        &self,
        offset: Xform2D,
    ) -> (
        Vec<ClipboardItemRecord>,
        std::collections::HashMap<ItemId, ItemId>,
    ) {
        let mut id_map = std::collections::HashMap::new();

        for rec in &self.records {
            if let InkItem::Image(img) = &rec.item {
                id_map.insert(img.id, ItemId::new());
            }
        }

        let mut result = Vec::new();

        for rec in &self.records {
            let pasted_item = match &rec.item {
                InkItem::Image(img) => {
                    let new_id = *id_map.get(&img.id).unwrap();
                    let mut cloned = img.clone();
                    cloned.id = new_id;
                    cloned.xform = offset.concat(cloned.xform);
                    cloned.recompute_world_bbox();
                    InkItem::Image(cloned)
                }
                InkItem::Stroke(s) => {
                    let new_stroke_id = ItemId::new();
                    id_map.insert(s.id, new_stroke_id);
                    let mut cloned = s.clone();
                    cloned.id = new_stroke_id;
                    if let Some(pid) = s.parent_id {
                        if let Some(&new_pid) = id_map.get(&pid) {
                            cloned.parent_id = Some(new_pid);
                        } else {
                            cloned.parent_id = None;
                            cloned.xform = offset.concat(cloned.xform);
                        }
                    } else {
                        cloned.xform = offset.concat(cloned.xform);
                    }
                    cloned.recompute_world_bbox();
                    InkItem::Stroke(cloned)
                }
            };
            result.push(ClipboardItemRecord {
                source_layer_id: rec.source_layer_id,
                source_layer_rank: rec.source_layer_rank,
                source_idx: rec.source_idx,
                item: pasted_item,
            });
        }

        (result, id_map)
    }
}
