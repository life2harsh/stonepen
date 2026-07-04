pub mod bbox;
pub mod brush;
pub mod color;
pub mod doc;
pub mod export_json;
pub mod export_svg;
pub mod geom;
pub mod hit;
pub mod ids;
pub mod layer;
pub mod ops;
pub mod point;
pub mod resample;
pub mod runtime;
pub mod sel;
pub mod session;
pub mod smooth;
pub mod spatial;
pub mod stroke;
pub mod viewport;
pub mod xform;

pub use bbox::BBox;
pub use brush::{stroke_w, Brush, BrushKind};
pub use color::ColorRgba;
pub use doc::{InkBackground, InkDoc};
pub use geom::{
    compute_conservative_stroke_bbox, compute_outline_bbox, generate_stroke_outline, xform_scale,
};
pub use ids::{BrushId, DocId, LayerId, StrokeId};
pub use layer::InkLayer;
pub use ops::{InkOp, InkTx, UndoRedo};
pub use point::{InkPoint, Point2, PointerKind, Vec2};
pub use runtime::{IndexedStroke, InkRuntime, StrokeAddress};
pub use session::{InkError, InkSession, Tool};
pub use smooth::adaptive_catmull_rom;
pub use stroke::{InkStroke, StrokeBuilder};
pub use viewport::Viewport;
pub use xform::Xform2D;

#[cfg(test)]
mod tests {
    use super::*;
    use geom::*;

    fn make_ink_point(x: f32, y: f32) -> InkPoint {
        InkPoint {
            x,
            y,
            t_ms: 0.0,
            press: 1.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            twist: 0.0,
            pointer_type: PointerKind::Pen,
        }
    }

