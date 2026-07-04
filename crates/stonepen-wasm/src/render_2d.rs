use std::cell::RefCell;
use std::collections::HashMap;
use stonepen_core::brush::BrushKind;
use stonepen_core::doc::{InkBackground, InkDoc};
use stonepen_core::ids::StrokeId;
use stonepen_core::point::{InkPoint, Point2};
use stonepen_core::session::InkSession;
use stonepen_core::stroke::InkStroke;
use stonepen_core::viewport::Viewport;
use web_sys::CanvasRenderingContext2d;

pub struct Renderer {
    pub ctx: CanvasRenderingContext2d,
    cache: RefCell<HashMap<(StrokeId, i64, i32), Vec<Point2>>>,
}

impl Renderer {
    pub fn new(ctx: CanvasRenderingContext2d) -> Self {
        Self {
            ctx,
            cache: RefCell::new(HashMap::new()),
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
        let dpr = vp.dpr as f64;
        let _ = self.ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
        self.clear(canvas_w, canvas_h);
        self.draw_paper(vp, canvas_w, canvas_h, &session.doc);
        self.draw_strokes(session, vp);
        if !preview.is_empty() {
            self.draw_preview(preview, &session.active_brush, vp);
        }
        if !lasso_poly.is_empty() {
            self.draw_lasso(lasso_poly, vp);
        }
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

    fn draw_strokes(&self, session: &InkSession, vp: &Viewport) {
        let visible = vp.visible_world_bbox();
        let candidates = session.doc.query_bbox(visible);
        let candidate_set: std::collections::HashSet<stonepen_core::ids::StrokeId> =
            candidates.into_iter().collect();
        for layer in &session.doc.layers {
            if !layer.visible {
                continue;
            }
            let layer_opacity = layer.opacity as f64;
            self.ctx.set_global_alpha(layer_opacity);
            for stroke in &layer.strokes {
                if !candidate_set.contains(&stroke.id) {
                    continue;
                }
                let is_sel = session.doc.runtime.sel_strokes.contains(&stroke.id);
                self.draw_stroke(stroke, vp, is_sel);
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
        stroke_id_opt: Option<(stonepen_core::ids::StrokeId, i64)>,
    ) {
        if pts.is_empty() {
            return;
        }
        let zoom_bucket = (vp.zoom.log2() * 4.0).round() as i32;
        let outline = if let Some((id, updated_at_ms)) = stroke_id_opt {
            let key = (id, updated_at_ms, zoom_bucket);
            let mut cache = self.cache.borrow_mut();
            if let Some(cached) = cache.get(&key) {
                cached.clone()
            } else {
                let centerline = stonepen_core::smooth::adaptive_catmull_rom(pts, vp.zoom);
                let radius_world = brush.base_w * 0.5;
                let radius_screen = radius_world * vp.zoom;
                let cap_segments = ((radius_screen * 1.5).round() as usize).clamp(8, 64);
                let o = stonepen_core::geom::generate_stroke_outline(&centerline, brush, cap_segments)
                    .unwrap_or_default();
                cache.insert(key, o.clone());
                o
            }
        } else {
            let centerline = stonepen_core::smooth::adaptive_catmull_rom(pts, vp.zoom);
            let radius_world = brush.base_w * 0.5;
            let radius_screen = radius_world * vp.zoom;
            let cap_segments = ((radius_screen * 1.5).round() as usize).clamp(8, 64);
            stonepen_core::geom::generate_stroke_outline(&centerline, brush, cap_segments)
                .unwrap_or_default()
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

    fn draw_stroke(&self, stroke: &InkStroke, vp: &Viewport, selected: bool) {
        self.draw_pts(
            &stroke.pts,
            &stroke.brush,
            stroke.xform,
            vp,
            Some((stroke.id, stroke.updated_at_ms)),
        );
        if selected {
            self.draw_selection_outline(stroke, vp);
        }
    }

    fn draw_selection_outline(&self, stroke: &InkStroke, vp: &Viewport) {
        let bbox = stroke.world_bbox;
        let tl = vp.world_to_screen(Point2::new(bbox.min_x, bbox.min_y));
        let br = vp.world_to_screen(Point2::new(bbox.max_x, bbox.max_y));
        let x = tl.x as f64 - 3.0;
        let y = tl.y as f64 - 3.0;
        let w = (br.x - tl.x) as f64 + 6.0;
        let h = (br.y - tl.y) as f64 + 6.0;
        self.ctx.set_stroke_style_str("rgba(60,120,220,0.7)");
        self.ctx.set_line_width(1.5);
        self.ctx.set_line_dash_offset(0.0);
        let _ = self.ctx.set_line_dash(&js_sys::Array::of2(
            &wasm_bindgen::JsValue::from(4.0),
            &wasm_bindgen::JsValue::from(3.0),
        ));
        self.ctx.stroke_rect(x, y, w, h);
        let _ = self.ctx.set_line_dash(&js_sys::Array::new());
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
}
