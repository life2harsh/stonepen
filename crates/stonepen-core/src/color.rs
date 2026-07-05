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

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl Default for ColorRgba {
    fn default() -> Self {
        Self::black()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_hex() {
        assert_eq!(
            ColorRgba {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            }
            .to_hex(),
            "#000000"
        );
        assert_eq!(
            ColorRgba {
                r: 60,
                g: 60,
                b: 60,
                a: 255
            }
            .to_hex(),
            "#3c3c3c"
        );
        assert_eq!(
            ColorRgba {
                r: 255,
                g: 255,
                b: 0,
                a: 255
            }
            .to_hex(),
            "#ffff00"
        );
    }
}
