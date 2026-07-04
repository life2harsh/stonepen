use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use stonepen_core::brush::BrushKind;
use stonepen_core::doc::{InkBackground, InkDoc};
use stonepen_core::ids::{AssetId, StrokeId};
use stonepen_core::item::InkItem;
use stonepen_core::point::{InkPoint, Point2};
use stonepen_core::session::InkSession;
use stonepen_core::stroke::InkStroke;
use stonepen_core::viewport::Viewport;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlImageElement};

struct StrokeCacheEntry {
    geom_rev: u64,
    buckets: HashMap<i32, Rc<Vec<Point2>>>,
    recent_buckets: VecDeque<i32>,
}

impl StrokeCacheEntry {
    fn new(geom_rev: u64) -> Self {
        Self {
            geom_rev,
            buckets: HashMap::new(),
            recent_buckets: VecDeque::new(),
        }
    }

    fn insert(&mut self, zoom_bucket: i32, outline: Rc<Vec<Point2>>) {
        if !self.buckets.contains_key(&zoom_bucket) {
            if self.recent_buckets.len() >= 4 {
                if let Some(old_bucket) = self.recent_buckets.pop_front() {
                    self.buckets.remove(&old_bucket);
                }
            }
            self.recent_buckets.push_back(zoom_bucket);
        }
        self.buckets.insert(zoom_bucket, outline);
    }
}

struct ImageCacheEntry {
    img: HtmlImageElement,
    _closure: wasm_bindgen::closure::Closure<dyn FnMut()>,
}

pub struct Renderer {
    pub ctx: CanvasRenderingContext2d,
    stroke_cache: RefCell<HashMap<StrokeId, StrokeCacheEntry>>,
    image_cache: RefCell<HashMap<AssetId, ImageCacheEntry>>,
}

