use crate::doc::InkDoc;
use crate::session::InkError;

pub fn serialize_doc(doc: &InkDoc) -> Result<String, InkError> {
    serde_json::to_string_pretty(doc).map_err(InkError::Serialize)
}

pub fn deserialize_doc(json: &str) -> Result<InkDoc, InkError> {
    let mut doc: InkDoc = serde_json::from_str(json).map_err(InkError::Serialize)?;
    doc.rebuild_runtime();
    Ok(doc)
}
