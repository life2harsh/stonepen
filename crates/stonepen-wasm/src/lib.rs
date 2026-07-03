mod app;
mod canvas;
mod file_io;
mod keyboard;
mod pointer;
mod render_2d;

use app::StonepenApp;
use wasm_bindgen::prelude::*;
use web_sys::{KeyboardEvent, PointerEvent, WheelEvent};

#[wasm_bindgen]
pub struct WasmApp {
    inner: StonepenApp,
}

#[wasm_bindgen]
impl WasmApp {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<WasmApp, JsValue> {
        let inner = StonepenApp::new(canvas_id)?;
        Ok(WasmApp { inner })
    }

    pub fn on_pointer_down(&mut self, e: PointerEvent) {
        self.inner.on_pointer_down(&e);
    }

    pub fn on_pointer_move(&mut self, e: PointerEvent) {
        self.inner.on_pointer_move(&e);
    }

    pub fn on_pointer_up(&mut self, e: PointerEvent) {
        self.inner.on_pointer_up(&e);
    }

    pub fn on_pointer_cancel(&mut self, e: PointerEvent) {
        self.inner.on_pointer_cancel(&e);
    }

    pub fn on_wheel(&mut self, e: WheelEvent) {
        self.inner.on_wheel(&e);
    }

    pub fn on_key(&mut self, e: KeyboardEvent) {
        self.inner.on_key(&e);
    }

    pub fn set_tool(&mut self, tool: &str) {
        self.inner.set_tool(tool);
    }

    pub fn set_brush_color(&mut self, r: u8, g: u8, b: u8) {
        self.inner.set_brush_color(r, g, b);
    }

    pub fn set_brush_width(&mut self, w: f32) {
        self.inner.set_brush_width(w);
    }

    pub fn action_undo(&mut self) {
        self.inner.action_undo();
    }

    pub fn action_redo(&mut self) {
        self.inner.action_redo();
    }

    pub fn action_clear(&mut self) {
        self.inner.action_clear();
    }

    pub fn action_save(&mut self) {
        self.inner.action_save();
    }

    pub fn action_load(&mut self, json: &str) {
        self.inner.action_load(json);
    }

    pub fn action_export_svg(&self) {
        self.inner.action_export_svg();
    }

    pub fn action_export_png(&self) {
        self.inner.action_export_png();
    }

    pub fn resize(&mut self) {
        self.inner.resize();
    }

    pub fn redraw(&self) {
        self.inner.redraw();
    }
}
