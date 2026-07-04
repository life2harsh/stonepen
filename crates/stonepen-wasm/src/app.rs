use stonepen_core::brush::Brush;
use stonepen_core::ids::LayerId;
use stonepen_core::ops::{InkOp, InkTx};
use stonepen_core::point::{InkPoint, Point2, PointerKind};
use stonepen_core::session::{InkSession, Tool};
use stonepen_core::stroke::{InkStroke, StrokeBuilder};
use stonepen_core::viewport::Viewport;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent, PointerEvent, WheelEvent};

use crate::canvas::{get_2d_context, get_canvas, sync_canvas_size};
use crate::file_io::{trigger_download, trigger_png_download};
use crate::keyboard::parse_key;
use crate::pointer::{get_inputs, PointerInput};
use crate::render_2d::Renderer;

pub enum InputState {
    Idle,
    Drawing {
        ptr_id: i32,
        builder: StrokeBuilder,
    },
    Erasing {
        ptr_id: i32,
        erased: Vec<(LayerId, InkStroke)>,
    },
    Lassoing {
        ptr_id: i32,
        polygon: Vec<Point2>,
    },
    Panning {
        ptr_id: i32,
        last_sx: f32,
        last_sy: f32,
    },
}

pub struct StonepenApp {
    canvas: HtmlCanvasElement,
    renderer: Renderer,
    session: InkSession,
    vp: Viewport,
    input: InputState,
    dpr: f64,
    preview_pts: Vec<InkPoint>,
    lasso_preview: Vec<Point2>,
}

impl StonepenApp {
    pub fn new(canvas_id: &str) -> Result<Self, JsValue> {
        let canvas = get_canvas(canvas_id)?;
        let ctx = get_2d_context(&canvas)?;
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let dpr = window.device_pixel_ratio();
        let css_w = canvas.client_width() as f32;
        let css_h = canvas.client_height() as f32;
        sync_canvas_size(&canvas, dpr);
        let mut vp = Viewport::new(css_w, css_h);
        vp.dpr = dpr as f32;
        let session = InkSession::new(css_w, css_h);
        let renderer = Renderer::new(ctx);
        Ok(Self {
            canvas,
            renderer,
            session,
            vp,
            input: InputState::Idle,
            dpr,
            preview_pts: Vec::new(),
            lasso_preview: Vec::new(),
        })
    }

    pub fn on_pointer_down(&mut self, e: &PointerEvent) {
        e.prevent_default();
        let pi = PointerInput::from_event(e);
        match &self.session.active_tool {
            Tool::Pen | Tool::Pencil | Tool::Highlighter => {
                let draws = match pi.kind {
                    PointerKind::Pen => true,
                    PointerKind::Mouse => pi.buttons & 1 != 0,
                    _ => false,
                };
                if draws {
                    let brush = self.session.active_brush.clone();
                    let mut builder = StrokeBuilder::new(brush);
                    let pt = pi.to_ink_point(&self.vp);
                    builder.push(pt);
                    self.preview_pts = builder.preview_pts().to_vec();
                    self.input = InputState::Drawing {
                        ptr_id: pi.id,
                        builder,
                    };
                }
            }
            Tool::StrokeEraser => {
                let world = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                let erased = self.erase_at_collect(world, 12.0);
                self.input = InputState::Erasing {
                    ptr_id: pi.id,
                    erased,
                };
            }
            Tool::Lasso => {
                let world = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                self.lasso_preview = vec![world];
                self.input = InputState::Lassoing {
                    ptr_id: pi.id,
                    polygon: vec![world],
                };
            }
            Tool::Pan | Tool::Select => {
                self.input = InputState::Panning {
                    ptr_id: pi.id,
                    last_sx: pi.x,
                    last_sy: pi.y,
                };
            }
        }
        self.redraw();
    }

