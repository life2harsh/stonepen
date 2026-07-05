use serde::Serialize;
use stonepen_core::bbox::BBox;
use stonepen_core::brush::Brush;
use stonepen_core::clipboard::ClipboardBundle;
use stonepen_core::ids::{AssetId, ItemId, LayerId};
use stonepen_core::item::{ImageAsset, InkItem};
use stonepen_core::ops::{InkOp, InkTx};
use stonepen_core::point::{InkPoint, Point2, PointerKind};
use stonepen_core::sel::SelectionIntent;
use stonepen_core::session::{InkSession, Tool, ZOrderCmd};
use stonepen_core::stroke::StrokeBuilder;
use stonepen_core::viewport::Viewport;
use stonepen_core::xform::Xform2D;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent, PointerEvent, WheelEvent};

use crate::canvas::{get_2d_context, get_canvas, sync_canvas_size};
use crate::file_io::{trigger_download, trigger_png_download};
use crate::keyboard::parse_event_to_chord;
use crate::pointer::{get_inputs, PointerInput};
use crate::render_2d::Renderer;
use crate::web_ui::WebUi;
use stonepen_core::shortcuts::{
    AppSettings, Command, ConflictError, KeyChord, ShortcutMap, TempPanController,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelHandle {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Top,
    Right,
    Bottom,
    Left,
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
        intent: SelectionIntent,
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
        handle: SelHandle,
        pivot: Point2,
        start_world: Point2,
        before: Vec<(ItemId, Xform2D)>,
        img_xform: Option<Xform2D>,
        img_size: Option<(f32, f32)>,
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
        intent: SelectionIntent,
    },
}

#[derive(Debug, Clone)]
pub struct NudgeState {
    pub active_cmd: Command,
    pub before_xforms: Vec<(ItemId, Xform2D)>,
    pub total_delta: Point2,
}

#[derive(Debug, Clone)]
pub struct StylePreviewState {
    pub stroke_ids: Vec<ItemId>,
    pub original_brushes: Vec<Brush>,
}

