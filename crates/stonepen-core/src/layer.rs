use serde::{Deserialize, Serialize};

use crate::ids::LayerId;
use crate::item::InkItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkLayer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub items: Vec<InkItem>,
}

impl InkLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            items: Vec::new(),
        }
    }
}