    pub fn on_pointer_move(&mut self, e: &PointerEvent) {
        e.prevent_default();
        let inputs = get_inputs(e);
        let ptr_id = e.pointer_id();
        let is_erasing =
            matches!(&self.input, InputState::Erasing { ptr_id: id, .. } if *id == ptr_id);
        if is_erasing {
            let pi = &inputs[inputs.len() - 1];
            let world = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
            let mut newly_erased = self.erase_at_collect(world, 12.0);
            if let InputState::Erasing { erased, .. } = &mut self.input {
                erased.append(&mut newly_erased);
            }
            self.redraw();
            return;
        }
        match &mut self.input {
            InputState::Drawing {
                ptr_id: id,
                builder,
            } if *id == ptr_id => {
                for pi in &inputs {
                    let pt = pi.to_ink_point(&self.vp);
                    builder.push(pt);
                }
                self.preview_pts = builder.preview_pts().to_vec();
            }
            InputState::Lassoing {
                ptr_id: id,
                polygon,
            } if *id == ptr_id => {
                let pi = &inputs[inputs.len() - 1];
                let world = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                polygon.push(world);
                self.lasso_preview = polygon.clone();
            }
            InputState::Panning {
                ptr_id: id,
                last_sx,
                last_sy,
            } if *id == ptr_id => {
                let pi = &inputs[inputs.len() - 1];
                let dx = pi.x - *last_sx;
                let dy = pi.y - *last_sy;
                self.vp.pan_by_screen_delta(dx, dy);
                *last_sx = pi.x;
                *last_sy = pi.y;
            }
            _ => return,
        }
        self.redraw();
    }

    pub fn on_pointer_up(&mut self, e: &PointerEvent) {
        e.prevent_default();
        let ptr_id = e.pointer_id();
        let finishing = match &self.input {
            InputState::Drawing { ptr_id: id, .. } => *id == ptr_id,
            InputState::Lassoing { ptr_id: id, .. } => *id == ptr_id,
            InputState::Erasing { ptr_id: id, .. } => *id == ptr_id,
            InputState::Panning { ptr_id: id, .. } => *id == ptr_id,
            InputState::Idle => false,
        };
        if !finishing {
            return;
        }
        let old_state = std::mem::replace(&mut self.input, InputState::Idle);
        match old_state {
            InputState::Drawing { builder, .. } => {
                let now_ms = js_sys::Date::now() as i64;
                if let Some(stroke) = builder.finish(now_ms) {
                    self.session.add_stroke(stroke);
                }
                self.preview_pts.clear();
            }
            InputState::Lassoing { polygon, .. } => {
                self.session.select_lasso(&polygon);
                self.lasso_preview.clear();
            }
            InputState::Erasing { erased, .. } => {
                if !erased.is_empty() {
                    let tx = InkTx::new("erase").push(InkOp::DeleteStrokes { strokes: erased });
                    self.session.undo_redo.push(tx);
                    self.session.rev += 1;
                    self.session.dirty = true;
                }
            }
            InputState::Panning { .. } => {}
            InputState::Idle => {}
        }
        self.update_status();
        self.redraw();
    }

    pub fn on_pointer_cancel(&mut self, e: &PointerEvent) {
        let ptr_id = e.pointer_id();
        let cancel = match &self.input {
            InputState::Drawing { ptr_id: id, .. } => *id == ptr_id,
            InputState::Lassoing { ptr_id: id, .. } => *id == ptr_id,
            InputState::Erasing { ptr_id: id, .. } => *id == ptr_id,
            InputState::Panning { ptr_id: id, .. } => *id == ptr_id,
            InputState::Idle => false,
        };
        if cancel {
            if let InputState::Erasing { erased, .. } =
                std::mem::replace(&mut self.input, InputState::Idle)
            {
                if !erased.is_empty() {
                    let tx = InkTx::new("erase").push(InkOp::DeleteStrokes { strokes: erased });
                    self.session.undo_redo.push(tx);
                    self.session.rev += 1;
                    self.session.dirty = true;
                }
            } else {
                self.input = InputState::Idle;
            }
            self.preview_pts.clear();
            self.lasso_preview.clear();
            self.redraw();
        }
    }

    pub fn on_wheel(&mut self, e: &WheelEvent) {
        e.prevent_default();
        let cx = e.client_x() as f32;
        let cy = e.client_y() as f32;
        let delta = e.delta_y();
        let factor = if delta > 0.0 { 0.9 } else { 1.0 / 0.9 };
        self.vp.zoom_at_screen_pos(Point2::new(cx, cy), factor);
        self.update_status();
        self.redraw();
    }

    pub fn on_key(&mut self, e: &KeyboardEvent) {
        let action = parse_key(e);
        if action.undo || action.redo || action.delete || action.escape {
            e.prevent_default();
        }
        if action.undo {
            self.session.undo();
        } else if action.redo {
            self.session.redo();
        } else if action.delete {
            self.session.delete_sel();
        } else if action.escape {
            self.session.doc.clear_sel();
            self.lasso_preview.clear();
            if matches!(self.input, InputState::Lassoing { .. }) {
                self.input = InputState::Idle;
            }
        }
        self.update_status();
        self.redraw();
    }

