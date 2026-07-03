use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StrokeId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BrushId(pub Uuid);

impl DocId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocId {
    fn default() -> Self {
        Self::new()
    }
}

impl LayerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for LayerId {
    fn default() -> Self {
        Self::new()
    }
}

impl StrokeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for StrokeId {
    fn default() -> Self {
        Self::new()
    }
}

impl BrushId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BrushId {
    fn default() -> Self {
        Self::new()
    }
}