impl Renderer {
    pub fn new(ctx: CanvasRenderingContext2d) -> Self {
        Self {
            ctx,
            stroke_cache: RefCell::new(HashMap::new()),
            image_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn render(
        &self,
        session: &InkSession,
        vp: &Viewport,
        preview: &[InkPoint],
        lasso_poly: &[Point2],
        canvas_w: f64,
        canvas_h: f64,
    ) {
        {
            let mut sc = self.stroke_cache.borrow_mut();
            sc.retain(|sid, _| session.doc.get_stroke(*sid).is_some());
        }
        {
            let mut ic = self.image_cache.borrow_mut();
            ic.retain(|aid, _| session.doc.get_asset(*aid).is_some());
        }
        let dpr = vp.dpr as f64;
        let _ = self.ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
        self.clear(canvas_w, canvas_h);
        self.draw_paper(vp, canvas_w, canvas_h, &session.doc);
        self.draw_items(session, vp);
        if !preview.is_empty() {
            self.draw_preview(preview, &session.active_brush, vp);
        }
        if !lasso_poly.is_empty() {
            self.draw_lasso(lasso_poly, vp);
        }
        self.draw_selection_overlay(session, vp);
    }

    fn clear(&self, w: f64, h: f64) {
        self.ctx.clear_rect(0.0, 0.0, w, h);
    }

    fn draw_paper(&self, vp: &Viewport, canvas_w: f64, canvas_h: f64, doc: &InkDoc) {
        self.ctx.set_fill_style_str("#f8f6f0");
        self.ctx.fill_rect(0.0, 0.0, canvas_w, canvas_h);
        match doc.background {
            InkBackground::Dots => self.draw_dots(vp, canvas_w, canvas_h),
            InkBackground::Grid => self.draw_grid(vp, canvas_w, canvas_h),
            InkBackground::Ruled => self.draw_ruled(vp, canvas_w, canvas_h),
            InkBackground::Plain => {}
        }
    }

    fn draw_dots(&self, vp: &Viewport, _canvas_w: f64, _canvas_h: f64) {
        let spacing = 24.0f32;
        let world_bbox = vp.visible_world_bbox();
        let start_x = (world_bbox.min_x / spacing).floor() * spacing;
        let start_y = (world_bbox.min_y / spacing).floor() * spacing;
        self.ctx.set_fill_style_str("#c8c0b8");
        let mut wx = start_x;
        while wx < world_bbox.max_x + spacing {
            let mut wy = start_y;
            while wy < world_bbox.max_y + spacing {
                let sp = vp.world_to_screen(Point2::new(wx, wy));
                self.ctx.begin_path();
                let _ = self
                    .ctx
                    .arc(sp.x as f64, sp.y as f64, 1.2, 0.0, std::f64::consts::TAU);
                self.ctx.fill();
                wy += spacing;
            }
            wx += spacing;
        }
    }

    fn draw_grid(&self, vp: &Viewport, canvas_w: f64, canvas_h: f64) {
        let spacing = 24.0f32;
        let world_bbox = vp.visible_world_bbox();
        self.ctx.set_stroke_style_str("#ddd8d0");
        self.ctx.set_line_width(0.5);
        let start_x = (world_bbox.min_x / spacing).floor() * spacing;
        let mut wx = start_x;
        while wx < world_bbox.max_x + spacing {
            let sx = vp.world_to_screen(Point2::new(wx, 0.0)).x as f64;
            self.ctx.begin_path();
            self.ctx.move_to(sx, 0.0);
            self.ctx.line_to(sx, canvas_h);
            self.ctx.stroke();
            wx += spacing;
        }
        let start_y = (world_bbox.min_y / spacing).floor() * spacing;
        let mut wy = start_y;
        while wy < world_bbox.max_y + spacing {
            let sy = vp.world_to_screen(Point2::new(0.0, wy)).y as f64;
            self.ctx.begin_path();
            self.ctx.move_to(0.0, sy);
            self.ctx.line_to(canvas_w, sy);
            self.ctx.stroke();
            wy += spacing;
        }
    }

    fn draw_ruled(&self, vp: &Viewport, canvas_w: f64, _canvas_h: f64) {
        let spacing = 32.0f32;
        let world_bbox = vp.visible_world_bbox();
        self.ctx.set_stroke_style_str("#d0c8c0");
        self.ctx.set_line_width(0.75);
        let start_y = (world_bbox.min_y / spacing).floor() * spacing;
        let mut wy = start_y;
        while wy < world_bbox.max_y + spacing {
            let sy = vp.world_to_screen(Point2::new(0.0, wy)).y as f64;
            self.ctx.begin_path();
            self.ctx.move_to(0.0, sy);
            self.ctx.line_to(canvas_w, sy);
            self.ctx.stroke();
            wy += spacing;
        }
    }

    fn draw_items(&self, session: &InkSession, vp: &Viewport) {
        let visible = vp.visible_world_bbox();
        let candidates = session.doc.query_bbox(visible);
        let candidate_set: std::collections::HashSet<stonepen_core::ids::ItemId> =
            candidates.into_iter().collect();
        for layer in &session.doc.layers {
            if !layer.visible {
                continue;
            }
            let layer_opacity = layer.opacity as f64;
            self.ctx.set_global_alpha(layer_opacity);
            for item in &layer.items {
                if !candidate_set.contains(&item.id()) {
                    continue;
                }
                match item {
                    InkItem::Stroke(stroke) => {
                        self.draw_stroke(stroke, vp);
                    }
                    InkItem::Image(img) => {
                        if let Some(asset) = session.doc.get_asset(img.asset_id) {
                            let mut cache = self.image_cache.borrow_mut();
                            let entry = cache.entry(img.asset_id).or_insert_with(|| {
                                let html_img = HtmlImageElement::new().unwrap();
                                let array = unsafe { js_sys::Uint8Array::view(&asset.bytes) };
                                let parts = js_sys::Array::new();
                                parts.push(&array);
                                let property_bag = web_sys::BlobPropertyBag::new();
                                property_bag.set_type(&asset.mime);
                                let blob = web_sys::Blob::new_with_blob_sequence_and_options(
                                    &parts,
                                    &property_bag,
                                )
                                .unwrap();
                                let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
                                html_img.set_src(&url);

                                let canvas_clone = self.ctx.canvas().unwrap();
                                let closure =
                                    wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                                        let _ = canvas_clone.dispatch_event(
                                            &web_sys::Event::new("redraw").unwrap(),
                                        );
                                    })
                                        as Box<dyn FnMut()>);
                                html_img.set_onload(Some(closure.as_ref().unchecked_ref()));
                                ImageCacheEntry {
                                    img: html_img,
                                    _closure: closure,
                                }
                            });

                            if entry.img.complete() {
                                let _ = self.ctx.save();
                                let m = img.xform;
                                // Convert world xform to screen coordinates and apply
                                let sp = vp.world_to_screen(Point2::new(m.tx, m.ty));
                                let s_scale = vp.zoom;
                                let _ = self.ctx.translate(sp.x as f64, sp.y as f64);
                                let _ = self.ctx.transform(
                                    (m.a) as f64,
                                    (m.b) as f64,
                                    (m.c) as f64,
                                    (m.d) as f64,
                                    0.0,
                                    0.0,
                                );
                                let _ = self.ctx.scale(s_scale as f64, s_scale as f64);
                                let _ = self.ctx.draw_image_with_html_image_element_and_dw_and_dh(
                                    &entry.img,
                                    0.0,
                                    0.0,
                                    img.width as f64,
                                    img.height as f64,
                                );
                                let _ = self.ctx.restore();
                            } else {
                                // Draw dashed placeholder while loading
                                let corners = [
                                    img.xform.apply(Point2::new(0.0, 0.0)),
                                    img.xform.apply(Point2::new(img.width, 0.0)),
                                    img.xform.apply(Point2::new(img.width, img.height)),
                                    img.xform.apply(Point2::new(0.0, img.height)),
                                ];
                                let sps: Vec<Point2> =
                                    corners.iter().map(|&p| vp.world_to_screen(p)).collect();
                                self.ctx.set_stroke_style_str("rgba(160,160,160,0.5)");
                                self.ctx.set_line_width(1.0);
                                let _ = self.ctx.set_line_dash(&js_sys::Array::of2(
                                    &wasm_bindgen::JsValue::from(4.0),
                                    &wasm_bindgen::JsValue::from(4.0),
                                ));
                                self.ctx.begin_path();
                                self.ctx.move_to(sps[0].x as f64, sps[0].y as f64);
                                for p in sps.iter().skip(1) {
                                    self.ctx.line_to(p.x as f64, p.y as f64);
                                }
                                self.ctx.close_path();
                                self.ctx.stroke();
                                let _ = self.ctx.set_line_dash(&js_sys::Array::new());
                            }
                        }
                    }
                }
            }
        }
        self.ctx.set_global_alpha(1.0);
    }

    fn stroke_style_str(brush: &stonepen_core::brush::Brush) -> String {
        let color = &brush.color;
        let opacity = match brush.kind {
            BrushKind::Highlighter => (brush.opacity * 0.6).min(0.55) as f64,
            _ => brush.opacity as f64,
        };
        format!("rgba({},{},{},{:.3})", color.r, color.g, color.b, opacity)
    }

    fn draw_pts(
        &self,
        pts: &[InkPoint],
        brush: &stonepen_core::brush::Brush,
        xform: stonepen_core::xform::Xform2D,
        vp: &Viewport,
        stroke_id_opt: Option<(stonepen_core::ids::StrokeId, u64)>,
    ) {
        if pts.is_empty() {
            return;
        }
        let effective_zoom = vp.zoom * stonepen_core::geom::xform_scale(xform);
        let zoom_bucket = (effective_zoom.log2() * 4.0).round() as i32;
        let outline = if let Some((id, geom_rev)) = stroke_id_opt {
            let mut cache = self.stroke_cache.borrow_mut();
            let entry = cache
                .entry(id)
                .or_insert_with(|| StrokeCacheEntry::new(geom_rev));
            if entry.geom_rev != geom_rev {
                entry.geom_rev = geom_rev;
                entry.buckets.clear();
                entry.recent_buckets.clear();
            }
            if let Some(cached) = entry.buckets.get(&zoom_bucket) {
                cached.clone()
            } else {
                let centerline = stonepen_core::smooth::adaptive_catmull_rom(pts, effective_zoom);
                let radius_world = brush.base_w * 0.5;
                let radius_screen = radius_world * effective_zoom;
                let cap_segments = ((radius_screen * 1.5).round() as usize).clamp(8, 64);
                let o = Rc::new(
                    stonepen_core::geom::generate_stroke_outline(&centerline, brush, cap_segments)
                        .unwrap_or_default(),
                );
                entry.insert(zoom_bucket, o.clone());
                o
            }
        } else {
            let centerline = stonepen_core::smooth::adaptive_catmull_rom(pts, vp.zoom);
            let radius_world = brush.base_w * 0.5;
            let radius_screen = radius_world * vp.zoom;
            let cap_segments = ((radius_screen * 1.5).round() as usize).clamp(8, 64);
            Rc::new(
                stonepen_core::geom::generate_stroke_outline(&centerline, brush, cap_segments)
                    .unwrap_or_default(),
            )
        };
        if outline.is_empty() {
            return;
        }
        let style = Self::stroke_style_str(brush);
        self.ctx.set_fill_style_str(&style);
        self.ctx.begin_path();
        let p0 = xform.apply(outline[0]);
        let sp0 = vp.world_to_screen(p0);
        self.ctx.move_to(sp0.x as f64, sp0.y as f64);
        for pt in outline.iter().skip(1) {
            let p = xform.apply(*pt);
            let sp = vp.world_to_screen(p);
            self.ctx.line_to(sp.x as f64, sp.y as f64);
        }
        self.ctx.close_path();
        self.ctx.fill();
    }

    fn draw_stroke(&self, stroke: &InkStroke, vp: &Viewport) {
        self.draw_pts(
            &stroke.pts,
            &stroke.brush,
            stroke.xform,
            vp,
            Some((stroke.id, stroke.geom_rev)),
        );
    }

    fn draw_preview(&self, pts: &[InkPoint], brush: &stonepen_core::brush::Brush, vp: &Viewport) {
        self.draw_pts(
            pts,
            brush,
            stonepen_core::xform::Xform2D::identity(),
            vp,
            None,
        );
    }

    fn draw_lasso(&self, poly: &[Point2], vp: &Viewport) {
        if poly.len() < 2 {
            return;
        }
        self.ctx.set_stroke_style_str("rgba(60,120,220,0.8)");
        self.ctx.set_fill_style_str("rgba(60,120,220,0.08)");
        self.ctx.set_line_width(1.5);
        let _ = self.ctx.set_line_dash(&js_sys::Array::of2(
            &wasm_bindgen::JsValue::from(5.0),
            &wasm_bindgen::JsValue::from(3.0),
        ));
        self.ctx.begin_path();
        let sp0 = vp.world_to_screen(poly[0]);
        self.ctx.move_to(sp0.x as f64, sp0.y as f64);
        for p in poly.iter().skip(1) {
            let sp = vp.world_to_screen(*p);
            self.ctx.line_to(sp.x as f64, sp.y as f64);
        }
        self.ctx.close_path();
        self.ctx.fill();
        self.ctx.stroke();
        let _ = self.ctx.set_line_dash(&js_sys::Array::new());
    }

    fn draw_selection_overlay(&self, session: &InkSession, vp: &Viewport) {
        let bbox = match session.doc.selection_bbox() {
            Some(b) => b,
            None => return,
        };
        let tl = vp.world_to_screen(Point2::new(bbox.min_x, bbox.min_y));
        let br = vp.world_to_screen(Point2::new(bbox.max_x, bbox.max_y));
        let x = tl.x as f64;
        let y = tl.y as f64;
        let w = (br.x - tl.x) as f64;
        let h = (br.y - tl.y) as f64;

        // Draw bounding box
        self.ctx.set_stroke_style_str("rgba(60,120,220,0.85)");
        self.ctx.set_line_width(1.5);
        self.ctx.stroke_rect(x, y, w, h);

        // Draw handles in screen space
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_stroke_style_str("#3b78dc");
        self.ctx.set_line_width(2.0);

        let h_size = 10.0;
        let h_half = h_size * 0.5;

        let draw_corner = |cx: f64, cy: f64| {
            self.ctx.fill_rect(cx - h_half, cy - h_half, h_size, h_size);
            self.ctx
                .stroke_rect(cx - h_half, cy - h_half, h_size, h_size);
        };

        draw_corner(x, y);
        draw_corner(x + w, y);
        draw_corner(x + w, y + h);
        draw_corner(x, y + h);

        // Draw rotation handle
        let rx = x + w * 0.5;
        let ry = y - 25.0;
        self.ctx.begin_path();
        self.ctx.move_to(rx, y);
        self.ctx.line_to(rx, ry);
        self.ctx.stroke();

        self.ctx.begin_path();
        let _ = self.ctx.arc(rx, ry, 5.0, 0.0, std::f64::consts::TAU);
        self.ctx.fill();
        self.ctx.stroke();
    }
}