    pub fn set_tool(&mut self, tool: &str) {
        self.session.active_tool = match tool {
            "pen" => {
                self.session.active_brush = Brush::default_pen();
                Tool::Pen
            }
            "pencil" => {
                self.session.active_brush = Brush::default_pencil();
                Tool::Pencil
            }
            "highlighter" => {
                self.session.active_brush = Brush::default_highlighter();
                Tool::Highlighter
            }
            "eraser" => Tool::StrokeEraser,
            "lasso" => Tool::Lasso,
            "pan" => Tool::Pan,
            _ => Tool::Pen,
        };
        self.update_status();
    }

    pub fn set_brush_color(&mut self, r: u8, g: u8, b: u8) {
        self.session.active_brush.color.r = r;
        self.session.active_brush.color.g = g;
        self.session.active_brush.color.b = b;
    }

    pub fn set_brush_width(&mut self, w: f32) {
        self.session.active_brush.base_w = w.clamp(0.5, 64.0);
    }

    pub fn action_undo(&mut self) {
        self.session.undo();
        self.update_status();
        self.redraw();
    }

    pub fn action_redo(&mut self) {
        self.session.redo();
        self.update_status();
        self.redraw();
    }

    pub fn action_clear(&mut self) {
        self.session.clear_active_layer();
        self.update_status();
        self.redraw();
    }

    pub fn action_save(&mut self) {
        match self.session.export_json() {
            Ok(json) => {
                let _ = trigger_download("drawing.stonepen.json", &json, "application/json");
                self.session.last_saved_rev = self.session.rev;
                self.session.dirty = false;
                self.update_status();
            }
            Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!("{e}"))),
        }
    }

    pub fn action_load(&mut self, json: &str) {
        match InkSession::import_json(json) {
            Ok(s) => {
                self.session = s;
                self.update_status();
                self.redraw();
            }
            Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!("{e}"))),
        }
    }

    pub fn action_export_svg(&self) {
        match self.session.export_svg() {
            Ok(svg) => {
                let _ = trigger_download("drawing.svg", &svg, "image/svg+xml");
            }
            Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!("{e}"))),
        }
    }

    pub fn action_export_png(&self) {
        let _ = trigger_png_download(&self.canvas, "drawing.png");
    }

    pub fn resize(&mut self) {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        self.dpr = window.device_pixel_ratio();
        sync_canvas_size(&self.canvas, self.dpr);
        let css_w = self.canvas.client_width() as f32;
        let css_h = self.canvas.client_height() as f32;
        self.vp.screen_w = css_w;
        self.vp.screen_h = css_h;
        self.vp.dpr = self.dpr as f32;
        self.redraw();
    }

    pub fn redraw(&self) {
        let canvas_w = self.canvas.width() as f64;
        let canvas_h = self.canvas.height() as f64;
        self.renderer.render(
            &self.session,
            &self.vp,
            &self.preview_pts,
            &self.lasso_preview,
            canvas_w,
            canvas_h,
        );
    }

    fn erase_at_collect(&mut self, pos: Point2, radius: f32) -> Vec<(LayerId, InkStroke)> {
        let candidates = self.session.doc.hit_eraser(pos, radius);
        if candidates.is_empty() {
            return Vec::new();
        }
        let mut to_delete = Vec::new();
        for sid in candidates {
            if let Some(s) = self.session.doc.get_stroke(sid) {
                if stonepen_core::hit::stroke_hit(s, pos, radius) {
                    to_delete.push(sid);
                }
            }
        }
        if to_delete.is_empty() {
            return Vec::new();
        }
        self.session.doc.delete_strokes(&to_delete)
    }

    fn update_status(&self) {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let document = match window.document() {
            Some(d) => d,
            None => return,
        };
        let total: usize = self
            .session
            .doc
            .layers
            .iter()
            .map(|l| l.strokes.len())
            .sum();
        let sel = self.session.doc.runtime.sel_strokes.len();
        let tool_str = match self.session.active_tool {
            Tool::Pen => "Pen",
            Tool::Pencil => "Pencil",
            Tool::Highlighter => "Highlighter",
            Tool::StrokeEraser => "Eraser",
            Tool::Lasso => "Lasso",
            Tool::Pan => "Pan",
            Tool::Select => "Select",
        };
        let zoom_pct = (self.vp.zoom * 100.0).round() as i32;
        let dirty_str = if self.session.dirty {
            "modified"
        } else {
            "saved"
        };
        let status = format!(
            "strokes: {total}  selected: {sel}  tool: {tool_str}  zoom: {zoom_pct}%  {dirty_str}"
        );
        if let Some(el) = document.get_element_by_id("status-bar") {
            el.set_text_content(Some(&status));
        }
    }
}