pub struct StonepenApp {
    canvas: HtmlCanvasElement,
    renderer: Renderer,
    pub session: InkSession,
    pub vp: Viewport,
    input: InputState,
    pub dpr: f64,
    preview_pts: Vec<InkPoint>,
    lasso_preview: Vec<Point2>,
    pub settings: AppSettings,
    pub temp_pan: TempPanController,
    pub capture_command: Option<Command>,
    pub last_conflict: Option<Command>,
    pub nudge_state: Option<NudgeState>,
    pub clipboard: Option<ClipboardBundle>,
    pub status_msg: Option<String>,
    pub paste_generation: u32,
    pub style_preview: Option<StylePreviewState>,
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
        let mut settings = AppSettings::new();
        if let Some(storage) = window.local_storage().ok().flatten() {
            if let Some(json_str) = storage.get_item("stonepen.settings.v1").ok().flatten() {
                match serde_json::from_str::<AppSettings>(&json_str) {
                    Ok(mut loaded) => {
                        if AppSettings::is_version_supported(loaded.version) {
                            loaded.validate_and_repair();
                            settings = loaded;
                        } else {
                            web_sys::console::warn_1(&JsValue::from_str(&format!(
                                "Unsupported settings version: {}, falling back to defaults",
                                loaded.version
                            )));
                        }
                    }
                    Err(e) => {
                        web_sys::console::warn_1(&JsValue::from_str(&format!(
                            "Malformed JSON in settings: {:?}",
                            e
                        )));
                    }
                }
            }
        }
        Ok(Self {
            canvas,
            renderer,
            session,
            vp,
            input: InputState::Idle,
            dpr,
            preview_pts: Vec::new(),
            lasso_preview: Vec::new(),
            settings,
            temp_pan: TempPanController::new(),
            capture_command: None,
            last_conflict: None,
            nudge_state: None,
            clipboard: None,
            status_msg: None,
            paste_generation: 0,
            style_preview: None,
        })
    }

    pub fn should_start_gesture(tool: Tool, pointer_kind: stonepen_core::point::PointerKind, buttons: u16) -> bool {
        match tool {
            Tool::Pen | Tool::Pencil | Tool::Highlighter => {
                match pointer_kind {
                    stonepen_core::point::PointerKind::Pen => true,
                    stonepen_core::point::PointerKind::Mouse => buttons & 1 != 0,
                    _ => false,
                }
            }
            Tool::StrokeEraser | Tool::Lasso | Tool::Pan | Tool::Select => true,
        }
    }

    pub fn should_cancel_gesture(input: &InputState, ptr_id: i32) -> bool {
        match input {
            InputState::Drawing { ptr_id: id, .. } => *id == ptr_id,
            InputState::Lassoing { ptr_id: id, .. } => *id == ptr_id,
            InputState::Erasing { ptr_id: id, .. } => *id == ptr_id,
            InputState::Panning { ptr_id: id, .. } => *id == ptr_id,
            InputState::MovingSel { ptr_id: id, .. } => *id == ptr_id,
            InputState::ScalingSel { ptr_id: id, .. } => *id == ptr_id,
            InputState::RotatingSel { ptr_id: id, .. } => *id == ptr_id,
            InputState::MarqueeSelecting { ptr_id: id, .. } => *id == ptr_id,
            InputState::Idle => false,
        }
    }

    pub fn on_pointer_down(&mut self, e: &PointerEvent) -> bool {
        self.commit_nudge();
        e.prevent_default();
        let pi = PointerInput::from_event(e, &self.canvas);
        let mut gesture_started = false;
        match self.effective_tool() {
            Tool::Pen | Tool::Pencil | Tool::Highlighter => {
                let draws = Self::should_start_gesture(self.effective_tool(), pi.kind, pi.buttons);
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
                    gesture_started = true;
                }
            }
            Tool::StrokeEraser => {
                let world = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                let erased = self.erase_at_collect(world, 12.0);
                self.input = InputState::Erasing {
                    ptr_id: pi.id,
                    erased,
                };
                gesture_started = true;
            }
            Tool::Lasso => {
                let world = self.vp.screen_to_world(Point2::new(pi.x, pi.y));
                self.lasso_preview = vec![world];
                let intent = if e.shift_key() {
                    SelectionIntent::Add
                } else {
                    SelectionIntent::Replace
                };
                self.input = InputState::Lassoing {
                    ptr_id: pi.id,
                    polygon: vec![world],
                    intent,
                };
                gesture_started = true;
            }
            Tool::Pan => {
                self.input = InputState::Panning {
                    ptr_id: pi.id,
                    last_sx: pi.x,
                    last_sy: pi.y,
                };
                gesture_started = true;
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
                        gesture_started = true;
                    }
                    SelHit::Scale(handle) => {
                        let roots = self.session.doc.transform_roots();
                        let before = roots
                            .iter()
                            .map(|&id| (id, self.session.doc.get_item(id).unwrap().xform()))
                            .collect();
                        let single_image = self.session.doc.single_selected_image_root();
                        let (img_xform, img_size) = if let Some(img_id) = single_image {
                            if let Some(InkItem::Image(img)) = self.session.doc.get_item(img_id) {
                                (Some(img.xform), Some((img.width, img.height)))
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        };
                        self.input = InputState::ScalingSel {
                            ptr_id: pi.id,
                            handle,
                            pivot: handle_pivot,
                            start_world,
                            before,
                            img_xform,
                            img_size,
                        };
                        gesture_started = true;
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
                        gesture_started = true;
                    }
                    SelHit::None => {
                        let clicked = self.session.doc.hit_test_item(world_pos, 8.0, self.vp.zoom);
                        if let Some(id) = clicked {
                            let is_selected = self.session.doc.runtime.sel_items.contains(&id);
                            if e.shift_key() {
                                stonepen_core::apply_selection_hits(
                                    &mut self.session.doc,
                                    &[id],
                                    SelectionIntent::Toggle,
                                );
                                if is_selected {
                                    self.update_cursor("default");
                                    self.input = InputState::Idle;
                                    gesture_started = false;
                                } else {
                                    let roots = self.session.doc.transform_roots();
                                    let before = roots
                                        .iter()
                                        .map(|&rid| {
                                            (rid, self.session.doc.get_item(rid).unwrap().xform())
                                        })
                                        .collect();
                                    self.update_cursor("grabbing");
                                    self.input = InputState::MovingSel {
                                        ptr_id: pi.id,
                                        start_world,
                                        before,
                                    };
                                    gesture_started = true;
                                }
                            } else {
                                if is_selected {
                                    let roots = self.session.doc.transform_roots();
                                    let before = roots
                                        .iter()
                                        .map(|&rid| {
                                            (rid, self.session.doc.get_item(rid).unwrap().xform())
                                        })
                                        .collect();
                                    self.update_cursor("grabbing");
                                    self.input = InputState::MovingSel {
                                        ptr_id: pi.id,
                                        start_world,
                                        before,
                                    };
                                    gesture_started = true;
                                } else {
                                    stonepen_core::apply_selection_hits(
                                        &mut self.session.doc,
                                        &[id],
                                        SelectionIntent::Replace,
                                    );
                                    let roots = self.session.doc.transform_roots();
                                    let before = roots
                                        .iter()
                                        .map(|&rid| {
                                            (rid, self.session.doc.get_item(rid).unwrap().xform())
                                        })
                                        .collect();
                                    self.update_cursor("grabbing");
                                    self.input = InputState::MovingSel {
                                        ptr_id: pi.id,
                                        start_world,
                                        before,
                                    };
                                    gesture_started = true;
                                }
                            }
                        } else {
                            let intent = if e.shift_key() {
                                SelectionIntent::Add
                            } else {
                                SelectionIntent::Replace
                            };
                            self.input = InputState::MarqueeSelecting {
                                ptr_id: pi.id,
                                start_screen: Point2::new(pi.x, pi.y),
                                start_world,
                                curr_screen: Point2::new(pi.x, pi.y),
                                curr_world: world_pos,
                                active: false,
                                intent,
                            };
                            gesture_started = true;
                        }
                    }
                }
            }
        }
        self.redraw();
        gesture_started
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
                    SelHit::Scale(handle) => {
                        let cursor = self.get_scale_cursor(handle);
                        self.update_cursor(cursor);
                    }
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
                ..
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
                handle,
                pivot,
                start_world,
                before,
                img_xform,
                img_size,
            } if *id == ptr_id => {
                let pi = &inputs[inputs.len() - 1];
                let world_pos = self.vp.screen_to_world(Point2::new(pi.x, pi.y));

                let world_xf = if let (Some(img_xf), Some((w, h))) = (img_xform, img_size) {
                    let inv = img_xf.inverse().unwrap_or_else(Xform2D::identity);
                    let m_local = inv.apply(world_pos);

                    let a_local = match handle {
                        SelHandle::TopLeft => Point2::new(*w, *h),
                        SelHandle::TopRight => Point2::new(0.0, *h),
                        SelHandle::BottomRight => Point2::new(0.0, 0.0),
                        SelHandle::BottomLeft => Point2::new(*w, 0.0),
                        SelHandle::Top => Point2::new(*w * 0.5, *h),
                        SelHandle::Right => Point2::new(0.0, *h * 0.5),
                        SelHandle::Bottom => Point2::new(*w * 0.5, 0.0),
                        SelHandle::Left => Point2::new(*w, *h * 0.5),
                    };

                    let h_local = match handle {
                        SelHandle::TopLeft => Point2::new(0.0, 0.0),
                        SelHandle::TopRight => Point2::new(*w, 0.0),
                        SelHandle::BottomRight => Point2::new(*w, *h),
                        SelHandle::BottomLeft => Point2::new(0.0, *h),
                        SelHandle::Top => Point2::new(*w * 0.5, 0.0),
                        SelHandle::Right => Point2::new(*w, *h * 0.5),
                        SelHandle::Bottom => Point2::new(*w * 0.5, *h),
                        SelHandle::Left => Point2::new(0.0, *h * 0.5),
                    };

                    let mut sx = if (h_local.x - a_local.x).abs() > 1e-4 {
                        (m_local.x - a_local.x) / (h_local.x - a_local.x)
                    } else {
                        1.0
                    };
                    let mut sy = if (h_local.y - a_local.y).abs() > 1e-4 {
                        (m_local.y - a_local.y) / (h_local.y - a_local.y)
                    } else {
                        1.0
                    };

                    match handle {
                        SelHandle::Top | SelHandle::Bottom => sx = 1.0,
                        SelHandle::Left | SelHandle::Right => sy = 1.0,
                        _ => {}
                    }

                    sx = sx.max(0.001);
                    sy = sy.max(0.001);

                    let s_local = Xform2D::translate(a_local.x, a_local.y)
                        .concat(Xform2D::scale(sx, sy))
                        .concat(Xform2D::translate(-a_local.x, -a_local.y));

                    img_xf.concat(s_local).concat(inv)
                } else {
                    let mut sx = if (start_world.x - pivot.x).abs() > 1e-4 {
                        (world_pos.x - pivot.x) / (start_world.x - pivot.x)
                    } else {
                        1.0
                    };
                    let mut sy = if (start_world.y - pivot.y).abs() > 1e-4 {
                        (world_pos.y - pivot.y) / (start_world.y - pivot.y)
                    } else {
                        1.0
                    };

                    match handle {
                        SelHandle::Top | SelHandle::Bottom => sx = 1.0,
                        SelHandle::Left | SelHandle::Right => sy = 1.0,
                        _ => {}
                    }

                    sx = sx.max(0.001);
                    sy = sy.max(0.001);

                    Xform2D::translate(pivot.x, pivot.y)
                        .concat(Xform2D::scale(sx, sy))
                        .concat(Xform2D::translate(-pivot.x, -pivot.y))
                };

                for (item_id, start_xf) in before {
                    self.session
                        .doc
                        .apply_world_xform_to_item(*item_id, world_xf, *start_xf);
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
            InputState::Lassoing {
                polygon, intent, ..
            } => {
                let hits = stonepen_core::lasso_query(&self.session.doc, &polygon);
                stonepen_core::apply_selection_hits(&mut self.session.doc, &hits, intent);
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
                intent,
                ..
            } => {
                if active {
                    let min_x = start_world.x.min(curr_world.x);
                    let max_x = start_world.x.max(curr_world.x);
                    let min_y = start_world.y.min(curr_world.y);
                    let max_y = start_world.y.max(curr_world.y);
                    let rect = stonepen_core::bbox::BBox::new(min_x, min_y, max_x, max_y);
                    let hits = stonepen_core::rect_query(&self.session.doc, rect);
                    stonepen_core::apply_selection_hits(&mut self.session.doc, &hits, intent);
                } else {
                    if intent == SelectionIntent::Replace {
                        self.session.doc.clear_sel();
                    }
                }
                let pi = PointerInput::from_event(e, &self.canvas);
                let (hit, _) = self.selection_hit_test(Point2::new(pi.x, pi.y));
                match hit {
                    SelHit::Move => self.update_cursor("move"),
                    SelHit::Rotate => self.update_cursor("grab"),
                    SelHit::Scale(handle) => {
                        let cursor = self.get_scale_cursor(handle);
                        self.update_cursor(cursor);
                    }
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
            InputState::Panning { .. } => {
                self.temp_pan.handle_gesture_end();
                self.refresh_cursor();
            }
            InputState::Idle => {}
        }
        self.update_status();
        self.redraw();
        self.sync_selection_bar();
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
                InputState::Panning { .. } => {
                    self.temp_pan.handle_gesture_end();
                }
                _ => {}
            }
            self.preview_pts.clear();
            self.lasso_preview.clear();
            self.refresh_cursor();
            self.redraw();
        }
    }

    pub fn on_wheel(&mut self, e: &WheelEvent) {
        self.commit_nudge();
        e.prevent_default();
        let cx = e.client_x() as f32;
        let cy = e.client_y() as f32;
        let delta = e.delta_y();
        let factor = if delta > 0.0 { 0.9 } else { 1.0 / 0.9 };
        self.vp.zoom_at_screen_pos(Point2::new(cx, cy), factor);
        self.update_status();
        self.redraw();
    }

    pub fn on_key_down(&mut self, e: &KeyboardEvent) {
        let chord = parse_event_to_chord(e);
        if let Some(cmd_to_capture) = self.capture_command {
            e.prevent_default();
            if chord.code == "Escape" {
                self.capture_command = None;
                return;
            }
            if chord.is_modifier_only() {
                return;
            }
            match self
                .settings
                .shortcuts
                .add_binding(cmd_to_capture, chord.clone())
            {
                Ok(_) => {
                    self.capture_command = None;
                    self.save_settings();
                }
                Err(ConflictError::Conflict(other_cmd)) => {
                    self.last_conflict = Some(other_cmd);
                }
                Err(ConflictError::ModifierOnly) => {}
            }
            return;
        }

        if chord.code == "Escape" {
            let has_preview = self.style_preview.is_some();
            let has_transient = !matches!(self.input, InputState::Idle);
            if has_preview {
                self.cancel_style_preview();
                if let Ok(ui) = WebUi::new() {
                    ui.focus_canvas();
                }
                e.prevent_default();
                return;
            }
            if has_transient {
                self.reset_transient_input();
                if let Ok(ui) = WebUi::new() {
                    ui.focus_canvas();
                }
                e.prevent_default();
                return;
            }
        }

        let mut matched_cmd = self.settings.shortcuts.command_for_chord(&chord);
        let mut shift_nudge = e.shift_key();

        if matched_cmd.is_none() && e.shift_key() {
            let mut fallback_chord = chord.clone();
            fallback_chord.shift = false;
            if let Some(cmd) = self.settings.shortcuts.command_for_chord(&fallback_chord) {
                if matches!(
                    cmd,
                    Command::NudgeLeft
                        | Command::NudgeRight
                        | Command::NudgeUp
                        | Command::NudgeDown
                ) {
                    matched_cmd = Some(cmd);
                    shift_nudge = true;
                }
            }
        }

        if let Some(cmd) = matched_cmd {
            if e.repeat() && !cmd.allows_repeat() {
                return;
            }
            let is_native_paste = cmd == Command::Paste
                && chord.code == "KeyV"
                && chord.primary
                && !chord.shift
                && !chord.alt;

            if !is_native_paste {
                e.prevent_default();
            }

            if cmd == Command::HoldPan {
                let is_idle = matches!(self.input, InputState::Idle);
                if self.temp_pan.handle_keydown(&chord.code, is_idle) {
                    self.refresh_cursor();
                    self.update_status();
                    self.redraw();
                }
            } else if matches!(
                cmd,
                Command::NudgeLeft | Command::NudgeRight | Command::NudgeUp | Command::NudgeDown
            ) {
                self.execute_nudge(cmd, shift_nudge);
            } else {
                if matches!(self.input, InputState::Idle)
                    || cmd == Command::Undo
                    || cmd == Command::Redo
                    || cmd == Command::ClearSelection
                {
                    if !is_native_paste {
                        self.dispatch_command(cmd);
                    }
                }
            }
        }
    }

    pub fn on_key_up(&mut self, e: &KeyboardEvent) {
        let code = e.code();
        let is_dragging_pan = matches!(self.input, InputState::Panning { .. });
        if self.temp_pan.handle_keyup(&code, is_dragging_pan) {
            self.refresh_cursor();
            self.update_status();
            self.redraw();
        }

        if let Some(ref state) = self.nudge_state {
            let chords = self.settings.shortcuts.bindings(state.active_cmd);
            let is_nudge_key = chords.iter().any(|c| c.code == code);
            if is_nudge_key {
                self.commit_nudge();
            }
        }
    }

    pub fn on_blur(&mut self) {
        self.reset_transient_input();
    }

    pub fn commit_nudge(&mut self) {
        if let Some(state) = self.nudge_state.take() {
            let mut item_ids = Vec::new();
            let mut before_xfs = Vec::new();
            let mut after_xfs = Vec::new();
            for (id, start_xf) in state.before_xforms {
                if let Some(item) = self.session.doc.get_item(id) {
                    item_ids.push(id);
                    before_xfs.push(start_xf);
                    after_xfs.push(item.xform());
                }
            }
            if !item_ids.is_empty() && before_xfs != after_xfs {
                let tx = InkTx::new("nudge").push(InkOp::TransformItems {
                    item_ids,
                    before: before_xfs,
                    after: after_xfs,
                });
                self.session.do_tx(tx);
                self.update_status();
            }
        }
    }

    pub fn execute_nudge(&mut self, cmd: Command, shift: bool) {
        if !matches!(
            cmd,
            Command::NudgeLeft | Command::NudgeRight | Command::NudgeUp | Command::NudgeDown
        ) {
            return;
        }

        if let Some(ref state) = self.nudge_state {
            if state.active_cmd != cmd {
                self.commit_nudge();
            }
        }

        if self.nudge_state.is_none() {
            let roots = self.session.doc.transform_roots();
            if roots.is_empty() {
                return;
            }
            let before_xforms = roots
                .iter()
                .map(|&id| (id, self.session.doc.get_item(id).unwrap().xform()))
                .collect();
            self.nudge_state = Some(NudgeState {
                active_cmd: cmd,
                before_xforms,
                total_delta: Point2::new(0.0, 0.0),
            });
        }

        let step_size = if shift { 10.0 } else { 1.0 };
        let dx_css = match cmd {
            Command::NudgeLeft => -step_size,
            Command::NudgeRight => step_size,
            _ => 0.0,
        };
        let dy_css = match cmd {
            Command::NudgeUp => -step_size,
            Command::NudgeDown => step_size,
            _ => 0.0,
        };

        let dx_world = dx_css / self.vp.zoom;
        let dy_world = dy_css / self.vp.zoom;

        if let Some(ref mut state) = self.nudge_state {
            state.total_delta.x += dx_world;
            state.total_delta.y += dy_world;

            let translation = Xform2D::translate(state.total_delta.x, state.total_delta.y);
            for &(id, start_xf) in &state.before_xforms {
                self.session
                    .doc
                    .apply_world_xform_to_item(id, translation, start_xf);
            }
            self.session.doc.rebuild_runtime();
        }
        self.redraw();
    }

    pub fn reset_transient_input(&mut self) {
        self.commit_nudge();
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
        self.temp_pan.reset();
        self.capture_command = None;
        self.refresh_cursor();
        self.update_status();
        self.redraw();
    }

    pub fn set_tool(&mut self, tool: &str) {
        let cmd = match tool {
            "pen" => Command::ToolPen,
            "pencil" => Command::ToolPencil,
            "highlighter" => Command::ToolHighlighter,
            "eraser" => Command::ToolEraser,
            "lasso" => Command::ToolLasso,
            "select" => Command::ToolSelect,
            "pan" => Command::ToolPan,
            _ => return,
        };
        self.dispatch_command(cmd);
    }

    fn save_settings(&self) {
        if let Some(window) = web_sys::window() {
            if let Some(storage) = window.local_storage().ok().flatten() {
                if let Ok(json) = serde_json::to_string(&self.settings) {
                    let _ = storage.set_item("stonepen.settings.v1", &json);
                }
            }
        }
    }

    pub fn get_platform_is_mac(&self) -> bool {
        if let Some(window) = web_sys::window() {
            let nav = window.navigator();
            if let Ok(ua) = nav.user_agent() {
                let lower = ua.to_lowercase();
                return lower.contains("macintosh")
                    || lower.contains("mac os x")
                    || lower.contains("ipad")
                    || lower.contains("iphone");
            }
        }
        false
    }

    pub fn get_shortcuts_json(&self) -> String {
        let mut rows = Vec::new();
        #[derive(Serialize)]
        struct ShortcutRow {
            command_id: &'static str,
            label: &'static str,
            bindings: Vec<String>,
            chords: Vec<KeyChord>,
        }
        let is_mac = self.get_platform_is_mac();
        for cmd in &Command::ALL {
            let chords = self.settings.shortcuts.bindings(*cmd);
            let bindings_str = chords.iter().map(|c| c.to_display_string(is_mac)).collect();
            rows.push(ShortcutRow {
                command_id: cmd.to_id(),
                label: cmd.label(),
                bindings: bindings_str,
                chords: chords.to_vec(),
            });
        }
        serde_json::to_string(&rows).unwrap_or_default()
    }

    pub fn start_capture(&mut self, command_id: &str) {
        if let Some(cmd) = Command::from_id(command_id) {
            self.capture_command = Some(cmd);
        }
    }

    pub fn cancel_capture(&mut self) {
        self.capture_command = None;
    }

    pub fn is_capturing(&self) -> bool {
        self.capture_command.is_some()
    }

    pub fn capturing_label(&self) -> String {
        self.capture_command
            .map(|c| c.label().to_string())
            .unwrap_or_default()
    }

    pub fn remove_shortcut_binding(&mut self, command_id: &str, index: usize) {
        if let Some(cmd) = Command::from_id(command_id) {
            let bindings = self.settings.shortcuts.bindings(cmd);
            if index < bindings.len() {
                let chord = bindings[index].clone();
                self.settings.shortcuts.remove_binding(cmd, &chord);
                self.save_settings();
            }
        }
    }

    pub fn reset_shortcuts_to_defaults(&mut self) {
        self.settings.shortcuts = ShortcutMap::defaults();
        self.save_settings();
        self.redraw();
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
        self.commit_nudge();
        self.session.undo();
        self.update_status();
        self.redraw();
    }

    pub fn action_redo(&mut self) {
        self.commit_nudge();
        self.session.redo();
        self.update_status();
        self.redraw();
    }

    pub fn action_clear(&mut self) {
        self.commit_nudge();
        self.session.clear_active_layer();
        self.update_status();
        self.redraw();
    }

    pub fn action_save(&mut self) {
        self.commit_nudge();
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
            Ok(mut new_session) => {
                self.reset_transient_input();
                self.nudge_state = None;
                self.input = InputState::Idle;
                self.preview_pts.clear();
                self.lasso_preview.clear();
                self.temp_pan.reset();
                self.capture_command = None;

                let old_tool = self.session.active_tool.clone();
                let old_brush = self.session.active_brush.clone();
                new_session.active_tool = old_tool;
                new_session.active_brush = old_brush;
                self.session = new_session;

                let tool_name = match self.session.active_tool {
                    Tool::Pen => "pen",
                    Tool::Pencil => "pencil",
                    Tool::Highlighter => "highlighter",
                    Tool::StrokeEraser => "eraser",
                    Tool::Lasso => "lasso",
                    Tool::Select => "select",
                    Tool::Pan => "pan",
                };
                self.sync_tool_ui(tool_name);
                self.sync_brush_controls();

                if let Ok(ui) = WebUi::new() {
                    ui.sync_capture_overlay(self);
                }

                self.update_status();
                self.redraw();
            }
            Err(e) => {
                web_sys::console::error_1(&JsValue::from_str(&format!("{e}")));
            }
        }
    }

    fn reset_trans_state(&mut self) {
        self.reset_transient_input();
        let tool_name = match self.session.active_tool {
            Tool::Pen => "pen",
            Tool::Pencil => "pencil",
            Tool::Highlighter => "highlighter",
            Tool::StrokeEraser => "eraser",
            Tool::Lasso => "lasso",
            Tool::Select => "select",
            Tool::Pan => "pan",
        };
        self.sync_tool_ui(tool_name);
        self.sync_brush_controls();
        if let Ok(ui) = WebUi::new() {
            ui.sync_capture_overlay(self);
        }
    }

    pub fn sync_brush_controls(&self) {
        if let Ok(ui) = WebUi::new() {
            ui.sync_brush_controls(&self.session.active_brush);
        }
    }

    pub fn action_export_svg(&mut self) {
        self.commit_nudge();
        match self.session.export_svg() {
            Ok(svg) => {
                let _ = trigger_download("drawing.svg", &svg, "image/svg+xml");
            }
            Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!("{e}"))),
        }
    }

    pub fn action_export_png(&mut self) {
        self.commit_nudge();
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

    pub fn effective_tool(&self) -> Tool {
        if self.temp_pan.is_active() {
            Tool::Pan
        } else {
            self.session.active_tool.clone()
        }
    }

    pub fn refresh_cursor(&self) {
        if self.temp_pan.is_active() {
            if matches!(self.input, InputState::Panning { .. }) {
                self.update_cursor("grabbing");
            } else {
                self.update_cursor("grab");
            }
            return;
        }
        match &self.input {
            InputState::Idle => match self.effective_tool() {
                Tool::Pan => self.update_cursor("grab"),
                Tool::Select => self.update_cursor("default"),
                Tool::StrokeEraser => self.update_cursor("cell"),
                Tool::Lasso => self.update_cursor("crosshair"),
                _ => self.update_cursor("default"),
            },
            InputState::Panning { .. } => {
                self.update_cursor("grabbing");
            }
            InputState::MovingSel { .. } => {
                self.update_cursor("grabbing");
            }
            InputState::ScalingSel { .. } => {}
            InputState::RotatingSel { .. } => {
                self.update_cursor("grabbing");
            }
            InputState::MarqueeSelecting { active: true, .. } => {
                self.update_cursor("crosshair");
            }
            InputState::MarqueeSelecting { active: false, .. } => {
                self.update_cursor("default");
            }
            _ => {
                self.update_cursor("default");
            }
        }
    }

    pub fn dispatch_command(&mut self, cmd: Command) {
        if self.capture_command.is_some() {
            return;
        }
        if self.nudge_state.is_some() {
            if !matches!(
                cmd,
                Command::NudgeLeft | Command::NudgeRight | Command::NudgeUp | Command::NudgeDown
            ) {
                self.commit_nudge();
            }
        }

        match cmd {
            Command::ToolPen => {
                self.session.active_brush = Brush::default_pen();
                self.session.active_tool = Tool::Pen;
                self.sync_tool_ui("pen");
            }
            Command::ToolPencil => {
                self.session.active_brush = Brush::default_pencil();
                self.session.active_tool = Tool::Pencil;
                self.sync_tool_ui("pencil");
            }
            Command::ToolHighlighter => {
                self.session.active_brush = Brush::default_highlighter();
                self.session.active_tool = Tool::Highlighter;
                self.sync_tool_ui("highlighter");
            }
            Command::ToolEraser => {
                self.session.active_tool = Tool::StrokeEraser;
                self.sync_tool_ui("eraser");
            }
            Command::ToolLasso => {
                self.session.active_tool = Tool::Lasso;
                self.sync_tool_ui("lasso");
            }
            Command::ToolSelect => {
                self.session.active_tool = Tool::Select;
                self.sync_tool_ui("select");
            }
            Command::ToolPan => {
                self.session.active_tool = Tool::Pan;
                self.sync_tool_ui("pan");
            }
            Command::Undo => {
                self.action_undo();
            }
            Command::Redo => {
                self.action_redo();
            }
            Command::DeleteSelection => {
                self.session.delete_sel();
            }
            Command::DuplicateSelection => {
                self.session.duplicate_sel();
            }
            Command::ClearSelection => {
                if self.style_preview.is_some() {
                    self.cancel_style_preview();
                } else if matches!(self.input, InputState::MarqueeSelecting { .. }) {
                    self.input = InputState::Idle;
                    self.refresh_cursor();
                } else if matches!(self.input, InputState::Lassoing { .. }) {
                    self.input = InputState::Idle;
                    self.lasso_preview.clear();
                    self.refresh_cursor();
                } else {
                    self.session.doc.clear_sel();
                }
            }
            Command::HoldPan => {}
            Command::SelectAll => {
                self.session.select_all();
                self.status_msg = Some("Selected all".into());
            }
            Command::Copy => {
                if let Some(bundle) = self.session.copy_sel() {
                    let count = bundle.records.len();
                    self.clipboard = Some(bundle);
                    self.paste_generation = 0;
                    self.status_msg = Some(format!("Copied {count} items"));
                }
            }
            Command::Cut => {
                if let Some(bundle) = self.session.cut_sel() {
                    let count = bundle.records.len();
                    self.clipboard = Some(bundle);
                    self.paste_generation = 0;
                    self.status_msg = Some(format!("Cut {count} items"));
                }
            }
            Command::Paste => {
                if let Some(ref bundle) = self.clipboard {
                    self.paste_generation += 1;
                    let offset_css = 20.0 * (self.paste_generation as f32);
                    let offset_world = offset_css / self.vp.zoom;
                    let xform = Xform2D::translate(offset_world, offset_world);
                    let pasted_ids = self.session.paste_sel(bundle, xform);
                    self.status_msg = Some(format!("Pasted {} items", pasted_ids.len()));
                }
            }
            Command::NudgeLeft | Command::NudgeRight | Command::NudgeUp | Command::NudgeDown => {
                self.execute_nudge(cmd, false);
            }
            Command::BringForward => {
                self.session.z_order_sel(ZOrderCmd::BringForward);
                self.status_msg = Some("Moved forward".into());
            }
            Command::SendBackward => {
                self.session.z_order_sel(ZOrderCmd::SendBackward);
                self.status_msg = Some("Moved backward".into());
            }
            Command::BringToFront => {
                self.session.z_order_sel(ZOrderCmd::BringToFront);
                self.status_msg = Some("Moved to front".into());
            }
            Command::SendToBack => {
                self.session.z_order_sel(ZOrderCmd::SendToBack);
                self.status_msg = Some("Moved to back".into());
            }
        }
        self.update_status();
        self.redraw();
        self.sync_brush_controls();
        self.sync_selection_bar();
    }

    fn sync_tool_ui(&self, active_tool_name: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
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
                    if let Some(el) = document.get_element_by_id(&id) {
                        if btn_name == active_tool_name {
                            let _ = el.class_list().add_1("active");
                        } else {
                            let _ = el.class_list().remove_1("active");
                        }
                    }
                }
                if let Some(canvas) = document.get_element_by_id("ink-canvas") {
                    let _ = canvas.set_class_name("");
                    if active_tool_name == "pan" {
                        let _ = canvas.class_list().add_1("tool-pan");
                    } else if active_tool_name == "eraser" {
                        let _ = canvas.class_list().add_1("tool-eraser");
                    } else if active_tool_name == "lasso" {
                        let _ = canvas.class_list().add_1("tool-lasso");
                    } else if active_tool_name == "select" {
                        let _ = canvas.class_list().add_1("tool-select");
                    }
                }
            }
        }
        self.refresh_cursor();
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

        // 1. Check all 8 handles
        if let Some(handles) = self.get_selection_handle_screen_pts() {
            for (handle, pt) in handles {
                if hit_handle(pt.x, pt.y) {
                    let anchor = self.get_handle_world_anchor(handle);
                    return (SelHit::Scale(handle), anchor);
                }
            }
        }

        // 2. Check rotation handle
        let single_image = self.session.doc.single_selected_image_root();
        if let Some(img_id) = single_image {
            if let Some(InkItem::Image(img)) = self.session.doc.get_item(img_id) {
                let w = img.width;
                let h = img.height;
                let corners = [
                    Point2::new(0.0, 0.0),
                    Point2::new(w, 0.0),
                ];
                let sc: Vec<Point2> = corners
                    .iter()
                    .map(|&p| self.vp.world_to_screen(img.xform.apply(p)))
                    .collect();
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
            }
        } else {
            if let Some(bbox) = self.session.doc.selection_bbox() {
                let tl = self.vp.world_to_screen(Point2::new(bbox.min_x, bbox.min_y));
                let br = self.vp.world_to_screen(Point2::new(bbox.max_x, bbox.max_y));
                let x = tl.x;
                let y = tl.y;
                let w = br.x - tl.x;

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
        self.commit_nudge();
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
        let mut status = format!(
            "items: {total}  selected: {sel}  tool: {tool_str}  zoom: {zoom_pct}%  {dirty_str}"
        );
        if let Some(ref msg) = self.status_msg {
            status = format!("{status} | {msg}");
        }
        if let Some(el) = document.get_element_by_id("status-bar") {
            el.set_text_content(Some(&status));
        }
    }

    pub fn get_selection_handle_screen_pts(&self) -> Option<Vec<(SelHandle, Point2)>> {
        let sel = &self.session.doc.runtime.sel_items;
        if sel.is_empty() {
            return None;
        }
        let single_image = self.session.doc.single_selected_image_root();

        if let Some(img_id) = single_image {
            if let Some(InkItem::Image(img)) = self.session.doc.get_item(img_id) {
                let w = img.width;
                let h = img.height;
                let corners = [
                    Point2::new(0.0, 0.0),      // TopLeft
                    Point2::new(w, 0.0),        // TopRight
                    Point2::new(w, h),          // BottomRight
                    Point2::new(0.0, h),        // BottomLeft
                ];
                let sc: Vec<Point2> = corners
                    .iter()
                    .map(|&p| self.vp.world_to_screen(img.xform.apply(p)))
                    .collect();

                return Some(vec![
                    (SelHandle::TopLeft, sc[0]),
                    (SelHandle::TopRight, sc[1]),
                    (SelHandle::BottomRight, sc[2]),
                    (SelHandle::BottomLeft, sc[3]),
                    (SelHandle::Top, Point2::new((sc[0].x + sc[1].x) * 0.5, (sc[0].y + sc[1].y) * 0.5)),
                    (SelHandle::Right, Point2::new((sc[1].x + sc[2].x) * 0.5, (sc[1].y + sc[2].y) * 0.5)),
                    (SelHandle::Bottom, Point2::new((sc[2].x + sc[3].x) * 0.5, (sc[2].y + sc[3].y) * 0.5)),
                    (SelHandle::Left, Point2::new((sc[3].x + sc[0].x) * 0.5, (sc[3].y + sc[0].y) * 0.5)),
                ]);
            }
        }

        if let Some(bbox) = self.session.doc.selection_bbox() {
            let tl = self.vp.world_to_screen(Point2::new(bbox.min_x, bbox.min_y));
            let br = self.vp.world_to_screen(Point2::new(bbox.max_x, bbox.max_y));
            let x = tl.x;
            let y = tl.y;
            let w = br.x - tl.x;
            let h = br.y - tl.y;

            return Some(vec![
                (SelHandle::TopLeft, Point2::new(x, y)),
                (SelHandle::TopRight, Point2::new(x + w, y)),
                (SelHandle::BottomRight, Point2::new(x + w, y + h)),
                (SelHandle::BottomLeft, Point2::new(x, y + h)),
                (SelHandle::Top, Point2::new(x + w * 0.5, y)),
                (SelHandle::Right, Point2::new(x + w, y + h * 0.5)),
                (SelHandle::Bottom, Point2::new(x + w * 0.5, y + h)),
                (SelHandle::Left, Point2::new(x, y + h * 0.5)),
            ]);
        }

        None
    }

    pub fn get_handle_world_anchor(&self, handle: SelHandle) -> Point2 {
        let single_image = self.session.doc.single_selected_image_root();
        if let Some(img_id) = single_image {
            if let Some(InkItem::Image(img)) = self.session.doc.get_item(img_id) {
                let w = img.width;
                let h = img.height;
                let anchor_local = match handle {
                    SelHandle::TopLeft => Point2::new(w, h),
                    SelHandle::TopRight => Point2::new(0.0, h),
                    SelHandle::BottomRight => Point2::new(0.0, 0.0),
                    SelHandle::BottomLeft => Point2::new(w, 0.0),
                    SelHandle::Top => Point2::new(w * 0.5, h),
                    SelHandle::Right => Point2::new(0.0, h * 0.5),
                    SelHandle::Bottom => Point2::new(w * 0.5, 0.0),
                    SelHandle::Left => Point2::new(w, h * 0.5),
                };
                return img.xform.apply(anchor_local);
            }
        }

        if let Some(bbox) = self.session.doc.selection_bbox() {
            let xmin = bbox.min_x;
            let xmax = bbox.max_x;
            let ymin = bbox.min_y;
            let ymax = bbox.max_y;
            return match handle {
                SelHandle::TopLeft => Point2::new(xmax, ymax),
                SelHandle::TopRight => Point2::new(xmin, ymax),
                SelHandle::BottomRight => Point2::new(xmin, ymin),
                SelHandle::BottomLeft => Point2::new(xmax, ymin),
                SelHandle::Top => Point2::new((xmin + xmax) * 0.5, ymax),
                SelHandle::Right => Point2::new(xmin, (ymin + ymax) * 0.5),
                SelHandle::Bottom => Point2::new((xmin + xmax) * 0.5, ymin),
                SelHandle::Left => Point2::new(xmax, (ymin + ymax) * 0.5),
            };
        }

        Point2::new(0.0, 0.0)
    }

    pub fn get_scale_cursor(&self, handle: SelHandle) -> &'static str {
        if let Some(handles) = self.get_selection_handle_screen_pts() {
            let get_pt = |h: SelHandle| -> Option<Point2> {
                handles.iter().find(|&&(hd, _)| hd == h).map(|&(_, p)| p)
            };
            if let (Some(tl), Some(tr), Some(br), Some(bl), Some(t), Some(r), Some(b), Some(l)) = (
                get_pt(SelHandle::TopLeft),
                get_pt(SelHandle::TopRight),
                get_pt(SelHandle::BottomRight),
                get_pt(SelHandle::BottomLeft),
                get_pt(SelHandle::Top),
                get_pt(SelHandle::Right),
                get_pt(SelHandle::Bottom),
                get_pt(SelHandle::Left),
            ) {
                let v = match handle {
                    SelHandle::TopLeft | SelHandle::BottomRight => br - tl,
                    SelHandle::TopRight | SelHandle::BottomLeft => tr - bl,
                    SelHandle::Top | SelHandle::Bottom => b - t,
                    SelHandle::Left | SelHandle::Right => r - l,
                };
                let angle = v.y.atan2(v.x);
                return cursor_for_angle(angle);
            }
        }
        "default"
    }

    pub fn set_selection_width_preview(&mut self, w: f32) {
        self.ensure_style_preview();
        let ids = if let Some(ref preview) = self.style_preview {
            preview.stroke_ids.clone()
        } else {
            return;
        };
        for id in ids {
            if let Some(stroke) = self.session.doc.get_stroke_mut(id) {
                stroke.brush.base_w = w.clamp(0.5, 64.0);
                stroke.geom_rev += 1;
                stroke.recompute_local_bbox();
                stroke.recompute_world_bbox();
            }
        }
        self.session.doc.rebuild_runtime();
        self.redraw();

        if let Ok(ui) = WebUi::new() {
            if let Some(el) = ui.get_element("sel-width-mixed") {
                let _ = el.class_list().add_1("hidden");
            }
        }
    }

    pub fn set_selection_color_preview(&mut self, r: u8, g: u8, b: u8) {
        self.ensure_style_preview();
        let ids = if let Some(ref preview) = self.style_preview {
            preview.stroke_ids.clone()
        } else {
            return;
        };
        for id in ids {
            if let Some(stroke) = self.session.doc.get_stroke_mut(id) {
                stroke.brush.color.r = r;
                stroke.brush.color.g = g;
                stroke.brush.color.b = b;
                stroke.geom_rev += 1;
            }
        }
        self.session.doc.rebuild_runtime();
        self.redraw();

        if let Ok(ui) = WebUi::new() {
            if let Some(el) = ui.get_element("sel-color-mixed") {
                let _ = el.class_list().add_1("hidden");
            }
        }
    }

    pub fn ensure_style_preview(&mut self) {
        if self.style_preview.is_none() {
            let sel_strokes: Vec<ItemId> = self.session.doc.runtime.sel_items.iter()
                .filter(|&&id| self.session.doc.get_stroke(id).is_some())
                .cloned()
                .collect();
            
            let original_brushes: Vec<Brush> = sel_strokes.iter()
                .map(|&id| self.session.doc.get_stroke(id).unwrap().brush.clone())
                .collect();
                
            self.style_preview = Some(StylePreviewState {
                stroke_ids: sel_strokes,
                original_brushes,
            });
        }
    }

    pub fn commit_style_preview(&mut self) {
        if let Some(preview) = self.style_preview.take() {
            let mut current_brushes = Vec::new();
            let mut changed = false;
            for (i, &id) in preview.stroke_ids.iter().enumerate() {
                if let Some(stroke) = self.session.doc.get_stroke(id) {
                    current_brushes.push(stroke.brush.clone());
                    if stroke.brush != preview.original_brushes[i] {
                        changed = true;
                    }
                } else {
                    current_brushes.push(preview.original_brushes[i].clone());
                }
            }
            
            if changed && !preview.stroke_ids.is_empty() {
                let tx = InkTx::new("change selection style").push(InkOp::SetStrokeBrushes {
                    stroke_ids: preview.stroke_ids,
                    before: preview.original_brushes,
                    after: current_brushes,
                });
                self.session.do_tx(tx);
            }
            self.update_status();
            self.sync_selection_bar();
        }
    }

    pub fn cancel_style_preview(&mut self) {
        if let Some(preview) = self.style_preview.take() {
            for (i, &id) in preview.stroke_ids.iter().enumerate() {
                if let Some(stroke) = self.session.doc.get_stroke_mut(id) {
                    stroke.brush = preview.original_brushes[i].clone();
                    stroke.geom_rev += 1;
                    stroke.recompute_local_bbox();
                    stroke.recompute_world_bbox();
                }
            }
            self.session.doc.rebuild_runtime();
            self.redraw();
            self.update_status();
            self.sync_selection_bar();
        }
    }

    pub fn sync_selection_bar(&self) {
        if let Ok(ui) = WebUi::new() {
            ui.sync_selection_bar(self);
        }
    }
}

fn cursor_for_angle(angle: f32) -> &'static str {
    use std::f32::consts::PI;
    let mut norm = angle % PI;
    if norm < 0.0 {
        norm += PI;
    }
    let p8 = PI / 8.0;
    if norm < p8 || norm >= 7.0 * p8 {
        "ew-resize"
    } else if norm < 3.0 * p8 {
        "nwse-resize"
    } else if norm < 5.0 * p8 {
        "ns-resize"
    } else {
        "nesw-resize"
    }
}
