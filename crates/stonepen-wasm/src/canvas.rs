use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub fn get_canvas(id: &str) -> Result<HtmlCanvasElement, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let el = document
        .get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str("canvas not found"))?;
    el.dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("element is not a canvas"))
}

pub fn get_2d_context(canvas: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d, JsValue> {
    canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("no 2d context"))?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|_| JsValue::from_str("context is not 2d"))
}

pub fn sync_canvas_size(canvas: &HtmlCanvasElement, dpr: f64) {
    let w = canvas.client_width();
    let h = canvas.client_height();
    let pw = (w as f64 * dpr).round() as u32;
    let ph = (h as f64 * dpr).round() as u32;
    if canvas.width() != pw {
        canvas.set_width(pw);
    }
    if canvas.height() != ph {
        canvas.set_height(ph);
    }
}
