use js_sys::Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, HtmlCanvasElement, Url};

pub fn trigger_download(filename: &str, data: &str, mime: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let arr = Array::new();
    arr.push(&JsValue::from_str(data));
    let opts = BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = Blob::new_with_str_sequence_and_options(&arr, &opts)?;
    let url = Url::create_object_url_with_blob(&blob)?;
    let a = document
        .create_element("a")?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|_| JsValue::from_str("not anchor"))?;
    a.set_href(&url);
    a.set_download(filename);
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("no body"))?;
    body.append_child(&a)?;
    a.click();
    body.remove_child(&a)?;
    Url::revoke_object_url(&url)?;
    Ok(())
}

pub fn trigger_png_download(canvas: &HtmlCanvasElement, filename: &str) -> Result<(), JsValue> {
    let data_url = canvas.to_data_url_with_type("image/png")?;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let a = document
        .create_element("a")?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|_| JsValue::from_str("not anchor"))?;
    a.set_href(&data_url);
    a.set_download(filename);
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("no body"))?;
    body.append_child(&a)?;
    a.click();
    body.remove_child(&a)?;
    Ok(())
}
