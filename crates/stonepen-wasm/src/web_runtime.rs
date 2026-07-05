/// WebRuntime — browser event lifecycle owner.
///
/// Owns:
/// - The StonepenApp (via Rc<RefCell<>>)
/// - The WebUi
/// - All registered event closures (must remain alive)
/// - The ResizeObserver handle
///
/// Created by `start_stonepen`. Intentionally leaked for page lifetime.
use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{
    AddEventListenerOptions, ClipboardEvent, Element, Event, FileReader, HtmlInputElement,
    KeyboardEvent, PointerEvent, ProgressEvent, ResizeObserver, WheelEvent,
};

use crate::app::StonepenApp;
use crate::web_ui::WebUi;
use stonepen_core::session::Tool;
use stonepen_core::shortcuts::Command;

/// Shared handle to the application. All event closures hold a clone.
type AppHandle = Rc<RefCell<StonepenApp>>;

/// All the closures that must survive for the lifetime of the page.
/// The closures are stored here so they are never dropped.
#[allow(dead_code)]
pub struct WebRuntime {
    // Pointer events on canvas
    _on_pointer_down: Closure<dyn FnMut(PointerEvent)>,
    _on_pointer_move: Closure<dyn FnMut(PointerEvent)>,
    _on_pointer_up: Closure<dyn FnMut(PointerEvent)>,
    _on_pointer_cancel: Closure<dyn FnMut(PointerEvent)>,
    _on_wheel: Closure<dyn FnMut(WheelEvent)>,
    // Keyboard / window events
    _on_keydown: Closure<dyn FnMut(KeyboardEvent)>,
    _on_keyup: Closure<dyn FnMut(KeyboardEvent)>,
    _on_blur: Closure<dyn FnMut(Event)>,
    _on_visibility: Closure<dyn FnMut(Event)>,
    // Toolbar / action buttons
    _tool_btns: Vec<Closure<dyn FnMut(Event)>>,
    _btn_undo: Closure<dyn FnMut(Event)>,
    _btn_redo: Closure<dyn FnMut(Event)>,
    _btn_clear: Closure<dyn FnMut(Event)>,
    _btn_save: Closure<dyn FnMut(Event)>,
    _btn_load: Closure<dyn FnMut(Event)>,
    _btn_export_svg: Closure<dyn FnMut(Event)>,
    _btn_export_png: Closure<dyn FnMut(Event)>,
    _btn_settings: Closure<dyn FnMut(Event)>,
    _btn_settings_close: Closure<dyn FnMut(Event)>,
    _btn_settings_close_footer: Closure<dyn FnMut(Event)>,
    _btn_settings_reset: Closure<dyn FnMut(Event)>,
    // Brush controls
    _width_slider: Closure<dyn FnMut(Event)>,
    _color_picker: Closure<dyn FnMut(Event)>,
    // File load input
    _load_input_change: Closure<dyn FnMut(Event)>,
    // Paste
    _on_paste: Closure<dyn FnMut(ClipboardEvent)>,
    // Resize observer
    _resize_observer: ResizeObserver,
    _resize_cb: Closure<dyn FnMut(js_sys::Array)>,
    // Shortcut table event delegation
    _shortcuts_table_click: Closure<dyn FnMut(Event)>,
}

impl WebRuntime {
    pub fn new(canvas_id: &str) -> Result<Self, JsValue> {
        let app = Rc::new(RefCell::new(StonepenApp::new(canvas_id)?));
        let ui = Rc::new(WebUi::new()?);

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("canvas not found"))?;
        let canvas_et: web_sys::EventTarget = canvas
            .dyn_into::<web_sys::EventTarget>()
            .map_err(|_| JsValue::from_str("canvas not event target"))?;

        // -----------------------------------------------------------------------
        // Pointer events
        // -----------------------------------------------------------------------

