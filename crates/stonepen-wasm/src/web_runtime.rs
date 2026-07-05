/// WebRuntime — browser event lifecycle owner.
///
/// Owns:
/// - The StonepenApp (via Rc<RefCell<>>)
/// - The WebUi
/// - All registered event closures (must remain alive)
/// - The ResizeObserver handle
///
/// Created by `mount_stonepen`. Owned by `StonepenHandle`.
use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{
    AddEventListenerOptions, ClipboardEvent, Element, Event, FileReader, HtmlCanvasElement,
    HtmlInputElement, KeyboardEvent, PointerEvent, ProgressEvent, ResizeObserver, WheelEvent,
};

use crate::app::{InputState, StonepenApp};
use crate::web_ui::WebUi;
use stonepen_core::session::Tool;
use stonepen_core::shortcuts::Command;

struct Listener {
    target: web_sys::EventTarget,
    event_type: String,
    callback: js_sys::Function,
    options: Option<web_sys::AddEventListenerOptions>,
    // Drop of _closure will automatically clean up/free the Wasm Closure.
    _closure: Box<dyn std::any::Any>,
}

impl Listener {
    fn remove(&self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(&self.event_type, &self.callback);
    }
}

pub struct WebRuntime {
    pub app: Rc<RefCell<StonepenApp>>,
    pub ui: Rc<WebUi>,
    listeners: Vec<Listener>,
    resize_observer: Option<ResizeObserver>,
    _resize_cb: Option<Closure<dyn FnMut(js_sys::Array)>>,
}

impl WebRuntime {
    fn reg_pointer(
        listeners: &mut Vec<Listener>,
        target: web_sys::EventTarget,
        event_type: &str,
        closure: Closure<dyn FnMut(PointerEvent)>,
    ) -> Result<(), JsValue> {
        let callback = closure.as_ref().clone().unchecked_into::<js_sys::Function>();
        target.add_event_listener_with_callback(event_type, &callback)?;
        listeners.push(Listener {
            target,
            event_type: event_type.to_string(),
            callback,
            options: None,
            _closure: Box::new(closure),
        });
        Ok(())
    }

    fn reg_keyboard(
        listeners: &mut Vec<Listener>,
        target: web_sys::EventTarget,
        event_type: &str,
        closure: Closure<dyn FnMut(KeyboardEvent)>,
    ) -> Result<(), JsValue> {
        let callback = closure.as_ref().clone().unchecked_into::<js_sys::Function>();
        target.add_event_listener_with_callback(event_type, &callback)?;
        listeners.push(Listener {
            target,
            event_type: event_type.to_string(),
            callback,
            options: None,
            _closure: Box::new(closure),
        });
        Ok(())
    }

    fn reg_wheel(
        listeners: &mut Vec<Listener>,
        target: web_sys::EventTarget,
        event_type: &str,
        closure: Closure<dyn FnMut(WheelEvent)>,
        options: web_sys::AddEventListenerOptions,
    ) -> Result<(), JsValue> {
        let callback = closure.as_ref().clone().unchecked_into::<js_sys::Function>();
        target.add_event_listener_with_callback_and_add_event_listener_options(
            event_type,
            &callback,
            &options,
        )?;
        listeners.push(Listener {
            target,
            event_type: event_type.to_string(),
            callback,
            options: Some(options),
            _closure: Box::new(closure),
        });
        Ok(())
    }

    fn reg_clipboard(
        listeners: &mut Vec<Listener>,
        target: web_sys::EventTarget,
        event_type: &str,
        closure: Closure<dyn FnMut(ClipboardEvent)>,
    ) -> Result<(), JsValue> {
        let callback = closure.as_ref().clone().unchecked_into::<js_sys::Function>();
        target.add_event_listener_with_callback(event_type, &callback)?;
        listeners.push(Listener {
            target,
            event_type: event_type.to_string(),
            callback,
            options: None,
            _closure: Box::new(closure),
        });
        Ok(())
    }

    fn reg_generic(
        listeners: &mut Vec<Listener>,
        target: web_sys::EventTarget,
        event_type: &str,
        closure: Closure<dyn FnMut(Event)>,
    ) -> Result<(), JsValue> {
        let callback = closure.as_ref().clone().unchecked_into::<js_sys::Function>();
        target.add_event_listener_with_callback(event_type, &callback)?;
        listeners.push(Listener {
            target,
            event_type: event_type.to_string(),
            callback,
            options: None,
            _closure: Box::new(closure),
        });
        Ok(())
    }

