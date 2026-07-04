mod app;
mod canvas;
mod file_io;
mod keyboard;
mod pointer;
mod render_2d;
mod web_runtime;
mod web_ui;

use app::StonepenApp;
use wasm_bindgen::prelude::*;
use web_sys::{KeyboardEvent, PointerEvent, WheelEvent};

/// Bootstrap entry point. JavaScript only needs to call this.
/// Creates WebRuntime (owns all event closures and app state)
/// and intentionally leaks it for the page lifetime.
#[wasm_bindgen]
pub fn start_stonepen(canvas_id: &str) -> Result<(), JsValue> {
    let runtime = web_runtime::WebRuntime::new(canvas_id)?;
    // Intentional page-lifetime leak: the runtime owns all closures and the app.
    std::mem::forget(runtime);
    Ok(())
}

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

    pub fn on_key_down(&mut self, e: KeyboardEvent) {
        self.inner.on_key_down(&e);
    }

    pub fn on_key_up(&mut self, e: KeyboardEvent) {
        self.inner.on_key_up(&e);
    }

    pub fn on_blur(&mut self) {
        self.inner.on_blur();
    }

    pub fn reset_transient_input(&mut self) {
        self.inner.reset_transient_input();
    }

    pub fn get_shortcuts_json(&self) -> String {
        self.inner.get_shortcuts_json()
    }

    pub fn start_capture(&mut self, command_id: &str) {
        self.inner.start_capture(command_id);
    }

    pub fn cancel_capture(&mut self) {
        self.inner.cancel_capture();
    }

    pub fn is_capturing(&self) -> bool {
        self.inner.is_capturing()
    }

    pub fn capturing_label(&self) -> String {
        self.inner.capturing_label()
    }

    pub fn remove_shortcut_binding(&mut self, command_id: &str, index: usize) {
        self.inner.remove_shortcut_binding(command_id, index);
    }

    pub fn reset_shortcuts_to_defaults(&mut self) {
        self.inner.reset_shortcuts_to_defaults();
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

    pub fn paste_image(&mut self, bytes: &[u8], mime: &str, width_px: u32, height_px: u32) {
        self.inner.paste_image(bytes, mime, width_px, height_px);
    }
}