    fn make_stroke_in_doc(doc: &mut InkDoc, pts: Vec<InkPoint>) -> StrokeId {
        let brush = Brush::default_pen();
        let local_bbox =
            compute_bbox(&pts, brush.base_w * 0.5).unwrap_or(BBox::new(0.0, 0.0, 1.0, 1.0));
        let xform = Xform2D::identity();
        let world_bbox = xform.apply_bbox(local_bbox);
        let stroke = InkStroke {
            id: StrokeId::new(),
            brush,
            raw_pts: pts.clone(),
            pts,
            local_bbox,
            world_bbox,
            xform,
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let sid = stroke.id;
        let layer_id = doc.active_layer_id;
        doc.add_stroke(layer_id, stroke);
        sid
    }

    #[test]
    fn test_bbox_computation() {
        let pts = vec![
            make_ink_point(1.0, 2.0),
            make_ink_point(5.0, 3.0),
            make_ink_point(3.0, 7.0),
        ];
        let bbox = compute_bbox(&pts, 0.0).unwrap();
        assert!((bbox.min_x - 1.0).abs() < 1e-5);
        assert!((bbox.min_y - 2.0).abs() < 1e-5);
        assert!((bbox.max_x - 5.0).abs() < 1e-5);
        assert!((bbox.max_y - 7.0).abs() < 1e-5);
    }

    #[test]
    fn test_bbox_computation_with_radius() {
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 10.0)];
        let bbox = compute_bbox(&pts, 2.0).unwrap();
        assert!((bbox.min_x - (-2.0)).abs() < 1e-5);
        assert!((bbox.max_x - 12.0).abs() < 1e-5);
    }

    #[test]
    fn test_bbox_empty() {
        let bbox = compute_bbox(&[], 0.0);
        assert!(bbox.is_none());
    }

    #[test]
    fn test_bbox_intersection_overlapping() {
        let a = BBox::new(0.0, 0.0, 10.0, 10.0);
        let b = BBox::new(5.0, 5.0, 15.0, 15.0);
        assert!(bbox_intersects(a, b));
    }

    #[test]
    fn test_bbox_intersection_touching() {
        let a = BBox::new(0.0, 0.0, 5.0, 5.0);
        let b = BBox::new(5.0, 5.0, 10.0, 10.0);
        assert!(bbox_intersects(a, b));
    }

    #[test]
    fn test_bbox_intersection_separate() {
        let a = BBox::new(0.0, 0.0, 4.0, 4.0);
        let b = BBox::new(5.0, 5.0, 10.0, 10.0);
        assert!(!bbox_intersects(a, b));
    }

    #[test]
    fn test_bbox_contains_point() {
        let bbox = BBox::new(0.0, 0.0, 10.0, 10.0);
        assert!(bbox_contains_point(bbox, Point2::new(5.0, 5.0)));
        assert!(!bbox_contains_point(bbox, Point2::new(11.0, 5.0)));
    }

    #[test]
    fn test_distance_to_segment_midpoint() {
        let pos = Point2::new(5.0, 5.0);
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(10.0, 0.0);
        let d = distance_to_segment(pos, a, b);
        assert!((d - 5.0).abs() < 1e-4);
    }

    #[test]
    fn test_distance_to_segment_endpoint() {
        let pos = Point2::new(-1.0, 0.0);
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(10.0, 0.0);
        let d = distance_to_segment(pos, a, b);
        assert!((d - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_distance_to_degenerate_segment() {
        let pos = Point2::new(3.0, 4.0);
        let a = Point2::new(0.0, 0.0);
        let d = distance_to_segment(pos, a, a);
        assert!((d - 5.0).abs() < 1e-4);
    }

    #[test]
    fn test_point_in_polygon_inside() {
        let polygon = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(Point2::new(5.0, 5.0), &polygon));
    }

    #[test]
    fn test_point_in_polygon_outside() {
        let polygon = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        assert!(!point_in_polygon(Point2::new(15.0, 5.0), &polygon));
    }

    #[test]
    fn test_point_in_polygon_degenerate() {
        let polygon = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)];
        assert!(!point_in_polygon(Point2::new(0.5, 0.5), &polygon));
    }

    #[test]
    fn test_polyline_hit() {
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 0.0)];
        assert!(polyline_hit(&pts, Point2::new(5.0, 2.0), 3.0));
        assert!(!polyline_hit(&pts, Point2::new(5.0, 10.0), 3.0));
    }

    #[test]
    fn test_polyline_hit_single_point() {
        let pts = vec![make_ink_point(5.0, 5.0)];
        assert!(polyline_hit(&pts, Point2::new(5.0, 5.0), 1.0));
        assert!(!polyline_hit(&pts, Point2::new(10.0, 10.0), 1.0));
    }

    #[test]
    fn test_stroke_hit() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let pts = vec![make_ink_point(10.0, 10.0), make_ink_point(100.0, 10.0)];
        make_stroke_in_doc(&mut doc, pts);
        let _layer_id = doc.active_layer_id;
        let stroke = &doc.active_layer().unwrap().strokes[0];
        assert!(hit::stroke_hit(stroke, Point2::new(50.0, 10.0), 5.0));
        assert!(!hit::stroke_hit(stroke, Point2::new(50.0, 100.0), 5.0));
    }

    #[test]
    fn test_rtree_query() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let pts1 = vec![make_ink_point(10.0, 10.0), make_ink_point(50.0, 10.0)];
        let pts2 = vec![make_ink_point(200.0, 200.0), make_ink_point(250.0, 200.0)];
        let s1 = make_stroke_in_doc(&mut doc, pts1);
        let s2 = make_stroke_in_doc(&mut doc, pts2);
        let candidates = doc.query_bbox(BBox::new(0.0, 0.0, 100.0, 100.0));
        assert!(candidates.contains(&s1));
        assert!(!candidates.contains(&s2));
    }

    #[test]
    fn test_rtree_query_no_results() {
        let doc = InkDoc::new(800.0, 600.0);
        let candidates = doc.query_bbox(BBox::new(0.0, 0.0, 100.0, 100.0));
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_lasso_selection() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let pts = vec![make_ink_point(50.0, 50.0), make_ink_point(60.0, 50.0)];
        let sid = make_stroke_in_doc(&mut doc, pts);
        let polygon = vec![
            Point2::new(40.0, 40.0),
            Point2::new(80.0, 40.0),
            Point2::new(80.0, 80.0),
            Point2::new(40.0, 80.0),
        ];
        let sel = doc.select_lasso(&polygon);
        assert!(sel.contains(&sid));
        assert!(doc.runtime.sel_strokes.contains(&sid));
    }

    #[test]
    fn test_lasso_selection_outside() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let pts = vec![make_ink_point(300.0, 300.0), make_ink_point(350.0, 300.0)];
        let sid = make_stroke_in_doc(&mut doc, pts);
        let polygon = vec![
            Point2::new(0.0, 0.0),
            Point2::new(50.0, 0.0),
            Point2::new(50.0, 50.0),
            Point2::new(0.0, 50.0),
        ];
        let sel = doc.select_lasso(&polygon);
        assert!(!sel.contains(&sid));
    }

    #[test]
    fn test_runtime_rebuild() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let pts = vec![make_ink_point(10.0, 10.0), make_ink_point(20.0, 20.0)];
        let sid = make_stroke_in_doc(&mut doc, pts);
        assert!(doc.runtime.stroke_pos.contains_key(&sid));
        doc.rebuild_runtime();
        assert!(doc.runtime.stroke_pos.contains_key(&sid));
    }

    #[test]
    fn test_delete_updates_index() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let pts = vec![make_ink_point(10.0, 10.0), make_ink_point(20.0, 10.0)];
        let sid = make_stroke_in_doc(&mut doc, pts);
        assert!(doc.runtime.stroke_pos.contains_key(&sid));
        doc.delete_stroke(sid);
        assert!(!doc.runtime.stroke_pos.contains_key(&sid));
        let candidates = doc.query_bbox(BBox::new(0.0, 0.0, 100.0, 100.0));
        assert!(!candidates.contains(&sid));
    }

    #[test]
    fn test_delete_multiple_updates_index() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let s1 = make_stroke_in_doc(
            &mut doc,
            vec![make_ink_point(10.0, 10.0), make_ink_point(20.0, 10.0)],
        );
        let s2 = make_stroke_in_doc(
            &mut doc,
            vec![make_ink_point(30.0, 10.0), make_ink_point(40.0, 10.0)],
        );
        doc.delete_strokes(&[s1, s2]);
        assert!(!doc.runtime.stroke_pos.contains_key(&s1));
        assert!(!doc.runtime.stroke_pos.contains_key(&s2));
    }

    #[test]
    fn test_undo_redo_add_stroke() {
        let mut session = InkSession::new(800.0, 600.0);
        let pts = vec![make_ink_point(10.0, 10.0), make_ink_point(20.0, 10.0)];
        let brush = Brush::default_pen();
        let local_bbox = compute_bbox(&pts, 2.0).unwrap();
        let xform = Xform2D::identity();
        let world_bbox = xform.apply_bbox(local_bbox);
        let stroke = InkStroke {
            id: StrokeId::new(),
            brush,
            raw_pts: pts.clone(),
            pts,
            local_bbox,
            world_bbox,
            xform,
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        session.add_stroke(stroke);
        assert_eq!(session.doc.active_layer().unwrap().strokes.len(), 1);
        session.undo();
        assert_eq!(session.doc.active_layer().unwrap().strokes.len(), 0);
        session.redo();
        assert_eq!(session.doc.active_layer().unwrap().strokes.len(), 1);
    }

    #[test]
    fn test_undo_redo_delete_strokes() {
        let mut session = InkSession::new(800.0, 600.0);
        let pts = vec![make_ink_point(10.0, 10.0), make_ink_point(20.0, 10.0)];
        let brush = Brush::default_pen();
        let local_bbox = compute_bbox(&pts, 2.0).unwrap();
        let xform = Xform2D::identity();
        let world_bbox = xform.apply_bbox(local_bbox);
        let stroke = InkStroke {
            id: StrokeId::new(),
            brush,
            raw_pts: pts.clone(),
            pts,
            local_bbox,
            world_bbox,
            xform,
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let _sid = stroke.id;
        session.add_stroke(stroke);
        session.erase_at(Point2::new(15.0, 10.0), 10.0);
        assert_eq!(session.doc.active_layer().unwrap().strokes.len(), 0);
        session.undo();
        assert_eq!(session.doc.active_layer().unwrap().strokes.len(), 1);
    }

    #[test]
    fn test_undo_redo_clear_layer() {
        let mut session = InkSession::new(800.0, 600.0);
        for i in 0..3 {
            let pts = vec![
                make_ink_point(i as f32 * 10.0, 0.0),
                make_ink_point(i as f32 * 10.0 + 5.0, 0.0),
            ];
            let brush = Brush::default_pen();
            let local_bbox = compute_bbox(&pts, 2.0).unwrap();
            let xform = Xform2D::identity();
            let world_bbox = xform.apply_bbox(local_bbox);
            let stroke = InkStroke {
                id: StrokeId::new(),
                brush,
                raw_pts: pts.clone(),
                pts,
                local_bbox,
                world_bbox,
                xform,
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            };
            session.add_stroke(stroke);
        }
        assert_eq!(session.doc.active_layer().unwrap().strokes.len(), 3);
        session.clear_active_layer();
        assert_eq!(session.doc.active_layer().unwrap().strokes.len(), 0);
        session.undo();
        assert_eq!(session.doc.active_layer().unwrap().strokes.len(), 3);
        session.redo();
        assert_eq!(session.doc.active_layer().unwrap().strokes.len(), 0);
    }

    #[test]
    fn test_json_roundtrip() {
        let mut session = InkSession::new(800.0, 600.0);
        let pts = vec![make_ink_point(10.0, 10.0), make_ink_point(20.0, 10.0)];
        let brush = Brush::default_pen();
        let local_bbox = compute_bbox(&pts, 2.0).unwrap();
        let xform = Xform2D::identity();
        let world_bbox = xform.apply_bbox(local_bbox);
        let stroke = InkStroke {
            id: StrokeId::new(),
            brush,
            raw_pts: pts.clone(),
            pts,
            local_bbox,
            world_bbox,
            xform,
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let sid = stroke.id;
        session.add_stroke(stroke);
        let json = session.export_json().unwrap();
        let restored = InkSession::import_json(&json).unwrap();
        assert_eq!(restored.doc.layers.len(), 1);
        let layer = restored.doc.active_layer().unwrap();
        assert_eq!(layer.strokes.len(), 1);
        assert_eq!(layer.strokes[0].id, sid);
    }

    #[test]
    fn test_json_roundtrip_empty_doc() {
        let session = InkSession::new(1024.0, 768.0);
        let json = session.export_json().unwrap();
        let restored = InkSession::import_json(&json).unwrap();
        assert_eq!(restored.doc.layers.len(), 1);
        assert_eq!(restored.doc.active_layer().unwrap().strokes.len(), 0);
    }

    #[test]
    fn test_svg_export_structure() {
        let mut session = InkSession::new(800.0, 600.0);
        let pts = vec![make_ink_point(10.0, 10.0), make_ink_point(50.0, 50.0)];
        let brush = Brush::default_pen();
        let local_bbox = compute_bbox(&pts, 2.0).unwrap();
        let xform = Xform2D::identity();
        let world_bbox = xform.apply_bbox(local_bbox);
        let stroke = InkStroke {
            id: StrokeId::new(),
            brush,
            raw_pts: pts.clone(),
            pts,
            local_bbox,
            world_bbox,
            xform,
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        session.add_stroke(stroke);
        let svg = session.export_svg().unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<path"));
        assert!(svg.contains("M "));
    }

    #[test]
    fn test_svg_export_empty() {
        let session = InkSession::new(800.0, 600.0);
        let svg = session.export_svg().unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_viewport_roundtrip() {
        let vp = Viewport::new(800.0, 600.0);
        let wp = Point2::new(100.0, 200.0);
        let sp = vp.world_to_screen(wp);
        let wp2 = vp.screen_to_world(sp);
        assert!((wp2.x - wp.x).abs() < 1e-3);
        assert!((wp2.y - wp.y).abs() < 1e-3);
    }

    #[test]
    fn test_viewport_visible_bbox() {
        let vp = Viewport::new(800.0, 600.0);
        let bbox = vp.visible_world_bbox();
        assert!(bbox.min_x <= 0.0);
        assert!(bbox.min_y <= 0.0);
        assert!(bbox.max_x >= 800.0);
        assert!(bbox.max_y >= 600.0);
    }

    #[test]
    fn test_stroke_builder_finish() {
        let brush = Brush::default_pen();
        let mut builder = StrokeBuilder::new(brush);
        for i in 0..10 {
            builder.push(make_ink_point(i as f32 * 5.0, 0.0));
        }
        let stroke = builder.finish(0);
        assert!(stroke.is_some());
        let s = stroke.unwrap();
        assert!(!s.pts.is_empty());
        assert!(!s.raw_pts.is_empty());
    }

    #[test]
    fn test_stroke_builder_invariants() {
        let brush = Brush::default_pen();
        let mut builder = StrokeBuilder::new(brush);

        let p1 = make_ink_point(10.0, 10.0);
        let mut p2 = make_ink_point(10.05, 10.0);
        p2.pointer_type = PointerKind::Pen;
        p2.press = 0.0; // 0 pressure pen release

        builder.push(p1);
        builder.push(p2); // Duplicate of first point, should be ignored
        assert_eq!(builder.raw_pts.len(), 1);

        let mut p3 = make_ink_point(20.0, 10.0);
        p3.pointer_type = PointerKind::Pen;
        p3.press = 0.8;
        builder.push(p3);
        assert_eq!(builder.raw_pts.len(), 2);

        let mut p4 = make_ink_point(30.0, 10.0);
        p4.pointer_type = PointerKind::Pen;
        p4.press = 0.0; // 0 pressure pen release
        builder.push(p4);
        assert_eq!(builder.raw_pts.len(), 3);
        assert_eq!(builder.raw_pts[2].press, 0.8); // Should normalize to last valid pressure

        let preview_pts = builder.preview_pts().to_vec();
        let s = builder.finish(0).unwrap();
        assert_eq!(s.pts.len(), preview_pts.len());
        for (a, b) in s.pts.iter().zip(preview_pts.iter()) {
            assert!((a.x - b.x).abs() < 1e-4);
            assert!((a.y - b.y).abs() < 1e-4);
            assert!((a.press - b.press).abs() < 1e-4);
        }
    }

    #[test]
    fn test_stroke_builder_empty_returns_none() {
        let brush = Brush::default_pen();
        let builder = StrokeBuilder::new(brush);
        assert!(builder.finish(0).is_none());
    }

    #[test]
    fn test_clear_layer_removes_from_rtree() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let pts = vec![make_ink_point(10.0, 10.0), make_ink_point(20.0, 10.0)];
        make_stroke_in_doc(&mut doc, pts);
        let layer_id = doc.active_layer_id;
        doc.clear_layer(layer_id);
        let candidates = doc.query_bbox(BBox::new(0.0, 0.0, 100.0, 100.0));
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_xform_identity() {
        let xf = Xform2D::identity();
        let p = Point2::new(3.0, 4.0);
        let out = xf.apply(p);
        assert!((out.x - p.x).abs() < 1e-5);
        assert!((out.y - p.y).abs() < 1e-5);
    }

    #[test]
    fn test_xform_translate() {
        let xf = Xform2D::translate(10.0, 20.0);
        let p = Point2::new(1.0, 1.0);
        let out = xf.apply(p);
        assert!((out.x - 11.0).abs() < 1e-5);
        assert!((out.y - 21.0).abs() < 1e-5);
    }

    #[test]
    fn test_resample_preserves_endpoints() {
        let pts = vec![
            make_ink_point(0.0, 0.0),
            make_ink_point(5.0, 0.0),
            make_ink_point(10.0, 0.0),
        ];
        let resampled = resample::resample_by_distance(&pts, 2.0);
        assert!(!resampled.is_empty());
        assert_eq!(resampled[0].x, 0.0);
        assert_eq!(resampled[resampled.len() - 1].x, 10.0);
    }

    #[test]
    fn test_smooth_preserves_endpoints() {
        let pts = vec![
            make_ink_point(0.0, 0.0),
            make_ink_point(5.0, 10.0),
            make_ink_point(10.0, 0.0),
        ];
        let smoothed = smooth::smooth_pts(&pts, 0.5);
        assert_eq!(smoothed[0].x, 0.0);
        assert_eq!(smoothed[smoothed.len() - 1].x, 10.0);
    }

    #[test]
    fn test_redo_stack_cleared_on_new_tx() {
        let mut session = InkSession::new(800.0, 600.0);
        let make_s = || {
            let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 0.0)];
            let brush = Brush::default_pen();
            let local_bbox = compute_bbox(&pts, 2.0).unwrap();
            let xform = Xform2D::identity();
            let world_bbox = xform.apply_bbox(local_bbox);
            InkStroke {
                id: StrokeId::new(),
                brush,
                raw_pts: pts.clone(),
                pts,
                local_bbox,
                world_bbox,
                xform,
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }
        };
        session.add_stroke(make_s());
        session.undo();
        assert_eq!(session.undo_redo.redo_stack.len(), 1);
        session.add_stroke(make_s());
        assert_eq!(session.undo_redo.redo_stack.len(), 0);
    }

    #[test]
    fn test_uniform_resampling() {
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 0.0)];
        let resampled = resample::resample_by_distance(&pts, 2.0);
        assert_eq!(resampled.len(), 6);
        assert!((resampled[0].x - 0.0).abs() < 1e-4);
        assert!((resampled[1].x - 2.0).abs() < 1e-4);
        assert!((resampled[2].x - 4.0).abs() < 1e-4);
        assert!((resampled[3].x - 6.0).abs() < 1e-4);
        assert!((resampled[4].x - 8.0).abs() < 1e-4);
        assert!((resampled[5].x - 10.0).abs() < 1e-4);
    }

    #[test]
    fn test_resampling_multisegment_carry() {
        let pts = vec![
            make_ink_point(0.0, 0.0),
            make_ink_point(5.0, 0.0),
            make_ink_point(10.0, 0.0),
        ];
        let resampled = resample::resample_by_distance(&pts, 2.0);
        assert_eq!(resampled.len(), 6);
        let expected = vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0];
        for (i, p) in resampled.iter().enumerate() {
            assert!((p.x - expected[i]).abs() < 1e-4);
            assert_eq!(p.y, 0.0);
        }
    }

    #[test]
    fn test_resampling_corner_polyline() {
        let pts = vec![
            make_ink_point(0.0, 0.0),
            make_ink_point(3.0, 0.0),
            make_ink_point(3.0, 4.0),
        ];
        let resampled = resample::resample_by_distance(&pts, 2.0);
        assert_eq!(resampled.len(), 5);
        assert!((resampled[0].x - 0.0).abs() < 1e-4);
        assert!((resampled[0].y - 0.0).abs() < 1e-4);
        assert!((resampled[1].x - 2.0).abs() < 1e-4);
        assert!((resampled[1].y - 0.0).abs() < 1e-4);
        assert!((resampled[2].x - 3.0).abs() < 1e-4);
        assert!((resampled[2].y - 1.0).abs() < 1e-4);
        assert!((resampled[3].x - 3.0).abs() < 1e-4);
        assert!((resampled[3].y - 3.0).abs() < 1e-4);
        assert!((resampled[4].x - 3.0).abs() < 1e-4);
        assert!((resampled[4].y - 4.0).abs() < 1e-4);
    }

    #[test]
    fn test_resampling_non_divisible_length() {
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(5.0, 0.0)];
        let resampled = resample::resample_by_distance(&pts, 2.0);
        assert_eq!(resampled.len(), 4);
        assert!((resampled[2].x - 4.0).abs() < 1e-4);
        assert!((resampled[3].x - 5.0).abs() < 1e-4);
    }

    #[test]
    fn test_resampling_interpolated_fields() {
        let mut p1 = make_ink_point(0.0, 0.0);
        p1.press = 0.2;
        p1.t_ms = 100.0;
        let mut p2 = make_ink_point(10.0, 0.0);
        p2.press = 0.8;
        p2.t_ms = 200.0;
        let pts = vec![p1, p2];
        let resampled = resample::resample_by_distance(&pts, 5.0);
        assert_eq!(resampled.len(), 3);
        assert!((resampled[1].press - 0.5).abs() < 1e-4);
        assert!((resampled[1].t_ms - 150.0).abs() < 1e-4);
    }

    #[test]
    fn test_s_curve_adaptive_tessellation() {
        let pts = vec![
            make_ink_point(0.0, 0.0),
            make_ink_point(3.0, 3.0),
            make_ink_point(7.0, -3.0),
            make_ink_point(10.0, 0.0),
        ];
        let spline = smooth::adaptive_catmull_rom(&pts, 10.0);
        assert!(spline.len() > 10);
    }

    #[test]
    fn test_zoom_affects_detail_and_bounded() {
        let pts = vec![
            make_ink_point(0.0, 0.0),
            make_ink_point(5.0, 5.0),
            make_ink_point(10.0, 0.0),
        ];
        let low = smooth::adaptive_catmull_rom(&pts, 1.0);
        let high = smooth::adaptive_catmull_rom(&pts, 100.0);
        assert!(high.len() > low.len());
        assert!(high.len() < 500);
        for p in &high {
            assert!(p.x.is_finite());
            assert!(p.y.is_finite());
        }
    }

    #[test]
    fn test_conservative_bbox_contains_geometry() {
        let pts = vec![
            make_ink_point(0.0, 0.0),
            make_ink_point(5.0, 10.0),
            make_ink_point(10.0, 0.0),
        ];
        let brush = Brush::default_pen();
        let bbox = geom::compute_conservative_stroke_bbox(&pts, &brush).unwrap();
        let centerline = smooth::adaptive_catmull_rom(&pts, 10.0);
        let outline = geom::generate_stroke_outline(&centerline, &brush, 16).unwrap();
        for p in &outline {
            assert!(
                p.x >= bbox.min_x,
                "p.x = {}, bbox.min_x = {}",
                p.x,
                bbox.min_x
            );
            assert!(
                p.y >= bbox.min_y,
                "p.y = {}, bbox.min_y = {}",
                p.y,
                bbox.min_y
            );
            assert!(
                p.x <= bbox.max_x,
                "p.x = {}, bbox.max_x = {}",
                p.x,
                bbox.max_x
            );
            assert!(
                p.y <= bbox.max_y,
                "p.y = {}, bbox.max_y = {}",
                p.y,
                bbox.max_y
            );
        }
    }

    #[test]
    fn test_thick_pressure_stroke_bbox() {
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 0.0)];
        let mut brush = Brush::default_pen();
        brush.base_w = 20.0;
        let bbox = geom::compute_conservative_stroke_bbox(&pts, &brush).unwrap();
        let centerline = smooth::adaptive_catmull_rom(&pts, 10.0);
        let outline = geom::generate_stroke_outline(&centerline, &brush, 16).unwrap();
        for p in &outline {
            assert!(p.x >= bbox.min_x);
            assert!(p.y >= bbox.min_y);
            assert!(p.x <= bbox.max_x);
            assert!(p.y <= bbox.max_y);
        }
    }

    #[test]
    fn test_taper_stability_growing_nonzero_start() {
        let mut brush = Brush::default_pen();
        brush.taper_start = 2.0;
        brush.taper_end = 2.0;
        let pts1 = vec![
            make_ink_point(0.0, 0.0),
            make_ink_point(5.0, 0.0),
            make_ink_point(10.0, 0.0),
        ];
        let outline1 = geom::generate_stroke_outline(&pts1, &brush, 8).unwrap();
        let mut pts2 = pts1.clone();
        pts2.push(make_ink_point(15.0, 0.0));
        pts2.push(make_ink_point(20.0, 0.0));
        pts2.push(make_ink_point(25.0, 0.0));
        let outline2 = geom::generate_stroke_outline(&pts2, &brush, 8).unwrap();
        assert!((outline1[0].x - outline2[0].x).abs() < 1e-4);
        assert!((outline1[0].y - outline2[0].y).abs() < 1e-4);
    }

    #[test]
    fn test_geom_rev_invalidation() {
        let mut session = InkSession::new(800.0, 600.0);
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 0.0)];
        let local_bbox =
            geom::compute_conservative_stroke_bbox(&pts, &Brush::default_pen()).unwrap();
        let s = InkStroke {
            id: StrokeId::new(),
            brush: Brush::default_pen(),
            raw_pts: pts.clone(),
            pts,
            local_bbox,
            world_bbox: local_bbox,
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let sid = s.id;
        session.add_stroke(s);
        assert_eq!(session.doc.get_stroke(sid).unwrap().geom_rev, 0);
        let mut brush = Brush::default_pen();
        brush.base_w = 5.0;
        let tx = InkTx::new("change brush").push(InkOp::SetStrokeBrush {
            stroke_ids: vec![sid],
            before: vec![Brush::default_pen()],
            after: brush,
        });
        session.do_tx(tx);
        assert_eq!(session.doc.get_stroke(sid).unwrap().geom_rev, 1);
    }
}
