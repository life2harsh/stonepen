/// WebUi — DOM synchronization layer.
///
/// Owns all DOM element references needed by the Stonepen UI.
/// Reads application state from StonepenApp and writes to DOM.
/// Does not own application logic.
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, Window};

use crate::app::StonepenApp;
use stonepen_core::brush::Brush;
use stonepen_core::shortcuts::Command;

pub struct WebUi {
    window: Window,
    document: Document,
}

impl WebUi {
    pub fn new() -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;
        Ok(Self { window, document })
    }

    // -----------------------------------------------------------------------
    // Brush controls synchronization
    // -----------------------------------------------------------------------

    pub fn sync_brush_controls(&self, brush: &Brush) {
        if let Some(el) = self.document.get_element_by_id("width-slider") {
            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                input.set_value(&brush.base_w.to_string());
            }
        }
        if let Some(el) = self.document.get_element_by_id("color-picker") {
            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                input.set_value(&brush.color.to_hex());
            }
        }
    }

    // -----------------------------------------------------------------------
    // Toolbar synchronization
    // -----------------------------------------------------------------------

    pub fn sync_tool_buttons(&self, active_tool_name: &str) {
        let btns = [
            "pen",
            "pencil",
            "highlighter",
            "eraser",
            "lasso",
            "pan",
            "select",
        ];
        for btn_name in btns {
            let id = format!("btn-{}", btn_name);
            if let Some(el) = self.document.get_element_by_id(&id) {
                if btn_name == active_tool_name {
                    let _ = el.class_list().add_1("active");
                } else {
                    let _ = el.class_list().remove_1("active");
                }
            }
        }
        if let Some(canvas) = self.document.get_element_by_id("ink-canvas") {
            let _ = canvas.set_class_name("");
            match active_tool_name {
                "pan" => {
                    let _ = canvas.class_list().add_1("tool-pan");
                }
                "eraser" => {
                    let _ = canvas.class_list().add_1("tool-eraser");
                }
                "lasso" => {
                    let _ = canvas.class_list().add_1("tool-lasso");
                }
                "select" => {
                    let _ = canvas.class_list().add_1("tool-select");
                }
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------------
    // Cursor
    // -----------------------------------------------------------------------

    pub fn set_cursor(&self, cursor_str: &str) {
        if let Some(canvas) = self.document.get_element_by_id("ink-canvas") {
            if let Ok(html_canvas) = canvas.dyn_into::<web_sys::HtmlCanvasElement>() {
                let _ = html_canvas.style().set_property("cursor", cursor_str);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Status bar
    // -----------------------------------------------------------------------

    pub fn update_status(&self, app: &StonepenApp) {
        use stonepen_core::session::Tool;
        let total: usize = app.session.doc.layers.iter().map(|l| l.items.len()).sum();
        let sel = app.session.doc.runtime.sel_items.len();
        let tool_str = match app.session.active_tool {
            Tool::Pen => "Pen",
            Tool::Pencil => "Pencil",
            Tool::Highlighter => "Highlighter",
            Tool::StrokeEraser => "Eraser",
            Tool::Lasso => "Lasso",
            Tool::Pan => "Pan",
            Tool::Select => "Select",
        };
        let zoom_pct = (app.vp.zoom * 100.0).round() as i32;
        let dirty_str = if app.session.dirty {
            "modified"
        } else {
            "saved"
        };
        let status = format!(
            "items: {total}  selected: {sel}  tool: {tool_str}  zoom: {zoom_pct}%  {dirty_str}"
        );
        if let Some(el) = self.document.get_element_by_id("status-bar") {
            el.set_text_content(Some(&status));
        }
    }

    // -----------------------------------------------------------------------
    // Settings modal
    // -----------------------------------------------------------------------

    pub fn open_settings(&self) {
        if let Some(modal) = self.document.get_element_by_id("settings-modal") {
            let _ = modal.class_list().add_1("show");
        }
    }

    pub fn close_settings(&self) {
        if let Some(modal) = self.document.get_element_by_id("settings-modal") {
            let _ = modal.class_list().remove_1("show");
        }
    }

    pub fn is_settings_open(&self) -> bool {
        if let Some(modal) = self.document.get_element_by_id("settings-modal") {
            return modal.class_list().contains("show");
        }
        false
    }

    // -----------------------------------------------------------------------
    // Shortcut table rendering
    // -----------------------------------------------------------------------

    pub fn render_shortcuts(&self, app: &StonepenApp) {
        let container = match self.document.get_element_by_id("shortcuts-table-container") {
            Some(el) => el,
            None => return,
        };
        container.set_inner_html("");

        let is_mac = app.get_platform_is_mac();

        let all_commands = [
            Command::ToolPen,
            Command::ToolPencil,
            Command::ToolHighlighter,
            Command::ToolEraser,
            Command::ToolLasso,
            Command::ToolSelect,
            Command::ToolPan,
            Command::Undo,
            Command::Redo,
            Command::DeleteSelection,
            Command::DuplicateSelection,
            Command::ClearSelection,
            Command::SelectAll,
            Command::Copy,
            Command::Cut,
            Command::Paste,
            Command::NudgeLeft,
            Command::NudgeRight,
            Command::NudgeUp,
            Command::NudgeDown,
            Command::BringForward,
            Command::SendBackward,
            Command::BringToFront,
            Command::SendToBack,
            Command::HoldPan,
        ];

        struct Group {
            name: &'static str,
            commands: Vec<Command>,
        }
        let mut tools_group = Group {
            name: "Tools",
            commands: Vec::new(),
        };
        let mut history_group = Group {
            name: "History",
            commands: Vec::new(),
        };
        let mut selection_group = Group {
            name: "Selection",
            commands: Vec::new(),
        };
        let mut editing_group = Group {
            name: "Editing",
            commands: Vec::new(),
        };
        let mut navigation_group = Group {
            name: "Navigation",
            commands: Vec::new(),
        };

        for cmd in all_commands {
            match cmd.group() {
                "Tools" => tools_group.commands.push(cmd),
                "History" => history_group.commands.push(cmd),
                "Selection" => selection_group.commands.push(cmd),
                "Editing" => editing_group.commands.push(cmd),
                "Navigation" => navigation_group.commands.push(cmd),
                _ => {}
            }
        }

        let groups = [
            tools_group,
            history_group,
            selection_group,
            editing_group,
            navigation_group,
        ];

        for group in &groups {
            // Group heading
            if let Ok(h4) = self.document.create_element("h4") {
                h4.set_text_content(Some(group.name));
                let _ = h4
                    .dyn_ref::<HtmlElement>()
                    .map(|el| el.style().set_property("margin", "12px 0 6px 0"));
                let _ = h4
                    .dyn_ref::<HtmlElement>()
                    .map(|el| el.style().set_property("color", "var(--title-color)"));
                let _ = container.append_child(&h4);
            }

            for &cmd in &group.commands {
                let chords = app.settings.shortcuts.bindings(cmd);
                if let Ok(row) = self.document.create_element("div") {
                    let _ = row.set_attribute("class", "shortcut-row");
                    let _ = row.set_attribute("data-cmd", cmd.to_id());

                    // Label
                    if let Ok(label_el) = self.document.create_element("div") {
                        let _ = label_el.set_attribute("class", "shortcut-label");
                        label_el.set_text_content(Some(cmd.label()));
                        let _ = row.append_child(&label_el);
                    }

                    // Bindings
                    if let Ok(bindings_div) = self.document.create_element("div") {
                        let _ = bindings_div.set_attribute("class", "shortcut-bindings");
                        for (idx, chord) in chords.iter().enumerate() {
                            if let Ok(badge) = self.document.create_element("span") {
                                let _ = badge.set_attribute("class", "shortcut-badge");
                                let display = chord.to_display_string(is_mac);
                                // We'll put the text + remove btn inside
                                if let Ok(text_node) = self.document.create_element("span") {
                                    text_node.set_text_content(Some(&display));
                                    let _ = badge.append_child(&text_node);
                                }
                                if let Ok(remove_btn) = self.document.create_element("button") {
                                    let _ =
                                        remove_btn.set_attribute("class", "shortcut-badge-remove");
                                    remove_btn.set_inner_html("&times;");
                                    // data-cmd and data-idx so the click handler can look it up
                                    let _ = remove_btn.set_attribute("data-cmd", cmd.to_id());
                                    let _ = remove_btn.set_attribute("data-idx", &idx.to_string());
                                    let _ = badge.append_child(&remove_btn);
                                }
                                let _ = bindings_div.append_child(&badge);
                            }
                        }
                        let _ = row.append_child(&bindings_div);
                    }

                    // Add/Bind button
                    if let Ok(actions_div) = self.document.create_element("div") {
                        let _ = actions_div.set_attribute("class", "shortcut-actions");
                        if let Ok(add_btn) = self.document.create_element("button") {
                            let _ = add_btn.set_attribute("class", "add-binding-btn");
                            let _ = add_btn.set_attribute("data-cmd", cmd.to_id());
                            add_btn.set_text_content(Some(if !chords.is_empty() {
                                "Add..."
                            } else {
                                "Bind..."
                            }));
                            let _ = actions_div.append_child(&add_btn);
                        }
                        let _ = row.append_child(&actions_div);
                    }

                    let _ = container.append_child(&row);
                }
            }
        }

        // Sync capture overlay
        self.sync_capture_overlay(app);
    }

    pub fn sync_capture_overlay(&self, app: &StonepenApp) {
        let overlay = match self.document.get_element_by_id("capture-overlay") {
            Some(el) => el,
            None => return,
        };
        if app.is_capturing() {
            let _ = overlay.class_list().remove_1("hidden");
            if let Some(label_el) = self.document.get_element_by_id("capture-cmd-name") {
                label_el.set_text_content(Some(&app.capturing_label()));
            }
        } else {
            let _ = overlay.class_list().add_1("hidden");
        }
    }

    pub fn show_conflict_alert(&self, other_cmd: Command) {
        let msg = format!("Already used by: {}", other_cmd.label());
        let _ = self.window.alert_with_message(&msg);
    }

    // -----------------------------------------------------------------------
    // Brush controls (read from DOM inputs)
    // -----------------------------------------------------------------------

    pub fn read_brush_width(&self) -> Option<f32> {
        self.document
            .get_element_by_id("width-slider")
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .and_then(|input| input.value().parse::<f32>().ok())
    }

    pub fn read_brush_color_hex(&self) -> Option<String> {
        self.document
            .get_element_by_id("color-picker")
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|input| input.value())
    }

    // -----------------------------------------------------------------------
    // Focus canvas
    // -----------------------------------------------------------------------

    pub fn focus_canvas(&self) {
        if let Some(el) = self.document.get_element_by_id("ink-canvas") {
            if let Ok(html) = el.dyn_into::<HtmlElement>() {
                let _ = html.focus();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Load input (hidden file input)
    // -----------------------------------------------------------------------

    pub fn trigger_load_input_click(&self) {
        if let Some(el) = self.document.get_element_by_id("load-input") {
            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                input.click();
            }
        }
    }

    pub fn clear_load_input_value(&self) {
        if let Some(el) = self.document.get_element_by_id("load-input") {
            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                input.set_value("");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Element queries used by event wiring
    // -----------------------------------------------------------------------

    pub fn get_element(&self, id: &str) -> Option<Element> {
        self.document.get_element_by_id(id)
    }
}
