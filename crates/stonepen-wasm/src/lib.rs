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
