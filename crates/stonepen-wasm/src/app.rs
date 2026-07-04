use stonepen_core::bbox::BBox;
use stonepen_core::brush::Brush;
use stonepen_core::ids::{AssetId, ItemId, LayerId};
use stonepen_core::item::{ImageAsset, InkItem};
use stonepen_core::ops::{InkOp, InkTx};
use stonepen_core::point::{InkPoint, Point2, PointerKind};
use stonepen_core::session::{InkSession, Tool};
use stonepen_core::stroke::StrokeBuilder;
use stonepen_core::viewport::Viewport;
use stonepen_core::xform::Xform2D;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent, PointerEvent, WheelEvent};

use crate::canvas::{get_2d_context, get_canvas, sync_canvas_size};
use crate::file_io::{trigger_download, trigger_png_download};
use crate::keyboard::parse_key;
use crate::pointer::{get_inputs, PointerInput};
use crate::render_2d::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelHandle {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelHit {
    None,
    Move,
    Scale(SelHandle),
    Rotate,
}

pub enum InputState {
    Idle,
    Drawing {
        ptr_id: i32,
        builder: StrokeBuilder,
        parent_id: Option<ItemId>,
        parent_xform_inv: Option<Xform2D>,
    },
    Erasing {
        ptr_id: i32,
        erased: Vec<(LayerId, usize, InkItem)>,
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
    MovingSel {
        ptr_id: i32,
        start_world: Point2,
        before: Vec<(ItemId, Xform2D)>,
    },
    ScalingSel {
        ptr_id: i32,
        pivot: Point2,
        start_world: Point2,
        before: Vec<(ItemId, Xform2D)>,
    },
    RotatingSel {
        ptr_id: i32,
        pivot: Point2,
        start_angle: f32,
        before: Vec<(ItemId, Xform2D)>,
    },
    MarqueeSelecting {
        ptr_id: i32,
        start_screen: Point2,
        start_world: Point2,
        curr_screen: Point2,
        curr_world: Point2,
        active: bool,
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
        let pi = PointerInput::from_event(e, &self.canvas);
        match &self.session.active_tool {
            Tool::Pen | Tool::Pencil | Tool::Highlighter => {
                let draws = match pi.kind {
                    PointerKind::Pen => true,
                    PointerKind::Mouse => pi.buttons & 1 != 0,
                    _ => false,
                };
                if draws {
                    let parent_id = self.session.doc.annotation_target_image();
                    let parent_xform_inv = if let Some(pid) = parent_id {
                        if let Some(InkItem::Image(img)) = self.session.doc.get_item(pid) {
                            img.xform.inverse()
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let brush = self.session.active_brush.clone();
                    let mut builder = StrokeBuilder::new(brush);
                    let mut pt = pi.to_ink_point(&self.vp);
                    if let Some(inv) = parent_xform_inv {
                        let lp = inv.apply(Point2::new(pt.x, pt.y));
                        pt.x = lp.x;
                        pt.y = lp.y;
                    }
                    builder.push(pt);
                    self.preview_pts = builder.preview_pts().to_vec();
                    self.input = InputState::Drawing {
                        ptr_id: pi.id,
                        builder,
                        parent_id,
                        parent_xform_inv,
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
            Tool::Pan => {
                self.input = InputState::Panning {
                    ptr_id: pi.id,
                    last_sx: pi.x,
                    last_sy: pi.y,
                };
            }
            Tool::Select => {
                let world_pos = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                let (hit, handle_pivot) = self.selection_hit_test(Point2::new(pi.x, pi.y));
                let start_world = world_pos;

                match hit {
                    SelHit::Rotate => {
                        let roots = self.session.doc.transform_roots();
                        let before = roots
                            .iter()
                            .map(|&id| (id, self.session.doc.get_item(id).unwrap().xform()))
                            .collect();
                        let d = world_pos - handle_pivot;
                        let start_angle = d.y.atan2(d.x);
                        self.update_cursor("grabbing");
                        self.input = InputState::RotatingSel {
                            ptr_id: pi.id,
                            pivot: handle_pivot,
                            start_angle,
                            before,
                        };
                    }
                    SelHit::Scale(_handle) => {
                        let roots = self.session.doc.transform_roots();
                        let before = roots
                            .iter()
                            .map(|&id| (id, self.session.doc.get_item(id).unwrap().xform()))
                            .collect();
                        self.input = InputState::ScalingSel {
                            ptr_id: pi.id,
                            pivot: handle_pivot,
                            start_world,
                            before,
                        };
                    }
                    SelHit::Move => {
                        let roots = self.session.doc.transform_roots();
                        let before = roots
                            .iter()
                            .map(|&id| (id, self.session.doc.get_item(id).unwrap().xform()))
                            .collect();
                        self.update_cursor("grabbing");
                        self.input = InputState::MovingSel {
                            ptr_id: pi.id,
                            start_world,
                            before,
                        };
                    }
                    SelHit::None => {
                        let clicked = self.session.doc.hit_test_item(world_pos, 8.0, self.vp.zoom);
                        if let Some(id) = clicked {
                            if self.session.doc.runtime.sel_items.contains(&id) {
                                let roots = self.session.doc.transform_roots();
                                let before = roots
                                    .iter()
                                    .map(|&id| (id, self.session.doc.get_item(id).unwrap().xform()))
                                    .collect();
                                self.update_cursor("grabbing");
                                self.input = InputState::MovingSel {
                                    ptr_id: pi.id,
                                    start_world,
                                    before,
                                };
                            } else {
                                self.session.doc.clear_sel();
                                self.session.doc.runtime.sel_items.insert(id);
                                let before =
                                    vec![(id, self.session.doc.get_item(id).unwrap().xform())];
                                self.update_cursor("grabbing");
                                self.input = InputState::MovingSel {
                                    ptr_id: pi.id,
                                    start_world,
                                    before,
                                };
                            }
                        } else {
                            self.input = InputState::MarqueeSelecting {
                                ptr_id: pi.id,
                                start_screen: Point2::new(pi.x, pi.y),
                                start_world,
                                curr_screen: Point2::new(pi.x, pi.y),
                                curr_world: world_pos,
                                active: false,
                            };
                        }
                    }
                }
            }
        }
        self.redraw();
    }

    pub fn on_pointer_move(&mut self, e: &PointerEvent) {
        e.prevent_default();
        let inputs = get_inputs(e, &self.canvas);
        let ptr_id = e.pointer_id();
        if matches!(self.input, InputState::Idle) && self.session.active_tool == Tool::Select {
            if !inputs.is_empty() {
                let pi = &inputs[inputs.len() - 1];
                let (hit, _) = self.selection_hit_test(Point2::new(pi.x, pi.y));
                match hit {
                    SelHit::Move => self.update_cursor("move"),
                    SelHit::Rotate => self.update_cursor("grab"),
                    SelHit::Scale(handle) => match handle {
                        SelHandle::TopLeft | SelHandle::BottomRight => {
                            self.update_cursor("nwse-resize")
                        }
                        SelHandle::TopRight | SelHandle::BottomLeft => {
                            self.update_cursor("nesw-resize")
                        }
                        _ => self.update_cursor("default"),
                    },
                    SelHit::None => self.update_cursor("default"),
                }
            }
        }

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
                parent_id: _,
                parent_xform_inv,
            } if *id == ptr_id => {
                for pi in &inputs {
                    let mut pt = pi.to_ink_point(&self.vp);
                    if let Some(inv) = parent_xform_inv {
                        let lp = inv.apply(Point2::new(pt.x, pt.y));
                        pt.x = lp.x;
                        pt.y = lp.y;
                    }
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
            InputState::MovingSel {
                ptr_id: id,
                start_world,
                before,
            } if *id == ptr_id => {
                let pi = &inputs[inputs.len() - 1];
                let world_pos = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                let delta = world_pos - *start_world;
                let xf = Xform2D::translate(delta.x, delta.y);
                for (item_id, start_xf) in before {
                    self.session
                        .doc
                        .apply_world_xform_to_item(*item_id, xf, *start_xf);
                }
                self.session.doc.rebuild_runtime();
            }
            InputState::ScalingSel {
                ptr_id: id,
                pivot,
                start_world,
                before,
            } if *id == ptr_id => {
                let pi = &inputs[inputs.len() - 1];
                let world_pos = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                let start_dist = (*start_world - *pivot).len();
                let curr_dist = (world_pos - *pivot).len();
                let scale = if start_dist > 1e-4 {
                    curr_dist / start_dist
                } else {
                    1.0
                };
                let scale = scale.max(0.05);
                let xf = Xform2D::scale_about(*pivot, scale, scale);
                for (item_id, start_xf) in before {
                    self.session
                        .doc
                        .apply_world_xform_to_item(*item_id, xf, *start_xf);
                }
                self.session.doc.rebuild_runtime();
            }
            InputState::RotatingSel {
                ptr_id: id,
                pivot,
                start_angle,
                before,
            } if *id == ptr_id => {
                let pi = &inputs[inputs.len() - 1];
                let world_pos = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                let d = world_pos - *pivot;
                let curr_angle = d.y.atan2(d.x);
                let delta_angle = curr_angle - *start_angle;
                let xf = Xform2D::rotate_about(*pivot, delta_angle);
                for (item_id, start_xf) in before {
                    self.session
                        .doc
                        .apply_world_xform_to_item(*item_id, xf, *start_xf);
                }
                self.session.doc.rebuild_runtime();
            }
            InputState::MarqueeSelecting {
                ptr_id: id,
                start_screen,
                curr_screen,
                curr_world,
                active,
                ..
            } if *id == ptr_id => {
                let pi = &inputs[inputs.len() - 1];
                let world_pos = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                *curr_screen = Point2::new(pi.x, pi.y);
                *curr_world = world_pos;
                let dx = curr_screen.x - start_screen.x;
                let dy = curr_screen.y - start_screen.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist >= 5.0 {
                    *active = true;
                    self.update_cursor("crosshair");
                }
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
            InputState::MovingSel { ptr_id: id, .. } => *id == ptr_id,
            InputState::ScalingSel { ptr_id: id, .. } => *id == ptr_id,
            InputState::RotatingSel { ptr_id: id, .. } => *id == ptr_id,
            InputState::MarqueeSelecting { ptr_id: id, .. } => *id == ptr_id,
            InputState::Idle => false,
        };
        if !finishing {
            return;
        }
        let old_state = std::mem::replace(&mut self.input, InputState::Idle);
        match old_state {
            InputState::Drawing {
                mut builder,
                parent_id,
                parent_xform_inv,
                ..
            } => {
                let pi = PointerInput::from_event(e, &self.canvas);
                let mut pt = pi.to_ink_point(&self.vp);
                if let Some(inv) = parent_xform_inv {
                    let lp = inv.apply(Point2::new(pt.x, pt.y));
                    pt.x = lp.x;
                    pt.y = lp.y;
                }
                let mut should_add = true;
                if let Some(last) = builder.raw_pts.last() {
                    let dx = pt.x - last.x;
                    let dy = pt.y - last.y;
                    const MIN_PT_DIST: f32 = 0.25;
                    if dx * dx + dy * dy < MIN_PT_DIST * MIN_PT_DIST {
                        should_add = false;
                    }
                }
                if should_add {
                    builder.push(pt);
                }
                let now_ms = js_sys::Date::now() as i64;
                if let Some(stroke) = builder.finish(now_ms, parent_id) {
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
                    let tx = InkTx::new("erase").push(InkOp::DeleteItems { items: erased });
                    self.session.do_tx(tx);
                }
            }
            InputState::MovingSel { before, .. }
            | InputState::ScalingSel { before, .. }
            | InputState::RotatingSel { before, .. } => {
                self.update_cursor("");
                let mut item_ids = Vec::new();
                let mut before_xfs = Vec::new();
                let mut after_xfs = Vec::new();
                for (id, start_xf) in before {
                    if let Some(item) = self.session.doc.get_item(id) {
                        item_ids.push(id);
                        before_xfs.push(start_xf);
                        after_xfs.push(item.xform());
                    }
                }
                if !item_ids.is_empty() && before_xfs != after_xfs {
                    let tx = InkTx::new("transform selection").push(InkOp::TransformItems {
                        item_ids,
                        before: before_xfs,
                        after: after_xfs,
                    });
                    self.session.do_tx(tx);
                }
            }
            InputState::MarqueeSelecting {
                start_world,
                curr_world,
                active,
                ..
            } => {
                if active {
                    let min_x = start_world.x.min(curr_world.x);
                    let max_x = start_world.x.max(curr_world.x);
                    let min_y = start_world.y.min(curr_world.y);
                    let max_y = start_world.y.max(curr_world.y);
                    let rect = stonepen_core::bbox::BBox::new(min_x, min_y, max_x, max_y);
                    stonepen_core::select_rect(&mut self.session.doc, rect);
                } else {
                    self.session.doc.clear_sel();
                }
                let pi = PointerInput::from_event(e, &self.canvas);
                let (hit, _) = self.selection_hit_test(Point2::new(pi.x, pi.y));
                match hit {
                    SelHit::Move => self.update_cursor("move"),
                    SelHit::Rotate => self.update_cursor("grab"),
                    SelHit::Scale(handle) => match handle {
                        SelHandle::TopLeft | SelHandle::BottomRight => {
                            self.update_cursor("nwse-resize")
                        }
                        SelHandle::TopRight | SelHandle::BottomLeft => {
                            self.update_cursor("nesw-resize")
                        }
                    },
                    SelHit::None => {
                        let world_pos = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                        let clicked = self.session.doc.hit_test_item(world_pos, 8.0, self.vp.zoom);
                        if clicked.is_some() {
                            self.update_cursor("pointer");
                        } else {
                            self.update_cursor("default");
                        }
                    }
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
            InputState::MovingSel { ptr_id: id, .. } => *id == ptr_id,
            InputState::ScalingSel { ptr_id: id, .. } => *id == ptr_id,
            InputState::RotatingSel { ptr_id: id, .. } => *id == ptr_id,
            InputState::MarqueeSelecting { ptr_id: id, .. } => *id == ptr_id,
            InputState::Idle => false,
        };
        if cancel {
            let old_state = std::mem::replace(&mut self.input, InputState::Idle);
            match old_state {
                InputState::Erasing { erased, .. } => {
                    if !erased.is_empty() {
                        let tx = InkTx::new("erase").push(InkOp::DeleteItems { items: erased });
                        self.session.do_tx(tx);
                    }
                }
                InputState::MovingSel { before, .. }
                | InputState::ScalingSel { before, .. }
                | InputState::RotatingSel { before, .. } => {
                    self.update_cursor("");
                    for (id, start_xf) in before {
                        if let Some(item) = self.session.doc.get_item_mut(id) {
                            item.set_xform(start_xf);
                        }
                    }
                    self.session.doc.rebuild_runtime();
                }
                _ => {}
            }
            self.preview_pts.clear();
            self.lasso_preview.clear();
            self.update_cursor("default");
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
        if action.undo || action.redo || action.delete || action.escape || action.duplicate {
            e.prevent_default();
        }
        if action.undo {
            self.session.undo();
        } else if action.redo {
            self.session.redo();
        } else if action.delete {
            self.session.delete_sel();
        } else if action.escape {
            if matches!(self.input, InputState::MarqueeSelecting { .. }) {
                self.input = InputState::Idle;
                self.update_cursor("default");
            } else {
                self.session.doc.clear_sel();
                self.lasso_preview.clear();
                if matches!(self.input, InputState::Lassoing { .. }) {
                    self.input = InputState::Idle;
                }
            }
        } else if action.duplicate {
            self.session.duplicate_sel();
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
            "select" => Tool::Select,
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

    fn update_cursor(&self, cursor_str: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(canvas) = document.get_element_by_id("ink-canvas") {
                    if let Ok(html_canvas) = canvas.dyn_into::<web_sys::HtmlCanvasElement>() {
                        let _ = html_canvas.style().set_property("cursor", cursor_str);
                    }
                }
            }
        }
    }

    pub fn selection_hit_test(&self, screen_pt: Point2) -> (SelHit, Point2) {
        let sel = &self.session.doc.runtime.sel_items;
        if sel.is_empty() {
            return (SelHit::None, Point2::new(0.0, 0.0));
        }

        let handle_radius = 12.0f32;
        let hit_handle = |hx: f32, hy: f32| -> bool {
            let dx = screen_pt.x - hx;
            let dy = screen_pt.y - hy;
            dx * dx + dy * dy <= handle_radius * handle_radius
        };

        let mut single_image = None;
        if sel.len() == 1 {
            let id = *sel.iter().next().unwrap();
            if let Some(InkItem::Image(img)) = self.session.doc.get_item(id) {
                single_image = Some(img);
            }
        }

        if let Some(img) = single_image {
            let w = img.width;
            let h = img.height;
            let corners = [
                Point2::new(0.0, 0.0),
                Point2::new(w, 0.0),
                Point2::new(w, h),
                Point2::new(0.0, h),
            ];
            let sc: Vec<Point2> = corners
                .iter()
                .map(|&p| self.vp.world_to_screen(img.xform.apply(p)))
                .collect();
            let wc: Vec<Point2> = corners.iter().map(|&p| img.xform.apply(p)).collect();

            if hit_handle(sc[0].x, sc[0].y) {
                return (SelHit::Scale(SelHandle::TopLeft), wc[2]);
            }
            if hit_handle(sc[1].x, sc[1].y) {
                return (SelHit::Scale(SelHandle::TopRight), wc[3]);
            }
            if hit_handle(sc[2].x, sc[2].y) {
                return (SelHit::Scale(SelHandle::BottomRight), wc[0]);
            }
            if hit_handle(sc[3].x, sc[3].y) {
                return (SelHit::Scale(SelHandle::BottomLeft), wc[1]);
            }

            let top_mid = Point2::new((sc[0].x + sc[1].x) * 0.5, (sc[0].y + sc[1].y) * 0.5);
            let dx = sc[1].x - sc[0].x;
            let dy = sc[1].y - sc[0].y;
            let len = (dx * dx + dy * dy).sqrt();
            let (nx, ny) = if len > 1e-4 {
                (dy / len, -dx / len)
            } else {
                (0.0, -1.0)
            };
            let rx = top_mid.x + nx * 25.0;
            let ry = top_mid.y + ny * 25.0;

            if hit_handle(rx, ry) {
                let center_local = Point2::new(w * 0.5, h * 0.5);
                let center_world = img.xform.apply(center_local);
                return (SelHit::Rotate, center_world);
            }

            if let Some(inv) = img.xform.inverse() {
                let wp = self.vp.screen_to_world(screen_pt);
                let lp = inv.apply(wp);
                if lp.x >= 0.0 && lp.x <= w && lp.y >= 0.0 && lp.y <= h {
                    return (SelHit::Move, Point2::new(0.0, 0.0));
                }
            }
        } else {
            if let Some(bbox) = self.session.doc.selection_bbox() {
                let tl = self.vp.world_to_screen(Point2::new(bbox.min_x, bbox.min_y));
                let br = self.vp.world_to_screen(Point2::new(bbox.max_x, bbox.max_y));
                let x = tl.x;
                let y = tl.y;
                let w = br.x - tl.x;
                let h = br.y - tl.y;

                if hit_handle(x, y) {
                    return (
                        SelHit::Scale(SelHandle::TopLeft),
                        Point2::new(bbox.max_x, bbox.max_y),
                    );
                }
                if hit_handle(x + w, y) {
                    return (
                        SelHit::Scale(SelHandle::TopRight),
                        Point2::new(bbox.min_x, bbox.max_y),
                    );
                }
                if hit_handle(x + w, y + h) {
                    return (
                        SelHit::Scale(SelHandle::BottomRight),
                        Point2::new(bbox.min_x, bbox.min_y),
                    );
                }
                if hit_handle(x, y + h) {
                    return (
                        SelHit::Scale(SelHandle::BottomLeft),
                        Point2::new(bbox.max_x, bbox.min_y),
                    );
                }
                if hit_handle(x + w * 0.5, y - 25.0) {
                    return (
                        SelHit::Rotate,
                        Point2::new(
                            (bbox.min_x + bbox.max_x) * 0.5,
                            (bbox.min_y + bbox.max_y) * 0.5,
                        ),
                    );
                }

                let wp = self.vp.screen_to_world(screen_pt);
                if wp.x >= bbox.min_x
                    && wp.x <= bbox.max_x
                    && wp.y >= bbox.min_y
                    && wp.y <= bbox.max_y
                {
                    for &id in sel {
                        if let Some(item) = self.session.doc.get_item(id) {
                            let hit = match item {
                                InkItem::Stroke(s) => {
                                    self.session.doc.stroke_hit(s, wp, 8.0 / self.vp.zoom)
                                }
                                InkItem::Image(img) => {
                                    if let Some(inv) = img.xform.inverse() {
                                        let lp = inv.apply(wp);
                                        lp.x >= 0.0
                                            && lp.x <= img.width
                                            && lp.y >= 0.0
                                            && lp.y <= img.height
                                    } else {
                                        false
                                    }
                                }
                            };
                            if hit {
                                return (SelHit::Move, Point2::new(0.0, 0.0));
                            }
                        }
                    }
                }
            }
        }

        (SelHit::None, Point2::new(0.0, 0.0))
    }

    pub fn redraw(&self) {
        let canvas_w = self.canvas.client_width() as f64;
        let canvas_h = self.canvas.client_height() as f64;
        let preview_xf = match &self.input {
            InputState::Drawing {
                parent_id: Some(pid),
                ..
            } => {
                if let Some(parent_item) = self.session.doc.get_item(*pid) {
                    parent_item.xform()
                } else {
                    Xform2D::identity()
                }
            }
            _ => Xform2D::identity(),
        };
        let marquee = match &self.input {
            InputState::MarqueeSelecting {
                start_screen,
                curr_screen,
                active: true,
                ..
            } => {
                let min_x = start_screen.x.min(curr_screen.x);
                let max_x = start_screen.x.max(curr_screen.x);
                let min_y = start_screen.y.min(curr_screen.y);
                let max_y = start_screen.y.max(curr_screen.y);
                Some(stonepen_core::bbox::BBox::new(min_x, min_y, max_x, max_y))
            }
            _ => None,
        };
        self.renderer.render(
            &self.session,
            &self.vp,
            &self.preview_pts,
            preview_xf,
            &self.lasso_preview,
            marquee,
            canvas_w,
            canvas_h,
        );
    }

    pub fn paste_image(&mut self, bytes: &[u8], mime: &str, width_px: u32, height_px: u32) {
        let asset_id = AssetId::new();
        let asset = ImageAsset {
            id: asset_id,
            mime: mime.to_string(),
            width_px,
            height_px,
            bytes: bytes.to_vec(),
        };
        let visible = self.vp.visible_world_bbox();
        let v_w = visible.max_x - visible.min_x;
        let v_h = visible.max_y - visible.min_y;
        let mut w = width_px as f32;
        let mut h = height_px as f32;
        let max_w = v_w * 0.8;
        let max_h = v_h * 0.8;
        if w > max_w || h > max_h {
            let scale = (max_w / w).min(max_h / h);
            w *= scale;
            h *= scale;
        }
        let center = Point2::new(
            (visible.min_x + visible.max_x) * 0.5,
            (visible.min_y + visible.max_y) * 0.5,
        );
        let x = center.x - w * 0.5;
        let y = center.y - h * 0.5;
        let xform = Xform2D::translate(x, y);
        let item_id = ItemId::new();
        let mut img_item = stonepen_core::item::InkImage {
            id: item_id,
            asset_id,
            width: w,
            height: h,
            opacity: 1.0,
            xform,
            local_bbox: BBox::new(0.0, 0.0, w, h),
            world_bbox: BBox::new(0.0, 0.0, w, h),
            created_at_ms: js_sys::Date::now() as i64,
            updated_at_ms: js_sys::Date::now() as i64,
            geom_rev: 0,
        };
        img_item.recompute_world_bbox();

        let layer_id = self.session.doc.active_layer_id;
        let insert_idx = self
            .session
            .doc
            .active_layer()
            .map(|l| l.items.len())
            .unwrap_or(0);
        let tx = InkTx::new("paste image")
            .push(InkOp::AddAsset { asset })
            .push(InkOp::AddItems {
                layer_id,
                items: vec![(insert_idx, InkItem::Image(img_item))],
            });
        self.session.do_tx(tx);

        self.session.doc.clear_sel();
        self.session.doc.runtime.sel_items.insert(item_id);
        self.update_status();
        self.redraw();
    }

    fn erase_at_collect(&mut self, pos: Point2, radius: f32) -> Vec<(LayerId, usize, InkItem)> {
        let candidates = self.session.doc.hit_eraser(pos, radius);
        if candidates.is_empty() {
            return Vec::new();
        }
        let mut to_delete = Vec::new();
        for id in candidates {
            if let Some(InkItem::Stroke(s)) = self.session.doc.get_item(id) {
                if stonepen_core::hit::stroke_hit(s, pos, radius) {
                    to_delete.push(id);
                }
            }
        }
        if to_delete.is_empty() {
            return Vec::new();
        }
        self.session.doc.delete_items(&to_delete)
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
        let total: usize = self.session.doc.layers.iter().map(|l| l.items.len()).sum();
        let sel = self.session.doc.runtime.sel_items.len();
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
            "items: {total}  selected: {sel}  tool: {tool_str}  zoom: {zoom_pct}%  {dirty_str}"
        );
        if let Some(el) = document.get_element_by_id("status-bar") {
            el.set_text_content(Some(&status));
        }
    }
}
