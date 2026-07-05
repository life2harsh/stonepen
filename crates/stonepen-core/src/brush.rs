use serde::{Deserialize, Serialize};

use crate::color::ColorRgba;
use crate::ids::BrushId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrushKind {
    Pen,
    Pencil,
    Highlighter,
    Marker,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Brush {
    pub id: BrushId,
    pub name: String,
    pub kind: BrushKind,
    pub color: ColorRgba,
    pub base_w: f32,
    pub opacity: f32,
    pub min_press: f32,
    pub max_press: f32,
    pub smooth: f32,
    pub streamline: f32,
    pub taper_start: f32,
    pub taper_end: f32,
}

impl Brush {
    pub fn default_pen() -> Self {
        Self {
            id: BrushId::new(),
            name: "Pen".into(),
            kind: BrushKind::Pen,
            color: ColorRgba::black(),
            base_w: 3.0,
            opacity: 1.0,
            min_press: 0.1,
            max_press: 1.0,
            smooth: 0.5,
            streamline: 0.5,
            taper_start: 0.0,
            taper_end: 2.0,
        }
    }

    pub fn default_pencil() -> Self {
        Self {
            id: BrushId::new(),
            name: "Pencil".into(),
            kind: BrushKind::Pencil,
            color: ColorRgba {
                r: 60,
                g: 60,
                b: 60,
                a: 200,
            },
            base_w: 2.0,
            opacity: 0.8,
            min_press: 0.05,
            max_press: 1.0,
            smooth: 0.3,
            streamline: 0.3,
            taper_start: 0.0,
            taper_end: 0.0,
        }
    }

    pub fn default_highlighter() -> Self {
        Self {
            id: BrushId::new(),
            name: "Highlighter".into(),
            kind: BrushKind::Highlighter,
            color: ColorRgba {
                r: 255,
                g: 255,
                b: 0,
                a: 100,
            },
            base_w: 16.0,
            opacity: 0.5,
            min_press: 0.5,
            max_press: 1.0,
            smooth: 0.8,
            streamline: 0.8,
            taper_start: 0.0,
            taper_end: 0.0,
        }
    }
}

pub fn stroke_w(brush: &Brush, press: f32) -> f32 {
    match brush.kind {
        BrushKind::Highlighter => brush.base_w,
        _ => {
            let p = press.clamp(0.0, 1.0);
            if p <= 0.0 {
                return 0.0;
            }
            let shaped = p * p * (3.0 - 2.0 * p);
            brush.base_w * (brush.min_press + shaped * (brush.max_press - brush.min_press))
        }
    }
}
