use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::doc::{InkBackground, InkDoc};
use crate::ids::{AssetId, DocId, LayerId};
use crate::item::{ImageAsset, InkItem};
use crate::layer::InkLayer;
use crate::session::InkError;

#[derive(Serialize)]
struct ImageAssetSerialization {
    id: AssetId,
    mime: String,
    width_px: u32,
    height_px: u32,
    bytes: String,
}

#[derive(Serialize)]
struct InkDocSerialization<'a> {
    schema_version: u32,
    id: DocId,
    width: f32,
    height: f32,
    background: InkBackground,
    active_layer_id: LayerId,
    layers: &'a [InkLayer],
    assets: Vec<ImageAssetSerialization>,
    created_at_ms: i64,
    updated_at_ms: i64,
}

#[derive(Deserialize)]
struct ImageAssetMigration {
    id: AssetId,
    mime: String,
    width_px: u32,
    height_px: u32,
    bytes: String,
}

#[derive(Deserialize)]
struct InkLayerMigration {
    id: LayerId,
    name: String,
    visible: bool,
    locked: bool,
    opacity: f32,
    strokes: Option<Vec<crate::stroke::InkStroke>>,
    items: Option<Vec<InkItem>>,
}

#[derive(Deserialize)]
struct InkDocMigration {
    _schema_version: Option<u32>,
    id: DocId,
    width: f32,
    height: f32,
    background: InkBackground,
    active_layer_id: LayerId,
    layers: Vec<InkLayerMigration>,
    #[serde(default)]
    assets: Vec<ImageAssetMigration>,
    created_at_ms: i64,
    updated_at_ms: i64,
}

pub fn serialize_doc(doc: &InkDoc) -> Result<String, InkError> {
    let assets = doc
        .assets
        .iter()
        .map(|a| ImageAssetSerialization {
            id: a.id,
            mime: a.mime.clone(),
            width_px: a.width_px,
            height_px: a.height_px,
            bytes: base64::engine::general_purpose::STANDARD.encode(&a.bytes),
        })
        .collect();

    let serializable = InkDocSerialization {
        schema_version: 3,
        id: doc.id,
        width: doc.width,
        height: doc.height,
        background: doc.background,
        active_layer_id: doc.active_layer_id,
        layers: &doc.layers,
        assets,
        created_at_ms: doc.created_at_ms,
        updated_at_ms: doc.updated_at_ms,
    };

    serde_json::to_string(&serializable).map_err(Into::into)
}

pub fn deserialize_doc(json: &str) -> Result<InkDoc, InkError> {
    let raw: InkDocMigration = serde_json::from_str(json)?;
    let mut assets = Vec::new();
    for a in raw.assets {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&a.bytes)
            .map_err(|e| {
                InkError::Serialize(<serde_json::Error as serde::de::Error>::custom(format!(
                    "invalid base64: {e}"
                )))
            })?;
        assets.push(ImageAsset {
            id: a.id,
            mime: a.mime,
            width_px: a.width_px,
            height_px: a.height_px,
            bytes,
        });
    }

    let mut layers = Vec::new();
    for l in raw.layers {
        let items = if let Some(items) = l.items {
            items
        } else if let Some(strokes) = l.strokes {
            strokes.into_iter().map(InkItem::Stroke).collect()
        } else {
            Vec::new()
        };
        layers.push(InkLayer {
            id: l.id,
            name: l.name,
            visible: l.visible,
            locked: l.locked,
            opacity: l.opacity,
            items,
        });
    }

    let mut seen_ids = std::collections::HashSet::new();
    let mut image_ids = std::collections::HashSet::new();

    for layer in &layers {
        for item in &layer.items {
            if !seen_ids.insert(item.id()) {
                return Err(InkError::Serialize(serde::de::Error::custom(
                    "duplicate ItemId detected",
                )));
            }
            if let InkItem::Image(img) = item {
                image_ids.insert(img.id);
                if !assets.iter().any(|a| a.id == img.asset_id) {
                    return Err(InkError::Serialize(serde::de::Error::custom(
                        "missing asset reference",
                    )));
                }
            }
        }
    }

    for layer in &layers {
        for item in &layer.items {
            if let InkItem::Stroke(s) = item {
                if let Some(pid) = s.parent_id {
                    if pid == s.id {
                        return Err(InkError::Serialize(serde::de::Error::custom(
                            "self-parenting detected",
                        )));
                    }
                    if !image_ids.contains(&pid) {
                        return Err(InkError::Serialize(serde::de::Error::custom(
                            "parent not found or parent is not an image",
                        )));
                    }
                }
            }
        }
    }

    let mut doc = InkDoc {
        schema_version: 3,
        id: raw.id,
        width: raw.width,
        height: raw.height,
        background: raw.background,
        active_layer_id: raw.active_layer_id,
        layers,
        assets,
        created_at_ms: raw.created_at_ms,
        updated_at_ms: raw.updated_at_ms,
        runtime: crate::runtime::InkRuntime::default(),
    };
    doc.rebuild_runtime();
    Ok(doc)
}
