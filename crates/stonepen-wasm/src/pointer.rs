use stonepen_core::point::{InkPoint, PointerKind};
use stonepen_core::viewport::Viewport;
use web_sys::PointerEvent;

pub struct PointerInput {
    pub id: i32,
    pub kind: PointerKind,
    pub x: f32,
    pub y: f32,
    pub press: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub twist: f32,
    pub t_ms: f64,
    pub _primary: bool,
    pub buttons: u16,
}

impl PointerInput {
    pub fn from_event(e: &PointerEvent) -> Self {
        Self {
            id: e.pointer_id(),
            kind: parse_pointer_kind(&e.pointer_type()),
            x: e.client_x() as f32,
            y: e.client_y() as f32,
            press: e.pressure(),
            tilt_x: e.tilt_x() as f32,
            tilt_y: e.tilt_y() as f32,
            twist: e.twist() as f32,
            t_ms: e.time_stamp(),
            _primary: e.is_primary(),
            buttons: e.buttons(),
        }
    }

    pub fn to_ink_point(&self, vp: &Viewport) -> InkPoint {
        let world = vp.screen_to_world(stonepen_core::point::Point2::new(self.x, self.y));
        InkPoint {
            x: world.x,
            y: world.y,
            t_ms: self.t_ms,
            press: self.press.max(0.001),
            tilt_x: self.tilt_x,
            tilt_y: self.tilt_y,
            twist: self.twist,
            pointer_type: self.kind,
        }
    }
}

pub fn parse_pointer_kind(s: &str) -> PointerKind {
    match s {
        "pen" => PointerKind::Pen,
        "touch" => PointerKind::Touch,
        "mouse" => PointerKind::Mouse,
        _ => PointerKind::Unknown,
    }
}

pub fn get_inputs(e: &PointerEvent) -> Vec<PointerInput> {
    vec![PointerInput::from_event(e)]
}