        let on_pointer_down = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
                e.prevent_default();
                ui.focus_canvas();
                {
                    let mut a = app.borrow_mut();
                    a.on_pointer_down(&e);
                }
            })
        };
        canvas_et.add_event_listener_with_callback(
            "pointerdown",
            on_pointer_down.as_ref().unchecked_ref(),
        )?;

        let on_pointer_move = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
                e.prevent_default();
                app.borrow_mut().on_pointer_move(&e);
            })
        };
        canvas_et.add_event_listener_with_callback(
            "pointermove",
            on_pointer_move.as_ref().unchecked_ref(),
        )?;

        let on_pointer_up = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
                e.prevent_default();
                app.borrow_mut().on_pointer_up(&e);
            })
        };
        canvas_et.add_event_listener_with_callback(
            "pointerup",
            on_pointer_up.as_ref().unchecked_ref(),
        )?;

        let on_pointer_cancel = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
                app.borrow_mut().on_pointer_cancel(&e);
            })
        };
        canvas_et.add_event_listener_with_callback(
            "pointercancel",
            on_pointer_cancel.as_ref().unchecked_ref(),
        )?;

        let on_wheel = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(WheelEvent)>::new(move |e: WheelEvent| {
                e.prevent_default();
                app.borrow_mut().on_wheel(&e);
            })
        };
        // Non-passive wheel listener
        let opts = AddEventListenerOptions::new();
        opts.set_passive(false);
        canvas_et.add_event_listener_with_callback_and_add_event_listener_options(
            "wheel",
            on_wheel.as_ref().unchecked_ref(),
            &opts,
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
                    // Always let capture consume the key
                    e.prevent_default();
                    let mut a = app.borrow_mut();
                    a.on_key_down(&e);
                    // Check for conflict after key down
                    let conflict = a.last_conflict.take();
                    drop(a);
                    if let Some(other_cmd) = conflict {
                        ui.show_conflict_alert(other_cmd);
                    }
                    // Sync capture overlay after state change
                    let a = app.borrow();
                    ui.sync_capture_overlay(&a);
                    if !a.is_capturing() && ui.is_settings_open() {
                        // Re-render shortcuts after capture finishes
                        drop(a);
                        let a = app.borrow();
                        ui.render_shortcuts(&a);
                    }
                    return;
                }
                // If settings modal is open, suppress normal shortcuts
                if ui.is_settings_open() {
                    return;
                }
                // If target is an editable element, pass through
                if is_editing_target(e.target()) {
                    return;
                }
                app.borrow_mut().on_key_down(&e);
            })
        };
        window.add_event_listener_with_callback("keydown", on_keydown.as_ref().unchecked_ref())?;

        let on_keyup = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
                // keyup always forwarded — never filtered
                app.borrow_mut().on_key_up(&e);
            })
        };
        window.add_event_listener_with_callback("keyup", on_keyup.as_ref().unchecked_ref())?;

        let on_blur = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
                {
                    app.borrow_mut().on_blur();
                }
                ui.sync_capture_overlay(&app.borrow());
            })
        };
        window.add_event_listener_with_callback("blur", on_blur.as_ref().unchecked_ref())?;

        let on_visibility = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if doc.hidden() {
                        {
                            app.borrow_mut().on_blur();
                        }
                        ui.sync_capture_overlay(&app.borrow());
                    }
                }
            })
        };
        document.add_event_listener_with_callback(
            "visibilitychange",
            on_visibility.as_ref().unchecked_ref(),
        )?;

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
        let mut tool_btns: Vec<Closure<dyn FnMut(Event)>> = Vec::new();
        for tool_name in tool_names {
            let app = Rc::clone(&app);
            let tool_name_owned = tool_name.to_string();
            let cb = Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
                app.borrow_mut().set_tool(&tool_name_owned);
            });
            let btn_id = format!("btn-{}", tool_name);
            if let Some(el) = document.get_element_by_id(&btn_id) {
                el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            }
            tool_btns.push(cb);
        }

        // -----------------------------------------------------------------------
        // Action buttons
        // -----------------------------------------------------------------------

        macro_rules! simple_btn {
            ($id:expr, $method:ident) => {{
                let app = Rc::clone(&app);
                let cb = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                    app.borrow_mut().$method();
                });
                if let Some(el) = document.get_element_by_id($id) {
                    el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
                }
                cb
            }};
        }

        let btn_undo = simple_btn!("btn-undo", action_undo);
        let btn_redo = simple_btn!("btn-redo", action_redo);
        let btn_clear = simple_btn!("btn-clear", action_clear);
        let btn_save = simple_btn!("btn-save", action_save);
        let btn_export_svg = simple_btn!("btn-export-svg", action_export_svg);
        let btn_export_png = simple_btn!("btn-export-png", action_export_png);

        // -----------------------------------------------------------------------
        // Settings button
        // -----------------------------------------------------------------------

        let btn_settings = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                let a = app.borrow();
                ui.render_shortcuts(&a);
                drop(a);
                ui.open_settings();
            })
        };
        if let Some(el) = document.get_element_by_id("btn-settings") {
            el.add_event_listener_with_callback("click", btn_settings.as_ref().unchecked_ref())?;
        }

        // We need two independent closures for the two close buttons
        let close_settings_rc: Rc<dyn Fn()> = Rc::new({
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            move || {
                {
                    app.borrow_mut().cancel_capture();
                }
                {
                    ui.sync_capture_overlay(&app.borrow());
                }
                ui.close_settings();
            }
        });

        let btn_settings_close = {
            let f = Rc::clone(&close_settings_rc);
            Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                f();
            })
        };
        if let Some(el) = document.get_element_by_id("btn-settings-close") {
            el.add_event_listener_with_callback(
                "click",
                btn_settings_close.as_ref().unchecked_ref(),
            )?;
        }

        let btn_settings_close_footer = {
            let f = Rc::clone(&close_settings_rc);
            Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                f();
            })
        };
        if let Some(el) = document.get_element_by_id("btn-settings-close-footer") {
            el.add_event_listener_with_callback(
                "click",
                btn_settings_close_footer.as_ref().unchecked_ref(),
            )?;
        }

        let btn_settings_reset = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                let confirmed = web_sys::window()
                    .and_then(|w| {
                        w.confirm_with_message(
                            "Are you sure you want to reset all keyboard shortcuts to defaults?",
                        )
                        .ok()
                    })
                    .unwrap_or(false);
                if confirmed {
                    {
                        app.borrow_mut().reset_shortcuts_to_defaults();
                    }
                    let a = app.borrow();
                    ui.render_shortcuts(&a);
                }
            })
        };
        if let Some(el) = document.get_element_by_id("btn-settings-reset") {
            el.add_event_listener_with_callback(
                "click",
                btn_settings_reset.as_ref().unchecked_ref(),
            )?;
        }

        // -----------------------------------------------------------------------
        // Shortcuts table — delegated click handler for remove + add buttons
        // -----------------------------------------------------------------------

        let shortcuts_table_click = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(Event)>::new(move |e: Event| {
                // Walk up the DOM to find which button was clicked
                let target = match e.target() {
                    Some(t) => t,
                    None => return,
                };
                let el = match target.dyn_into::<Element>() {
                    Ok(el) => el,
                    Err(_) => return,
                };
                // Check if it's a remove button
                if el.class_list().contains("shortcut-badge-remove") {
                    if let (Some(cmd_id), Some(idx_str)) =
                        (el.get_attribute("data-cmd"), el.get_attribute("data-idx"))
                    {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            e.stop_propagation();
                            {
                                app.borrow_mut().remove_shortcut_binding(&cmd_id, idx);
                            }
                            let a = app.borrow();
                            ui.render_shortcuts(&a);
                        }
                    }
                    return;
                }
                // Check if it's an add/bind button
                if el.class_list().contains("add-binding-btn") {
                    if let Some(cmd_id) = el.get_attribute("data-cmd") {
                        {
                            app.borrow_mut().start_capture(&cmd_id);
                        }
                        let a = app.borrow();
                        ui.sync_capture_overlay(&a);
                    }
                }
            })
        };
        if let Some(container) = document.get_element_by_id("shortcuts-table-container") {
            container.add_event_listener_with_callback(
                "click",
                shortcuts_table_click.as_ref().unchecked_ref(),
            )?;
        }

        // -----------------------------------------------------------------------
        // Load button — trigger hidden file input
        // -----------------------------------------------------------------------

        let btn_load = {
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                ui.trigger_load_input_click();
            })
        };
        if let Some(el) = document.get_element_by_id("btn-load") {
            el.add_event_listener_with_callback("click", btn_load.as_ref().unchecked_ref())?;
        }

        // -----------------------------------------------------------------------
        // File input change — FileReader
        // -----------------------------------------------------------------------

        let load_input_change = {
            let app = Rc::clone(&app);
            let ui = Rc::clone(&ui);
            Closure::<dyn FnMut(Event)>::new(move |_e: Event| {
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
            })
        };
        if let Some(el) = document.get_element_by_id("load-input") {
            el.add_event_listener_with_callback(
                "change",
                load_input_change.as_ref().unchecked_ref(),
            )?;
        }
        // Clear input value so selecting the same file again triggers a change event.

        // -----------------------------------------------------------------------
        // Brush controls
        // -----------------------------------------------------------------------

        let width_slider = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(Event)>::new(move |e: Event| {
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
            })
        };
        if let Some(el) = document.get_element_by_id("width-slider") {
            el.add_event_listener_with_callback("input", width_slider.as_ref().unchecked_ref())?;
        }

        let color_picker = {
            let app = Rc::clone(&app);
            Closure::<dyn FnMut(Event)>::new(move |e: Event| {
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
            })
        };
        if let Some(el) = document.get_element_by_id("color-picker") {
            el.add_event_listener_with_callback("input", color_picker.as_ref().unchecked_ref())?;
        }

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
        window.add_event_listener_with_callback("paste", on_paste.as_ref().unchecked_ref())?;

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
        // Observe the canvas
        if let Some(canvas_el) = document.get_element_by_id(canvas_id) {
            resize_observer.observe(&canvas_el);
        }

        // -----------------------------------------------------------------------
        // Initial state
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
        }
        app.borrow().redraw();

        Ok(Self {
            _on_pointer_down: on_pointer_down,
            _on_pointer_move: on_pointer_move,
            _on_pointer_up: on_pointer_up,
            _on_pointer_cancel: on_pointer_cancel,
            _on_wheel: on_wheel,
            _on_keydown: on_keydown,
            _on_keyup: on_keyup,
            _on_blur: on_blur,
            _on_visibility: on_visibility,
            _tool_btns: tool_btns,
            _btn_undo: btn_undo,
            _btn_redo: btn_redo,
            _btn_clear: btn_clear,
            _btn_save: btn_save,
            _btn_load: btn_load,
            _btn_export_svg: btn_export_svg,
            _btn_export_png: btn_export_png,
            _btn_settings: btn_settings,
            _btn_settings_close: btn_settings_close,
            _btn_settings_close_footer: btn_settings_close_footer,
            _btn_settings_reset: btn_settings_reset,
            _width_slider: width_slider,
            _color_picker: color_picker,
            _load_input_change: load_input_change,
            _on_paste: on_paste,
            _resize_observer: resize_observer,
            _resize_cb: resize_cb,
            _shortcuts_table_click: shortcuts_table_click,
        })
    }
}