    pub fn new(canvas_id: &str) -> Result<Self, JsValue> {
        let app = Rc::new(RefCell::new(StonepenApp::new(canvas_id)?));
        let ui = Rc::new(WebUi::new(canvas_id)?);

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("canvas not found"))?;
        let canvas_html = canvas
            .clone()
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("element not HtmlCanvasElement"))?;
        let canvas_et: web_sys::EventTarget = canvas
            .dyn_into::<web_sys::EventTarget>()
            .map_err(|_| JsValue::from_str("canvas not event target"))?;

        let mut listeners = Vec::new();

        // -----------------------------------------------------------------------
        // Pointer events
        // -----------------------------------------------------------------------

        let on_pointer_down = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            let canvas_html = canvas_html.clone();
            Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
                e.prevent_default();
                ui.focus_canvas();
                let gesture_started = {
                    let mut a = app.borrow_mut();
                    a.on_pointer_down(&e)
                };
                if gesture_started {
                    let _ = canvas_html.set_pointer_capture(e.pointer_id());
                }
            })
        };
        Self::reg_pointer(&mut listeners, canvas_et.clone(), "pointerdown", on_pointer_down)?;

        let on_pointer_move = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
                e.prevent_default();
                app.borrow_mut().on_pointer_move(&e);
            })
        };
        Self::reg_pointer(&mut listeners, canvas_et.clone(), "pointermove", on_pointer_move)?;

        let on_pointer_up = {
            let app = Rc::clone(&app);
            let canvas_html = canvas_html.clone();
            Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
                e.prevent_default();
                app.borrow_mut().on_pointer_up(&e);
                let _ = canvas_html.release_pointer_capture(e.pointer_id());
            })
        };
        Self::reg_pointer(&mut listeners, canvas_et.clone(), "pointerup", on_pointer_up)?;

        let on_pointer_cancel = {
            let app = Rc::clone(&app);
            let canvas_html = canvas_html.clone();
            Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
                app.borrow_mut().on_pointer_cancel(&e);
                let _ = canvas_html.release_pointer_capture(e.pointer_id());
            })
        };
        Self::reg_pointer(&mut listeners, canvas_et.clone(), "pointercancel", on_pointer_cancel)?;

        let on_lost_pointer_capture = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
                let active_ptr_id = {
                    let a = app.borrow();
                    match &a.input {
                        InputState::Drawing { ptr_id, .. } => Some(ptr_id),
                        InputState::Lassoing { ptr_id, .. } => Some(ptr_id),
                        InputState::Erasing { ptr_id, .. } => Some(ptr_id),
                        InputState::Panning { ptr_id, .. } => Some(ptr_id),
                        InputState::MovingSel { ptr_id, .. } => Some(ptr_id),
                        InputState::ScalingSel { ptr_id, .. } => Some(ptr_id),
                        InputState::RotatingSel { ptr_id, .. } => Some(ptr_id),
                        InputState::MarqueeSelecting { ptr_id, .. } => Some(ptr_id),
                        InputState::Idle => None,
                    }
                };
                if let Some(pid) = active_ptr_id {
                    if *pid == e.pointer_id() {
                        app.borrow_mut().on_pointer_cancel(&e);
                    }
                }
            })
        };
        Self::reg_pointer(
            &mut listeners,
            canvas_et.clone(),
            "lostpointercapture",
            on_lost_pointer_capture,
        )?;

        // -----------------------------------------------------------------------
        // Keyboard events
        // -----------------------------------------------------------------------

        fn is_editing_target(target: Option<web_sys::EventTarget>) -> bool {
            if let Some(t) = target {
                if let Ok(el) = t.dyn_into::<Element>() {
                    let tag = el.tag_name().to_uppercase();
                    if tag == "INPUT" || tag == "TEXTAREA" || tag == "SELECT" {
                        return true;
                    }
                    if let Some(attr) = el.get_attribute("contenteditable") {
                        if attr != "false" {
                            return true;
                        }
                    }
                }
            }
            false
        }

        let on_keydown = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
                let capturing = app.borrow().is_capturing();
                if capturing {
                    e.prevent_default();
                    let mut a = app.borrow_mut();
                    a.on_key_down(&e);
                    let conflict = a.last_conflict.take();
                    drop(a);
                    if let Some(other_cmd) = conflict {
                        ui.show_conflict_alert(other_cmd);
                    }
                    let a = app.borrow();
                    ui.sync_capture_overlay(&a);
                    if !a.is_capturing() && ui.is_settings_open() {
                        drop(a);
                        let a = app.borrow();
                        ui.render_shortcuts(&a);
                    }
                    return;
                }
                if ui.is_settings_open() {
                    return;
                }
                if is_editing_target(e.target()) {
                    return;
                }
                app.borrow_mut().on_key_down(&e);
            })
        };
        Self::reg_keyboard(&mut listeners, window.clone().into(), "keydown", on_keydown)?;

        let on_keyup = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
                app.borrow_mut().on_key_up(&e);
            })
        };
        Self::reg_keyboard(&mut listeners, window.clone().into(), "keyup", on_keyup)?;

        let on_blur = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
                app.borrow_mut().on_blur();
                ui.sync_capture_overlay(&app.borrow());
            })
        };
        Self::reg_generic(&mut listeners, window.clone().into(), "blur", on_blur)?;

        let on_visibility = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if doc.hidden() {
                        app.borrow_mut().on_blur();
                        ui.sync_capture_overlay(&app.borrow());
                    }
                }
            })
        };
        Self::reg_generic(&mut listeners, document.clone().into(), "visibilitychange", on_visibility)?;

        // -----------------------------------------------------------------------
        // Toolbar: tool buttons
        // -----------------------------------------------------------------------

        let tool_names = [
            "pen",
            "pencil",
            "highlighter",
            "eraser",
            "lasso",
            "pan",
            "select",
        ];
        for tool_name in tool_names {
            let app = Rc::clone(&app);
            let tool_name_owned = tool_name.to_string();
            let cb = Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
                app.borrow_mut().set_tool(&tool_name_owned);
            });
            let btn_id = format!("btn-{}", tool_name);
            if let Some(el) = document.get_element_by_id(&btn_id) {
                let target = el.dyn_into::<web_sys::EventTarget>()?;
                Self::reg_generic(&mut listeners, target, "click", cb)?;
            }
        }

        // -----------------------------------------------------------------------
        // Action buttons
        // -----------------------------------------------------------------------

        let mut reg_simple_btn = |id: &str, action: Rc<dyn Fn() + 'static>, listeners_ref: &mut Vec<Listener>| -> Result<(), JsValue> {
            if let Some(el) = document.get_element_by_id(id) {
                let target = el.dyn_into::<web_sys::EventTarget>()?;
                let act = Rc::clone(&action);
                let cb = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                    act();
                });
                Self::reg_generic(listeners_ref, target, "click", cb)?;
            }
            Ok(())
        };

        reg_simple_btn("btn-undo", Rc::new({
            let app = Rc::clone(&app);
            move || app.borrow_mut().action_undo()
        }), &mut listeners)?;

        reg_simple_btn("btn-redo", Rc::new({
            let app = Rc::clone(&app);
            move || app.borrow_mut().action_redo()
        }), &mut listeners)?;

        reg_simple_btn("btn-clear", Rc::new({
            let app = Rc::clone(&app);
            move || app.borrow_mut().action_clear()
        }), &mut listeners)?;

        reg_simple_btn("btn-save", Rc::new({
            let app = Rc::clone(&app);
            move || app.borrow_mut().action_save()
        }), &mut listeners)?;

        reg_simple_btn("btn-export-svg", Rc::new({
            let app = Rc::clone(&app);
            move || app.borrow_mut().action_export_svg()
        }), &mut listeners)?;

        reg_simple_btn("btn-export-png", Rc::new({
            let app = Rc::clone(&app);
            move || app.borrow_mut().action_export_png()
        }), &mut listeners)?;

        // -----------------------------------------------------------------------
        // Settings button
        // -----------------------------------------------------------------------

        if let Some(el) = document.get_element_by_id("btn-settings") {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                let a = app.borrow();
                ui.render_shortcuts(&a);
                drop(a);
                ui.open_settings();
            });
            Self::reg_generic(&mut listeners, target, "click", cb)?;
        }

        let close_settings_rc: Rc<dyn Fn() + 'static> = Rc::new({
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            move || {
                app.borrow_mut().cancel_capture();
                ui.sync_capture_overlay(&app.borrow());
                ui.close_settings();
            }
        });

        if let Some(el) = document.get_element_by_id("btn-settings-close") {
            let f = Rc::clone(&close_settings_rc);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                f();
            });
            Self::reg_generic(&mut listeners, target, "click", cb)?;
        }

        if let Some(el) = document.get_element_by_id("btn-settings-close-footer") {
            let f = Rc::clone(&close_settings_rc);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                f();
            });
            Self::reg_generic(&mut listeners, target, "click", cb)?;
        }

        if let Some(el) = document.get_element_by_id("btn-settings-reset") {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                let confirmed = web_sys::window()
                    .and_then(|w| {
                        w.confirm_with_message(
                            "Are you sure you want to reset all keyboard shortcuts to defaults?",
                        )
                        .ok()
                    })
                    .unwrap_or(false);
                if confirmed {
                    app.borrow_mut().reset_shortcuts_to_defaults();
                    let a = app.borrow();
                    ui.render_shortcuts(&a);
                }
            });
            Self::reg_generic(&mut listeners, target, "click", cb)?;
        }

        // -----------------------------------------------------------------------
        // Shortcuts table — delegated click handler
        // -----------------------------------------------------------------------

        if let Some(container) = document.get_element_by_id("shortcuts-table-container") {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            let target = container.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
                let target = match e.target() {
                    Some(t) => t,
                    None => return,
                };
                let el = match target.dyn_into::<Element>() {
                    Ok(el) => el,
                    Err(_) => return,
                };
                if el.class_list().contains("shortcut-badge-remove") {
                    if let (Some(cmd_id), Some(idx_str)) =
                        (el.get_attribute("data-cmd"), el.get_attribute("data-idx"))
                    {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            e.stop_propagation();
                            app.borrow_mut().remove_shortcut_binding(&cmd_id, idx);
                            let a = app.borrow();
                            ui.render_shortcuts(&a);
                        }
                    }
                    return;
                }
                if el.class_list().contains("add-binding-btn") {
                    if let Some(cmd_id) = el.get_attribute("data-cmd") {
                        app.borrow_mut().start_capture(&cmd_id);
                        let a = app.borrow();
                        ui.sync_capture_overlay(&a);
                    }
                }
            });
            Self::reg_generic(&mut listeners, target, "click", cb)?;
        }

        // -----------------------------------------------------------------------
        // Load button — trigger hidden file input
        // -----------------------------------------------------------------------

        if let Some(el) = document.get_element_by_id("btn-load") {
            let ui = Rc::clone(&ui);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                ui.trigger_load_input_click();
            });
            Self::reg_generic(&mut listeners, target, "click", cb)?;
        }

        // -----------------------------------------------------------------------
        // File input change — FileReader
        // -----------------------------------------------------------------------

        if let Some(el) = document.get_element_by_id("load-input") {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
                let input_el = match ui.get_element("load-input") {
                    Some(el) => el,
                    None => return,
                };
                let input = match input_el.dyn_into::<HtmlInputElement>() {
                    Ok(i) => i,
                    Err(_) => return,
                };
                let files = match input.files() {
                    Some(f) => f,
                    None => return,
                };
                let file = match files.get(0) {
                    Some(f) => f,
                    None => return,
                };
                let reader = match FileReader::new() {
                    Ok(r) => r,
                    Err(_) => return,
                };
                input.set_value("");
                let app_c = Rc::clone(&app);
                let onload = Closure::once_into_js(move |ev: ProgressEvent| {
                    let target = match ev.target() {
                        Some(t) => t,
                        None => return,
                    };
                    let reader = match target.dyn_into::<FileReader>() {
                        Ok(r) => r,
                        Err(_) => return,
                    };
                    let result = match reader.result() {
                        Ok(v) => v,
                        Err(_) => return,
                    };
                    let json = result.as_string().unwrap_or_default();
                    app_c.borrow_mut().action_load(&json);
                })
                .unchecked_into::<js_sys::Function>();
                reader.set_onload(Some(&onload));
                let _ = reader.read_as_text(&file);
            });
            Self::reg_generic(&mut listeners, target, "change", cb)?;
        }

        // -----------------------------------------------------------------------
        // Brush controls
        // -----------------------------------------------------------------------

        if let Some(el) = document.get_element_by_id("width-slider") {
            let app = Rc::clone(&app);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
                let input = match e
                    .target()
                    .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
                {
                    Some(i) => i,
                    None => return,
                };
                if let Ok(w) = input.value().parse::<f32>() {
                    app.borrow_mut().set_brush_width(w);
                }
            });
            Self::reg_generic(&mut listeners, target, "input", cb)?;
        }

        if let Some(el) = document.get_element_by_id("color-picker") {
            let app = Rc::clone(&app);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
                let input = match e
                    .target()
                    .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
                {
                    Some(i) => i,
                    None => return,
                };
                let hex = input.value();
                if hex.len() >= 7 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[1..3], 16),
                        u8::from_str_radix(&hex[3..5], 16),
                        u8::from_str_radix(&hex[5..7], 16),
                    ) {
                        app.borrow_mut().set_brush_color(r, g, b);
                    }
                }
            });
            Self::reg_generic(&mut listeners, target, "input", cb)?;
        }

        // -----------------------------------------------------------------------
        // Paste
        // -----------------------------------------------------------------------

        let on_paste = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(ClipboardEvent)>::new(move |e: ClipboardEvent| {
                if is_editing_target(e.target()) {
                    return;
                }
                let clipboard_data = match e.clipboard_data() {
                    Some(d) => d,
                    None => return,
                };
                let items = clipboard_data.items();
                let mut has_image = false;
                let mut image_item = None;
                for i in 0..items.length() {
                    if let Some(item) = items.get(i) {
                        if item.kind() == "file" && item.type_().starts_with("image/") {
                            has_image = true;
                            image_item = Some(item);
                            break;
                        }
                    }
                }
                if has_image {
                    let item = image_item.unwrap();
                    let file = match item.get_as_file() {
                        Ok(Some(f)) => f,
                        _ => return,
                    };
                    let mime = file.type_();
                    let reader = match FileReader::new() {
                        Ok(r) => r,
                        Err(_) => return,
                    };
                    let object_url = match web_sys::Url::create_object_url_with_blob(&file) {
                        Ok(url) => url,
                        Err(_) => return,
                    };
                    let revoked = Rc::new(std::cell::Cell::new(false));
                    let object_url_clone = object_url.clone();
                    let object_url_clone2 = object_url.clone();
                    let revoked_clone = Rc::clone(&revoked);
                    let revoke_fn = {
                        let url_to_revoke = object_url_clone.clone();
                        move || {
                            if !revoked_clone.get() {
                                revoked_clone.set(true);
                                let _ = web_sys::Url::revoke_object_url(&url_to_revoke);
                            }
                        }
                    };
                    let app_c = Rc::clone(&app);
                    let revoke_fn_onload = revoke_fn.clone();
                    let onload = {
                        let mime_c = mime.clone();
                        Closure::once_into_js(move |ev: ProgressEvent| {
                            let target = match ev.target() {
                                Some(t) => t,
                                None => {
                                    revoke_fn_onload();
                                    return;
                                }
                            };
                            let reader = match target.dyn_into::<FileReader>() {
                                Ok(r) => r,
                                Err(_) => {
                                    revoke_fn_onload();
                                    return;
                                }
                            };
                            let result = match reader.result() {
                                Ok(v) => v,
                                Err(_) => {
                                    revoke_fn_onload();
                                    return;
                                }
                            };
                            let array_buf = match result.dyn_into::<js_sys::ArrayBuffer>() {
                                Ok(ab) => ab,
                                Err(_) => {
                                    revoke_fn_onload();
                                    return;
                                }
                            };
                            let bytes = js_sys::Uint8Array::new(&array_buf).to_vec();
                            let img = match web_sys::HtmlImageElement::new() {
                                Ok(i) => i,
                                Err(_) => {
                                    revoke_fn_onload();
                                    return;
                                }
                            };
                            let app_cc = Rc::clone(&app_c);
                            let bytes_rc = Rc::new(bytes);
                            let img_onload = {
                                let img_ref = img.clone();
                                let bytes_r = Rc::clone(&bytes_rc);
                                let revoke_fn_img_success = revoke_fn_onload.clone();
                                Closure::once_into_js(move || {
                                    let w = img_ref.natural_width();
                                    let h = img_ref.natural_height();
                                    revoke_fn_img_success();
                                    if w > 0 && h > 0 {
                                        app_cc.borrow_mut().paste_image(&bytes_r, &mime_c, w, h);
                                    }
                                })
                                .unchecked_into::<js_sys::Function>()
                            };
                            img.set_onload(Some(&img_onload));
                            let revoke_fn_img_error = revoke_fn_onload.clone();
                            let img_onerror = Closure::once_into_js(move || {
                                revoke_fn_img_error();
                            })
                            .unchecked_into::<js_sys::Function>();
                            img.set_onerror(Some(&img_onerror));
                            img.set_src(&object_url_clone2);
                        })
                        .unchecked_into::<js_sys::Function>()
                    };
                    reader.set_onload(Some(&onload));
                    let revoke_fn_reader_error = revoke_fn.clone();
                    let reader_onerror = Closure::once_into_js(move |_e: ProgressEvent| {
                        revoke_fn_reader_error();
                    })
                    .unchecked_into::<js_sys::Function>();
                    reader.set_onerror(Some(&reader_onerror));
                    let revoke_fn_reader_abort = revoke_fn.clone();
                    let reader_onabort = Closure::once_into_js(move |_e: ProgressEvent| {
                        revoke_fn_reader_abort();
                    })
                    .unchecked_into::<js_sys::Function>();
                    reader.set_onabort(Some(&reader_onabort));
                    let _ = reader.read_as_array_buffer(&file);
                    e.prevent_default();
                } else if app.borrow().clipboard.is_some() {
                    e.prevent_default();
                    app.borrow_mut().dispatch_command(Command::Paste);
                }
            })
        };
        Self::reg_clipboard(&mut listeners, window.clone().into(), "paste", on_paste)?;

        // -----------------------------------------------------------------------
        // Selection bar
        // -----------------------------------------------------------------------

        let actions = [
            ("btn-sel-bring-forward", Command::BringForward),
            ("btn-sel-send-backward", Command::SendBackward),
            ("btn-sel-bring-to-front", Command::BringToFront),
            ("btn-sel-send-to-back", Command::SendToBack),
            ("btn-sel-copy", Command::Copy),
            ("btn-sel-cut", Command::Cut),
            ("btn-sel-duplicate", Command::DuplicateSelection),
            ("btn-sel-delete", Command::DeleteSelection),
        ];

        for &(id, cmd) in &actions {
            let app = Rc::clone(&app);
            let cb = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                app.borrow_mut().dispatch_command(cmd);
            });
            if let Some(el) = document.get_element_by_id(id) {
                let target = el.dyn_into::<web_sys::EventTarget>()?;
                Self::reg_generic(&mut listeners, target, "click", cb)?;
            }
        }

        if let Some(el) = document.get_element_by_id("sel-width-slider") {
            let app = Rc::clone(&app);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
                if let Some(target) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                    if let Ok(w) = target.value().parse::<f32>() {
                        app.borrow_mut().set_selection_width_preview(w);
                    }
                }
            });
            Self::reg_generic(&mut listeners, target, "input", cb)?;
        }

        let reg_sel_width_commit = |el: Element, app: Rc<RefCell<StonepenApp>>, event_type: &str, listeners_ref: &mut Vec<Listener>| -> Result<(), JsValue> {
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new({
                let app = Rc::clone(&app);
                move |_: Event| {
                    app.borrow_mut().commit_style_preview();
                }
            });
            let callback = cb.as_ref().clone().unchecked_into::<js_sys::Function>();
            target.add_event_listener_with_callback(event_type, &callback)?;
            listeners_ref.push(Listener {
                target,
                event_type: event_type.to_string(),
                callback,
                options: None,
                _closure: Box::new(cb),
            });
            Ok(())
        };

        if let Some(el) = document.get_element_by_id("sel-width-slider") {
            reg_sel_width_commit(el.clone(), Rc::clone(&app), "change", &mut listeners)?;
            reg_sel_width_commit(el.clone(), Rc::clone(&app), "blur", &mut listeners)?;
            reg_sel_width_commit(el.clone(), Rc::clone(&app), "pointerup", &mut listeners)?;
        }

        if let Some(el) = document.get_element_by_id("sel-color-picker") {
            let app = Rc::clone(&app);
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new(move |e: Event| {
                if let Some(target) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                    let hex = target.value();
                    if hex.len() >= 7 {
                        if let (Ok(r), Ok(g), Ok(b)) = (
                            u8::from_str_radix(&hex[1..3], 16),
                            u8::from_str_radix(&hex[3..5], 16),
                            u8::from_str_radix(&hex[5..7], 16),
                        ) {
                            app.borrow_mut().set_selection_color_preview(r, g, b);
                        }
                    }
                }
            });
            Self::reg_generic(&mut listeners, target, "input", cb)?;
        }

        let reg_sel_color_commit = |el: Element, app: Rc<RefCell<StonepenApp>>, event_type: &str, listeners_ref: &mut Vec<Listener>| -> Result<(), JsValue> {
            let target = el.dyn_into::<web_sys::EventTarget>()?;
            let cb = Closure::<dyn FnMut(Event)>::new({
                let app = Rc::clone(&app);
                move |_: Event| {
                    app.borrow_mut().commit_style_preview();
                }
            });
            let callback = cb.as_ref().clone().unchecked_into::<js_sys::Function>();
            target.add_event_listener_with_callback(event_type, &callback)?;
            listeners_ref.push(Listener {
                target,
                event_type: event_type.to_string(),
                callback,
                options: None,
                _closure: Box::new(cb),
            });
            Ok(())
        };

        if let Some(el) = document.get_element_by_id("sel-color-picker") {
            reg_sel_color_commit(el.clone(), Rc::clone(&app), "change", &mut listeners)?;
            reg_sel_color_commit(el.clone(), Rc::clone(&app), "blur", &mut listeners)?;
            reg_sel_color_commit(el.clone(), Rc::clone(&app), "pointerup", &mut listeners)?;
        }

        // -----------------------------------------------------------------------
        // Wheel events (non-passive)
        // -----------------------------------------------------------------------

        let on_wheel = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(WheelEvent)>::new(move |e: WheelEvent| {
                e.prevent_default();
                app.borrow_mut().on_wheel(&e);
            })
        };
        let opts = AddEventListenerOptions::new();
        opts.set_passive(false);
        Self::reg_wheel(&mut listeners, canvas_et.clone(), "wheel", on_wheel, opts)?;

        // -----------------------------------------------------------------------
        // ResizeObserver
        // -----------------------------------------------------------------------

        let resize_cb: Closure<dyn FnMut(js_sys::Array)> = {
            let app = Rc::clone(&app);
            Closure::new(move |_entries: js_sys::Array| {
                app.borrow_mut().resize();
            })
        };
        let resize_observer = ResizeObserver::new(resize_cb.as_ref().unchecked_ref())?;
        if let Some(canvas_el) = document.get_element_by_id(canvas_id) {
            resize_observer.observe(&canvas_el);
        }

        // -----------------------------------------------------------------------
        // Initial UI Synchronization
        // -----------------------------------------------------------------------

        {
            let a = app.borrow();
            let tool_name = match a.session.active_tool {
                Tool::Pen => "pen",
                Tool::Pencil => "pencil",
                Tool::Highlighter => "highlighter",
                Tool::StrokeEraser => "eraser",
                Tool::Lasso => "lasso",
                Tool::Select => "select",
                Tool::Pan => "pan",
            };
            ui.sync_tool_buttons(tool_name);
            ui.sync_brush_controls(&a.session.active_brush);
            ui.update_status(&a);
            ui.sync_selection_bar(&a);
            a.redraw();
        }

        Ok(Self {
            app,
            ui,
            listeners,
            resize_observer: Some(resize_observer),
            _resize_cb: Some(resize_cb),
        })
    }

    pub fn destroy(&mut self) {
        for listener in self.listeners.drain(..) {
            listener.remove();
        }
        if let Some(ref ro) = self.resize_observer.take() {
            ro.disconnect();
        }
        self._resize_cb.take();
        self.app.borrow_mut().reset_transient_input();
    }
}

impl Drop for WebRuntime {
    fn drop(&mut self) {
        self.destroy();
    }
}
