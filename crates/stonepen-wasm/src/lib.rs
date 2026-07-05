mod app;
mod canvas;
mod file_io;
mod keyboard;
mod pointer;
mod render_2d;
mod web_runtime;
mod web_ui;

use wasm_bindgen::prelude::*;
use web_runtime::WebRuntime;

#[wasm_bindgen]
pub struct StonepenHandle {
    runtime: Option<WebRuntime>,
}

#[wasm_bindgen]
pub fn mount_stonepen(canvas_id: &str) -> Result<StonepenHandle, JsValue> {
    let runtime = WebRuntime::new(canvas_id)?;
    Ok(StonepenHandle {
        runtime: Some(runtime),
    })
}

#[wasm_bindgen]
impl StonepenHandle {
    pub fn destroy(&mut self) {
        if let Some(mut runtime) = self.runtime.take() {
            runtime.destroy();
        }
    }

    pub fn load_json(&mut self, json: &str) -> Result<(), JsValue> {
        if let Some(ref mut runtime) = self.runtime {
            runtime.app.borrow_mut().action_load(json);
            Ok(())
        } else {
            Err(JsValue::from_str("Stonepen handle is destroyed"))
        }
    }

    pub fn export_json(&self) -> Result<String, JsValue> {
        if let Some(ref runtime) = self.runtime {
            runtime
                .app
                .borrow()
                .session
                .export_json()
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
        } else {
            Err(JsValue::from_str("Stonepen handle is destroyed"))
        }
    }

    pub fn undo(&mut self) {
        if let Some(ref mut runtime) = self.runtime {
            runtime.app.borrow_mut().action_undo();
        }
    }

    pub fn redo(&mut self) {
        if let Some(ref mut runtime) = self.runtime {
            runtime.app.borrow_mut().action_redo();
        }
    }

    pub fn set_tool(&mut self, tool_id: &str) {
        if let Some(ref mut runtime) = self.runtime {
            runtime.app.borrow_mut().set_tool(tool_id);
        }
    }

    pub fn set_brush_width(&mut self, width: f32) {
        if let Some(ref mut runtime) = self.runtime {
            runtime.app.borrow_mut().set_brush_width(width);
        }
    }

    pub fn set_brush_color(&mut self, r: u8, g: u8, b: u8) {
        if let Some(ref mut runtime) = self.runtime {
            runtime.app.borrow_mut().set_brush_color(r, g, b);
        }
    }

    pub fn resize(&mut self) {
        if let Some(ref mut runtime) = self.runtime {
            runtime.app.borrow_mut().resize();
        }
    }

    pub fn redraw(&self) {
        if let Some(ref runtime) = self.runtime {
            runtime.app.borrow().redraw();
        }
    }

    pub fn is_dirty(&self) -> bool {
        if let Some(ref runtime) = self.runtime {
            runtime.app.borrow().session.dirty
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stonepen_core::bbox::BBox;
    use stonepen_core::brush::Brush;
    use stonepen_core::ids::StrokeId;
    use stonepen_core::point::PointerKind;
    use stonepen_core::session::{InkSession, Tool};
    use stonepen_core::stroke::InkStroke;
    use stonepen_core::xform::Xform2D;

    #[test]
    fn test_handle_destroy_idempotent() {
        let mut handle = StonepenHandle { runtime: None };
        handle.destroy();
        handle.destroy();
    }

    #[test]
    fn test_destroyed_handle_ignores_mutations() {
        let mut handle = StonepenHandle { runtime: None };
        assert!(handle.load_json("{}").is_err());
        assert!(handle.export_json().is_err());
        handle.undo();
        handle.redo();
        handle.set_tool("pen");
        handle.set_brush_width(5.0);
        handle.set_brush_color(255, 0, 0);
        handle.resize();
        handle.redraw();
        assert!(!handle.is_dirty());
    }

    #[test]
    fn test_gesture_start_distinguishes_real_gesture() {
        use crate::app::StonepenApp;
        assert!(StonepenApp::should_start_gesture(
            Tool::Pen,
            PointerKind::Pen,
            0
        ));
        assert!(StonepenApp::should_start_gesture(
            Tool::Pen,
            PointerKind::Mouse,
            1
        ));
        assert!(!StonepenApp::should_start_gesture(
            Tool::Pen,
            PointerKind::Mouse,
            0
        ));
        assert!(!StonepenApp::should_start_gesture(
            Tool::Pen,
            PointerKind::Touch,
            0
        ));

        assert!(StonepenApp::should_start_gesture(
            Tool::StrokeEraser,
            PointerKind::Touch,
            0
        ));
        assert!(StonepenApp::should_start_gesture(
            Tool::Lasso,
            PointerKind::Touch,
            0
        ));
        assert!(StonepenApp::should_start_gesture(
            Tool::Pan,
            PointerKind::Touch,
            0
        ));
        assert!(StonepenApp::should_start_gesture(
            Tool::Select,
            PointerKind::Touch,
            0
        ));
    }

    #[test]
    fn test_pointer_interruption_cancels_active_gesture() {
        use crate::app::{InputState, StonepenApp};
        let drawing_state = InputState::Drawing {
            ptr_id: 42,
            builder: stonepen_core::stroke::StrokeBuilder::new(Brush::default_pen()),
            parent_id: None,
            parent_xform_inv: None,
        };

        assert!(StonepenApp::should_cancel_gesture(&drawing_state, 42));
        assert!(!StonepenApp::should_cancel_gesture(&drawing_state, 99));
        assert!(!StonepenApp::should_cancel_gesture(&InputState::Idle, 42));
    }

    #[test]
    fn test_host_load_export_round_trip() {
        let mut session = InkSession::new(800.0, 600.0);
        let stroke = InkStroke {
            id: StrokeId::new(),
            parent_id: None,
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        session.add_stroke(stroke);

        let exported = session.export_json().unwrap();
        let imported = InkSession::import_json(&exported).unwrap();

        assert_eq!(imported.doc.layers.len(), session.doc.layers.len());
    }
}
