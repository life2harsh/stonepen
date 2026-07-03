use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorRgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorRgba {
    pub fn black() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    pub fn to_css_rgba(self) -> String {
        format!(
            "rgba({},{},{},{:.3})",
            self.r,
            self.g,
            self.b,
            self.a as f32 / 255.0
        )
    }
}

impl Default for ColorRgba {
    fn default() -> Self {
        Self::black()
    }
}
