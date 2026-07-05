pub mod bbox;
pub mod brush;
pub mod clipboard;
pub mod color;
pub mod doc;
pub mod export_json;
pub mod export_svg;
pub mod geom;
pub mod hit;
pub mod ids;
pub mod item;
pub mod layer;
pub mod ops;
pub mod point;
pub mod resample;
pub mod runtime;
pub mod sel;
pub mod session;
pub mod shortcuts;
pub mod smooth;
pub mod spatial;
pub mod stroke;
pub mod viewport;
pub mod xform;

pub use bbox::BBox;
pub use brush::{stroke_w, Brush, BrushKind};
pub use clipboard::{ClipboardBundle, ClipboardItemRecord};
pub use color::ColorRgba;
pub use doc::{InkBackground, InkDoc};
pub use geom::{
    compute_conservative_stroke_bbox, compute_outline_bbox, generate_stroke_outline, xform_scale,
};
pub use ids::{AssetId, BrushId, DocId, ItemId, LayerId, StrokeId};
pub use item::{ImageAsset, InkImage, InkItem};
pub use layer::InkLayer;
pub use ops::{InkOp, InkTx, UndoRedo};
pub use point::{InkPoint, Point2, PointerKind, Vec2};
pub use runtime::InkRuntime;
pub use sel::{
    apply_selection_hits, lasso_query, lasso_select, rect_query, select_rect, SelectionIntent,
};
pub use session::{InkError, InkSession, Tool, ZOrderCmd};
pub use shortcuts::{
    AppSettings, Command, ConflictError, KeyChord, ShortcutMap, TempPanController,
};
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
            parent_id: None,
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
        let item = &doc.active_layer().unwrap().items[0];
        if let InkItem::Stroke(stroke) = item {
            assert!(hit::stroke_hit(stroke, Point2::new(50.0, 10.0), 5.0));
            assert!(!hit::stroke_hit(stroke, Point2::new(50.0, 100.0), 5.0));
        } else {
            panic!("expected stroke");
        }
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
        assert!(doc.runtime.sel_items.contains(&sid));
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
        assert!(doc.runtime.item_pos.contains_key(&sid));
        doc.rebuild_runtime();
        assert!(doc.runtime.item_pos.contains_key(&sid));
    }

    #[test]
    fn test_delete_updates_index() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let pts = vec![make_ink_point(10.0, 10.0), make_ink_point(20.0, 10.0)];
        let sid = make_stroke_in_doc(&mut doc, pts);
        assert!(doc.runtime.item_pos.contains_key(&sid));
        doc.delete_items(&[sid]);
        assert!(!doc.runtime.item_pos.contains_key(&sid));
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
        doc.delete_items(&[s1, s2]);
        assert!(!doc.runtime.item_pos.contains_key(&s1));
        assert!(!doc.runtime.item_pos.contains_key(&s2));
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
            parent_id: None,
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
        assert_eq!(session.doc.active_layer().unwrap().items.len(), 1);
        session.undo();
        assert_eq!(session.doc.active_layer().unwrap().items.len(), 0);
        session.redo();
        assert_eq!(session.doc.active_layer().unwrap().items.len(), 1);
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
            parent_id: None,
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
        assert_eq!(session.doc.active_layer().unwrap().items.len(), 0);
        session.undo();
        assert_eq!(session.doc.active_layer().unwrap().items.len(), 1);
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
                parent_id: None,
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
        assert_eq!(session.doc.active_layer().unwrap().items.len(), 3);
        session.clear_active_layer();
        assert_eq!(session.doc.active_layer().unwrap().items.len(), 0);
        session.undo();
        assert_eq!(session.doc.active_layer().unwrap().items.len(), 3);
        session.redo();
        assert_eq!(session.doc.active_layer().unwrap().items.len(), 0);
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
            parent_id: None,
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
        assert_eq!(layer.items.len(), 1);
        assert_eq!(layer.items[0].id(), sid);
    }

    #[test]
    fn test_json_roundtrip_empty_doc() {
        let session = InkSession::new(1024.0, 768.0);
        let json = session.export_json().unwrap();
        let restored = InkSession::import_json(&json).unwrap();
        assert_eq!(restored.doc.layers.len(), 1);
        assert_eq!(restored.doc.active_layer().unwrap().items.len(), 0);
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
            parent_id: None,
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
        let stroke = builder.finish(0, None);
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
        let s = builder.finish(0, None).unwrap();
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
        assert!(builder.finish(0, None).is_none());
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
                parent_id: None,
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
    fn test_resampling_endpoint_precision_needed() {
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(2.05, 0.0)];
        let resampled = resample::resample_by_distance(&pts, 2.0);
        assert_eq!(resampled.len(), 3);
        assert!((resampled[2].x - 2.05).abs() < 1e-4);
    }

    #[test]
    fn test_resampling_endpoint_precision_identical() {
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(2.0 + 1e-12, 0.0)];
        let resampled = resample::resample_by_distance(&pts, 2.0);
        assert_eq!(resampled.len(), 2);
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
        assert!(high.len() < smooth::MAX_OUTPUT_PTS);
        for p in &high {
            assert!(p.x.is_finite());
            assert!(p.y.is_finite());
        }
    }

    #[test]
    fn test_strict_point_limit_and_tessellation() {
        let mut pts = Vec::new();
        for i in 0..100 {
            let x = i as f32;
            let y = if i % 2 == 0 { 10.0 } else { 0.0 };
            pts.push(make_ink_point(x, y));
        }
        let res = smooth::adaptive_catmull_rom(&pts, 100000.0);
        assert!(res.len() <= smooth::MAX_OUTPUT_PTS);
        assert!(res.len() > 100);
        for p in &res {
            assert!(p.x.is_finite());
            assert!(p.y.is_finite());
        }
    }

    #[test]
    fn test_final_endpoint_preserved_under_budget_pressure() {
        let mut pts = Vec::new();
        for i in 0..150 {
            let x = i as f32;
            let y = if i % 2 == 0 { 20.0 } else { 0.0 };
            pts.push(make_ink_point(x, y));
        }
        let first_in = pts[0];
        let last_in = pts[pts.len() - 1];
        let res = smooth::adaptive_catmull_rom(&pts, 1000000.0);
        assert!(res.len() <= smooth::MAX_OUTPUT_PTS);
        assert_eq!(res[0].x, first_in.x);
        assert_eq!(res[0].y, first_in.y);
        let last_out = res[res.len() - 1];
        assert!((last_out.x - last_in.x).abs() < 1e-4);
        assert!((last_out.y - last_in.y).abs() < 1e-4);
    }

    #[test]
    fn test_no_stroke_suffix_truncation() {
        let mut pts = Vec::new();
        for i in 0..120 {
            let x = i as f32 * 0.1;
            let y = if i % 2 == 0 { 5.0 } else { 0.0 };
            pts.push(make_ink_point(x, y));
        }
        pts.push(make_ink_point(1000.0, 0.0));
        let res = smooth::adaptive_catmull_rom(&pts, 1000000.0);
        assert!(res.len() <= smooth::MAX_OUTPUT_PTS);
        let has_suffix = res.iter().any(|p| p.x > 900.0);
        assert!(has_suffix, "Suffix was truncated");
        let last_out = res[res.len() - 1];
        assert!((last_out.x - 1000.0).abs() < 1e-4);
        assert!((last_out.y - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_finite_and_ordered_under_budget() {
        let mut pts = Vec::new();
        for i in 0..200 {
            let x = i as f32;
            let y = if i % 2 == 0 { 50.0 } else { 0.0 };
            pts.push(make_ink_point(x, y));
        }
        let res = smooth::adaptive_catmull_rom(&pts, 1000000.0);
        assert!(res.len() <= smooth::MAX_OUTPUT_PTS);
        for i in 0..res.len() - 1 {
            assert!(res[i].x.is_finite());
            assert!(res[i].y.is_finite());
            let dx = res[i + 1].x - res[i].x;
            let dy = res[i + 1].y - res[i].y;
            assert!(
                dx * dx + dy * dy >= 1e-12,
                "Duplicate adjacent point found at index {}",
                i
            );
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
        let centerline = smooth::adaptive_catmull_rom(&pts, 500.0);
        let outline = geom::generate_stroke_outline(&centerline, &brush, 64).unwrap();
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
        let centerline = smooth::adaptive_catmull_rom(&pts, 500.0);
        let outline = geom::generate_stroke_outline(&centerline, &brush, 64).unwrap();
        for p in &outline {
            assert!(p.x >= bbox.min_x);
            assert!(p.y >= bbox.min_y);
            assert!(p.x <= bbox.max_x);
            assert!(p.y <= bbox.max_y);
        }
    }

    #[test]
    fn test_highlighter_stroke_bbox() {
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 0.0)];
        let brush = Brush::default_highlighter();
        let bbox = geom::compute_conservative_stroke_bbox(&pts, &brush).unwrap();
        let centerline = smooth::adaptive_catmull_rom(&pts, 500.0);
        let outline = geom::generate_stroke_outline(&centerline, &brush, 64).unwrap();
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
        brush.taper_end = 0.0;
        let pts1 = vec![
            make_ink_point(0.0, 0.0),
            make_ink_point(1.0, 0.0),
            make_ink_point(2.0, 0.0),
            make_ink_point(5.0, 0.0),
            make_ink_point(10.0, 0.0),
        ];
        let outline1 = geom::generate_stroke_outline(&pts1, &brush, 8).unwrap();
        let mut pts2 = pts1.clone();
        pts2.push(make_ink_point(15.0, 0.0));
        pts2.push(make_ink_point(20.0, 0.0));
        pts2.push(make_ink_point(30.0, 0.0));
        let outline2 = geom::generate_stroke_outline(&pts2, &brush, 8).unwrap();

        let get_width_at_1 = |outline: &[Point2]| -> f32 {
            let mut left_p = Point2::new(0.0, 0.0);
            let mut left_min_dist = f32::MAX;
            let mut right_p = Point2::new(0.0, 0.0);
            let mut right_min_dist = f32::MAX;
            let mid = outline.len() / 2;
            for (idx, p) in outline.iter().enumerate() {
                let dist = (p.x - 1.0).abs();
                if idx < mid {
                    if dist < left_min_dist {
                        left_min_dist = dist;
                        left_p = *p;
                    }
                } else {
                    if dist < right_min_dist {
                        right_min_dist = dist;
                        right_p = *p;
                    }
                }
            }
            (left_p.y - right_p.y).abs()
        };

        let w1 = get_width_at_1(&outline1);
        let w2 = get_width_at_1(&outline2);
        assert!((w1 - 1.5).abs() < 0.1);
        assert!((w1 - w2).abs() < 1e-4);
    }

    #[test]
    fn test_geom_rev_invalidation() {
        let mut session = InkSession::new(800.0, 600.0);
        let pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 0.0)];
        let local_bbox =
            geom::compute_conservative_stroke_bbox(&pts, &Brush::default_pen()).unwrap();
        let s = InkStroke {
            id: StrokeId::new(),
            parent_id: None,
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
        let tx = InkTx::new("change brush").push(InkOp::SetStrokeBrushes {
            stroke_ids: vec![sid],
            before: vec![Brush::default_pen()],
            after: vec![brush],
        });
        session.do_tx(tx);
        assert_eq!(session.doc.get_stroke(sid).unwrap().geom_rev, 1);
    }

    #[test]
    fn test_xform_inverse() {
        let xf = Xform2D::translate(10.0, -5.0)
            .concat(Xform2D::rotate(0.5))
            .concat(Xform2D::scale(2.0, 3.0));
        let inv = xf.inverse().unwrap();
        let pt = Point2::new(1.0, 2.0);
        let transformed = xf.apply(pt);
        let restored = inv.apply(transformed);
        assert!((restored.x - pt.x).abs() < 1e-4);
        assert!((restored.y - pt.y).abs() < 1e-4);

        let singular = Xform2D {
            a: 1.0,
            b: 2.0,
            c: 2.0,
            d: 4.0,
            tx: 0.0,
            ty: 0.0,
        };
        assert!(singular.inverse().is_none());
    }

    #[test]
    fn test_v1_v2_migration() {
        let v1_json = r#"{
            "schema_version": 1,
            "id": "00000000-0000-0000-0000-000000000000",
            "width": 800.0,
            "height": 600.0,
            "background": "Plain",
            "active_layer_id": "11111111-1111-1111-1111-111111111111",
            "layers": [
                {
                    "id": "11111111-1111-1111-1111-111111111111",
                    "name": "Layer 1",
                    "visible": true,
                    "locked": false,
                    "opacity": 1.0,
                    "strokes": [
                        {
                            "id": "22222222-2222-2222-2222-222222222222",
                            "brush": {
                                "id": "33333333-3333-3333-3333-333333333333",
                                "name": "Pen",
                                "kind": "Pen",
                                "color": {"r": 0, "g": 0, "b": 0, "a": 255},
                                "base_w": 2.0,
                                "opacity": 1.0,
                                "min_press": 0.1,
                                "max_press": 1.0,
                                "smooth": 0.5,
                                "streamline": 0.5,
                                "taper_start": 0.0,
                                "taper_end": 2.0
                            },
                            "raw_pts": [],
                            "pts": [],
                            "local_bbox": {"min_x": 0.0, "min_y": 0.0, "max_x": 1.0, "max_y": 1.0},
                            "world_bbox": {"min_x": 0.0, "min_y": 0.0, "max_x": 1.0, "max_y": 1.0},
                            "xform": {"a": 1.0, "b": 0.0, "c": 0.0, "d": 1.0, "tx": 0.0, "ty": 0.0},
                            "created_at_ms": 0,
                            "updated_at_ms": 0
                        }
                    ]
                }
            ],
            "created_at_ms": 0,
            "updated_at_ms": 0
        }"#;

        let session = InkSession::import_json(v1_json).unwrap();
        assert_eq!(session.doc.schema_version, 3);
        let layer = &session.doc.layers[0];
        assert_eq!(layer.items.len(), 1);
        assert!(layer.items[0].is_stroke());
    }

    #[test]
    fn test_z_order_preservation() {
        let mut session = InkSession::new(800.0, 600.0);
        let pts = vec![make_ink_point(0.0, 0.0)];
        make_stroke_in_doc(&mut session.doc, pts.clone());
        make_stroke_in_doc(&mut session.doc, pts.clone());
        make_stroke_in_doc(&mut session.doc, pts.clone());
        assert_eq!(session.doc.layers[0].items.len(), 3);

        let ids: Vec<ItemId> = session.doc.layers[0]
            .items
            .iter()
            .map(|item| item.id())
            .collect();
        let to_delete = vec![ids[0], ids[2]];
        let removed = session.doc.delete_items(&to_delete);
        assert_eq!(session.doc.layers[0].items.len(), 1);
        assert_eq!(session.doc.layers[0].items[0].id(), ids[1]);

        let tx = InkTx::new("delete").push(InkOp::DeleteItems { items: removed });
        session.do_tx(tx);

        session.undo();
        assert_eq!(session.doc.layers[0].items.len(), 3);
        assert_eq!(session.doc.layers[0].items[0].id(), ids[0]);
        assert_eq!(session.doc.layers[0].items[1].id(), ids[1]);
        assert_eq!(session.doc.layers[0].items[2].id(), ids[2]);
    }

    #[test]
    fn test_attachment_and_transforms() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let s_id = make_stroke_in_doc(&mut doc, vec![make_ink_point(10.0, 10.0)]);
        let s = doc.get_stroke(s_id).unwrap();
        assert_eq!(s.parent_id, None);

        let asset = ImageAsset {
            id: AssetId::new(),
            mime: "image/png".to_string(),
            width_px: 100,
            height_px: 100,
            bytes: vec![0; 10],
        };
        doc.add_asset(asset.clone());
        let img_id = ItemId::new();
        let img_item = InkItem::Image(InkImage {
            id: img_id,
            asset_id: asset.id,
            width: 100.0,
            height: 100.0,
            opacity: 1.0,
            xform: Xform2D::translate(100.0, 200.0),
            local_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            world_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        });
        let layer_id = doc.active_layer_id;
        doc.add_item(layer_id, img_item);

        let pts = vec![make_ink_point(50.0, 50.0)];
        let brush = Brush::default_pen();
        let local_bbox = compute_bbox(&pts, brush.base_w * 0.5).unwrap();
        let stroke = InkStroke {
            id: ItemId::new(),
            parent_id: Some(img_id),
            brush,
            raw_pts: pts.clone(),
            pts,
            local_bbox,
            world_bbox: local_bbox,
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let kid_id = stroke.id;
        doc.add_item(layer_id, InkItem::Stroke(stroke));

        let s2 = doc.get_stroke(kid_id).unwrap();
        assert_eq!(s2.parent_id, Some(img_id));

        let world_pointer = Point2::new(150.0, 250.0);
        let img = match doc.get_item(img_id).unwrap() {
            InkItem::Image(img) => img,
            _ => panic!("expected image"),
        };
        let local_pos = img.xform.inverse().unwrap().apply(world_pointer);
        assert_eq!(local_pos, Point2::new(50.0, 50.0));

        let free_eff = doc.effective_xform(s_id);
        assert_eq!(free_eff, Xform2D::identity());

        let kid_eff = doc.effective_xform(kid_id);
        assert_eq!(kid_eff, Xform2D::translate(100.0, 200.0));

        let kid = doc.get_stroke(kid_id).unwrap();
        assert_eq!(kid.world_bbox.min_x, 148.5);

        if let Some(InkItem::Image(img)) = doc.get_item_mut(img_id) {
            img.xform = Xform2D::translate(300.0, 400.0);
        }
        doc.rebuild_runtime();
        let _kid = doc.get_stroke(kid_id).unwrap();
        let query_candidates = doc.query_bbox(BBox::new(340.0, 440.0, 360.0, 460.0));
        assert!(query_candidates.contains(&kid_id));
        let query_old = doc.query_bbox(BBox::new(140.0, 240.0, 160.0, 260.0));
        assert!(!query_old.contains(&kid_id));

        doc.clear_sel();
        doc.runtime.sel_items.insert(img_id);
        let pts_out = vec![make_ink_point(-200.0, -200.0)];
        let local_bbox_out = compute_bbox(&pts_out, 2.0).unwrap();
        let s_out = InkStroke {
            id: ItemId::new(),
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: pts_out.clone(),
            pts: pts_out,
            local_bbox: local_bbox_out,
            world_bbox: local_bbox_out,
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let out_id = s_out.id;
        doc.add_item(layer_id, InkItem::Stroke(s_out));
        let sel_bounds2 = doc.selection_bbox().unwrap();
        assert!(sel_bounds2.min_x <= 100.0);

        let deleted = doc.delete_items(&[img_id]);
        assert_eq!(deleted.len(), 3);
        let deleted_ids: Vec<ItemId> = deleted.iter().map(|(_, _, item)| item.id()).collect();
        assert!(deleted_ids.contains(&img_id));
        assert!(deleted_ids.contains(&kid_id));
        assert!(deleted_ids.contains(&out_id));
    }

    #[test]
    fn test_duplicate_remap_and_cascades() {
        let mut session = InkSession::new(800.0, 600.0);
        let asset = ImageAsset {
            id: AssetId::new(),
            mime: "image/png".to_string(),
            width_px: 100,
            height_px: 100,
            bytes: vec![0; 5],
        };
        session.doc.add_asset(asset.clone());
        let img_id = ItemId::new();
        let img = InkImage {
            id: img_id,
            asset_id: asset.id,
            width: 100.0,
            height: 100.0,
            opacity: 1.0,
            xform: Xform2D::translate(10.0, 10.0),
            local_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            world_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        session
            .doc
            .add_item(session.doc.active_layer_id, InkItem::Image(img));

        let stroke = InkStroke {
            id: ItemId::new(),
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(50.0, 50.0)],
            pts: vec![make_ink_point(50.0, 50.0)],
            local_bbox: BBox::new(48.0, 48.0, 52.0, 52.0),
            world_bbox: BBox::new(48.0, 48.0, 52.0, 52.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let kid_id = stroke.id;
        session
            .doc
            .add_item(session.doc.active_layer_id, InkItem::Stroke(stroke));

        session.doc.clear_sel();
        session.doc.runtime.sel_items.insert(img_id);
        session.duplicate_sel();

        let layer = session.doc.active_layer().unwrap();
        assert_eq!(layer.items.len(), 4);

        let dup_img_id = layer.items[1].id();
        let dup_stroke = match &layer.items[3] {
            InkItem::Stroke(s) => s,
            _ => panic!("expected stroke"),
        };
        assert_eq!(dup_stroke.parent_id, Some(dup_img_id));
        let dup_img = match &layer.items[1] {
            InkItem::Image(img) => img,
            _ => panic!("expected image"),
        };
        assert_eq!(dup_img.asset_id, asset.id);

        session.doc.clear_sel();
        session.doc.runtime.sel_items.insert(kid_id);
        session.duplicate_sel();
        let layer2 = session.doc.active_layer().unwrap();
        assert_eq!(layer2.items.len(), 5);
        let lone_dup = match &layer2.items[3] {
            InkItem::Stroke(s) => s,
            _ => panic!("expected stroke"),
        };
        assert_eq!(lone_dup.parent_id, Some(img_id));
    }

    #[test]
    fn test_save_load_validation() {
        let mut session = InkSession::new(800.0, 600.0);
        let asset = ImageAsset {
            id: AssetId::new(),
            mime: "image/png".to_string(),
            width_px: 10,
            height_px: 10,
            bytes: vec![1, 2, 3],
        };
        session.doc.add_asset(asset.clone());
        let img_id = ItemId::new();
        let img = InkImage {
            id: img_id,
            asset_id: asset.id,
            width: 10.0,
            height: 10.0,
            opacity: 1.0,
            xform: Xform2D::identity(),
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        session
            .doc
            .add_item(session.doc.active_layer_id, InkItem::Image(img));

        let stroke = InkStroke {
            id: ItemId::new(),
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(5.0, 5.0)],
            pts: vec![make_ink_point(5.0, 5.0)],
            local_bbox: BBox::new(3.0, 3.0, 7.0, 7.0),
            world_bbox: BBox::new(3.0, 3.0, 7.0, 7.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let kid_id = stroke.id;
        session
            .doc
            .add_item(session.doc.active_layer_id, InkItem::Stroke(stroke));

        let json = session.export_json().unwrap();
        let restored = InkSession::import_json(&json).unwrap();
        assert_eq!(restored.doc.schema_version, 3);
        let restored_stroke = restored.doc.get_stroke(kid_id).unwrap();
        assert_eq!(restored_stroke.parent_id, Some(img_id));

        let bad_json = json.replace(
            &format!("\"parent_id\":\"{}\"", img_id.0),
            &format!("\"parent_id\":\"{}\"", ItemId::new().0),
        );
        assert!(InkSession::import_json(&bad_json).is_err());
    }

    #[test]
    fn test_stale_selected_ids_pruned() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let id1 = ItemId::new();
        doc.runtime.sel_items.insert(id1);
        doc.rebuild_runtime();
        assert!(!doc.runtime.sel_items.contains(&id1));
    }

    #[test]
    fn test_annotation_target_resolution_rules() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let asset = ImageAsset {
            id: AssetId::new(),
            mime: "image/png".to_string(),
            width_px: 100,
            height_px: 100,
            bytes: vec![0; 5],
        };
        doc.add_asset(asset.clone());
        let img1_id = ItemId::new();
        doc.add_item(
            doc.active_layer_id,
            InkItem::Image(InkImage {
                id: img1_id,
                asset_id: asset.id,
                width: 100.0,
                height: 100.0,
                opacity: 1.0,
                xform: Xform2D::identity(),
                local_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
                world_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        );

        let img2_id = ItemId::new();
        doc.add_item(
            doc.active_layer_id,
            InkItem::Image(InkImage {
                id: img2_id,
                asset_id: asset.id,
                width: 100.0,
                height: 100.0,
                opacity: 1.0,
                xform: Xform2D::identity(),
                local_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
                world_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        );

        let stroke1 = InkStroke {
            id: ItemId::new(),
            parent_id: Some(img1_id),
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(10.0, 10.0)],
            pts: vec![make_ink_point(10.0, 10.0)],
            local_bbox: BBox::new(9.0, 9.0, 11.0, 11.0),
            world_bbox: BBox::new(9.0, 9.0, 11.0, 11.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let s1_id = stroke1.id;
        doc.add_item(doc.active_layer_id, InkItem::Stroke(stroke1));

        doc.clear_sel();
        doc.runtime.sel_items.insert(img1_id);
        assert_eq!(doc.annotation_target_image(), Some(img1_id));

        doc.runtime.sel_items.insert(s1_id);
        assert_eq!(doc.annotation_target_image(), Some(img1_id));

        doc.clear_sel();
        doc.runtime.sel_items.insert(img1_id);
        doc.runtime.sel_items.insert(img2_id);
        assert_eq!(doc.annotation_target_image(), None);

        let free_s_id = make_stroke_in_doc(&mut doc, vec![make_ink_point(0.0, 0.0)]);
        doc.clear_sel();
        doc.runtime.sel_items.insert(img1_id);
        doc.runtime.sel_items.insert(free_s_id);
        assert_eq!(doc.annotation_target_image(), None);
    }

    #[test]
    fn test_transform_roots_behavior() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let asset = ImageAsset {
            id: AssetId::new(),
            mime: "image/png".to_string(),
            width_px: 10,
            height_px: 10,
            bytes: vec![0; 5],
        };
        doc.add_asset(asset.clone());
        let img_id = ItemId::new();
        doc.add_item(
            doc.active_layer_id,
            InkItem::Image(InkImage {
                id: img_id,
                asset_id: asset.id,
                width: 10.0,
                height: 10.0,
                opacity: 1.0,
                xform: Xform2D::identity(),
                local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        );
        let stroke = InkStroke {
            id: ItemId::new(),
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(5.0, 5.0)],
            pts: vec![make_ink_point(5.0, 5.0)],
            local_bbox: BBox::new(4.0, 4.0, 6.0, 6.0),
            world_bbox: BBox::new(4.0, 4.0, 6.0, 6.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let s_id = stroke.id;
        doc.add_item(doc.active_layer_id, InkItem::Stroke(stroke));

        doc.clear_sel();
        doc.runtime.sel_items.insert(img_id);
        doc.runtime.sel_items.insert(s_id);
        let roots1 = doc.transform_roots();
        assert_eq!(roots1.len(), 1);
        assert!(roots1.contains(&img_id));

        doc.clear_sel();
        doc.runtime.sel_items.insert(s_id);
        let roots2 = doc.transform_roots();
        assert_eq!(roots2.len(), 1);
        assert!(roots2.contains(&s_id));
    }

    #[test]
    fn test_new_doc_schema_version_3() {
        let doc = InkDoc::new(800.0, 600.0);
        assert_eq!(doc.schema_version, 3);
    }

    #[test]
    fn test_live_parent_transforms_kid_moves_rtree() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let asset = ImageAsset {
            id: AssetId::new(),
            mime: "image/png".to_string(),
            width_px: 10,
            height_px: 10,
            bytes: vec![0],
        };
        doc.add_asset(asset.clone());
        let img_id = ItemId::new();
        doc.add_item(
            doc.active_layer_id,
            InkItem::Image(InkImage {
                id: img_id,
                asset_id: asset.id,
                width: 10.0,
                height: 10.0,
                opacity: 1.0,
                xform: Xform2D::translate(10.0, 10.0),
                local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        );
        let stroke = InkStroke {
            id: ItemId::new(),
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(5.0, 5.0)],
            pts: vec![make_ink_point(5.0, 5.0)],
            local_bbox: BBox::new(4.0, 4.0, 6.0, 6.0),
            world_bbox: BBox::new(4.0, 4.0, 6.0, 6.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let s_id = stroke.id;
        doc.add_item(doc.active_layer_id, InkItem::Stroke(stroke));

        if let Some(InkItem::Image(img)) = doc.get_item_mut(img_id) {
            img.xform = Xform2D::translate(50.0, 50.0);
        }
        doc.rebuild_runtime();
        let kid = doc.get_stroke(s_id).unwrap();
        assert_eq!(kid.world_bbox.min_x, 54.0);

        let query1 = doc.query_bbox(BBox::new(50.0, 50.0, 60.0, 60.0));
        assert!(query1.contains(&s_id));

        let query2 = doc.query_bbox(BBox::new(10.0, 10.0, 20.0, 20.0));
        assert!(!query2.contains(&s_id));
    }

    #[test]
    fn test_direct_child_world_transforms() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let asset = ImageAsset {
            id: AssetId::new(),
            mime: "image/png".to_string(),
            width_px: 10,
            height_px: 10,
            bytes: vec![0],
        };
        doc.add_asset(asset.clone());
        let img_id = ItemId::new();
        let p_xf = Xform2D::translate(10.0, 10.0)
            .concat(Xform2D::rotate(1.5707963))
            .concat(Xform2D::scale(2.0, 2.0));
        doc.add_item(
            doc.active_layer_id,
            InkItem::Image(InkImage {
                id: img_id,
                asset_id: asset.id,
                width: 10.0,
                height: 10.0,
                opacity: 1.0,
                xform: p_xf,
                local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        );
        let stroke = InkStroke {
            id: ItemId::new(),
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(5.0, 5.0)],
            pts: vec![make_ink_point(5.0, 5.0)],
            local_bbox: BBox::new(4.0, 4.0, 6.0, 6.0),
            world_bbox: BBox::new(4.0, 4.0, 6.0, 6.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let s_id = stroke.id;
        doc.add_item(doc.active_layer_id, InkItem::Stroke(stroke));

        let w_trans = Xform2D::translate(20.0, 30.0);
        let orig_local = doc.get_item(s_id).unwrap().xform();
        doc.apply_world_xform_to_item(s_id, w_trans, orig_local);
        doc.rebuild_runtime();

        let eff_xf = doc.effective_xform(s_id);
        let expected = w_trans.concat(p_xf).concat(orig_local);
        assert!((eff_xf.tx - expected.tx).abs() < 1e-3);
        assert!((eff_xf.ty - expected.ty).abs() < 1e-3);

        if let Some(InkItem::Image(img)) = doc.get_item_mut(img_id) {
            img.xform = Xform2D {
                a: 0.0,
                b: 0.0,
                c: 0.0,
                d: 0.0,
                tx: 0.0,
                ty: 0.0,
            };
        }
        let pre_s_xf = doc.get_stroke(s_id).unwrap().xform;
        doc.apply_world_xform_to_item(s_id, w_trans, pre_s_xf);
        assert_eq!(doc.get_stroke(s_id).unwrap().xform, pre_s_xf);
    }

    #[test]
    fn test_undo_duplicate_stale_sel_ids() {
        let mut session = InkSession::new(800.0, 600.0);
        let asset = ImageAsset {
            id: AssetId::new(),
            mime: "image/png".to_string(),
            width_px: 10,
            height_px: 10,
            bytes: vec![0],
        };
        session.doc.add_asset(asset.clone());
        let img_id = ItemId::new();
        session.doc.add_item(
            session.doc.active_layer_id,
            InkItem::Image(InkImage {
                id: img_id,
                asset_id: asset.id,
                width: 10.0,
                height: 10.0,
                opacity: 1.0,
                xform: Xform2D::identity(),
                local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        );

        session.doc.clear_sel();
        session.doc.runtime.sel_items.insert(img_id);
        session.duplicate_sel();
        assert_eq!(session.doc.runtime.sel_items.len(), 1);

        session.undo();
        assert_eq!(session.doc.runtime.sel_items.len(), 0);
    }

    #[test]
    fn test_repeated_sequence_regression() {
        let mut session = InkSession::new(800.0, 600.0);
        let asset = ImageAsset {
            id: AssetId::new(),
            mime: "image/png".to_string(),
            width_px: 100,
            height_px: 100,
            bytes: vec![0; 5],
        };
        session.doc.add_asset(asset.clone());
        let img_id = ItemId::new();
        session.doc.add_item(
            session.doc.active_layer_id,
            InkItem::Image(InkImage {
                id: img_id,
                asset_id: asset.id,
                width: 100.0,
                height: 100.0,
                opacity: 1.0,
                xform: Xform2D::identity(),
                local_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
                world_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        );

        session.doc.clear_sel();
        session.doc.runtime.sel_items.insert(img_id);
        let target1 = session.doc.annotation_target_image();
        assert_eq!(target1, Some(img_id));

        let kid1 = InkStroke {
            id: ItemId::new(),
            parent_id: target1,
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(50.0, 50.0)],
            pts: vec![make_ink_point(50.0, 50.0)],
            local_bbox: BBox::new(48.0, 48.0, 52.0, 52.0),
            world_bbox: BBox::new(48.0, 48.0, 52.0, 52.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let kid1_id = kid1.id;
        session
            .doc
            .add_item(session.doc.active_layer_id, InkItem::Stroke(kid1));

        if let Some(InkItem::Image(img)) = session.doc.get_item_mut(img_id) {
            img.xform = Xform2D::translate(100.0, 100.0);
        }
        session.doc.rebuild_runtime();

        let target2 = session.doc.annotation_target_image();
        assert_eq!(target2, Some(img_id));

        let kid2 = InkStroke {
            id: ItemId::new(),
            parent_id: target2,
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(30.0, 30.0)],
            pts: vec![make_ink_point(30.0, 30.0)],
            local_bbox: BBox::new(28.0, 28.0, 32.0, 32.0),
            world_bbox: BBox::new(28.0, 28.0, 32.0, 32.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let kid2_id = kid2.id;
        session
            .doc
            .add_item(session.doc.active_layer_id, InkItem::Stroke(kid2));

        if let Some(InkItem::Image(img)) = session.doc.get_item_mut(img_id) {
            img.xform = Xform2D::translate(200.0, 200.0);
        }
        session.doc.rebuild_runtime();

        let eff1 = session.doc.effective_xform(kid1_id);
        let eff2 = session.doc.effective_xform(kid2_id);
        assert_eq!(eff1, Xform2D::translate(200.0, 200.0));
        assert_eq!(eff2, Xform2D::translate(200.0, 200.0));
    }

    #[test]
    fn test_select_rect_stroke_intersection() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let s_pts = vec![make_ink_point(10.0, 10.0), make_ink_point(30.0, 10.0)];
        let sid = make_stroke_in_doc(&mut doc, s_pts);

        let rect2 = BBox::new(15.0, 5.0, 25.0, 15.0);
        let sel2 = select_rect(&mut doc, rect2);
        assert_eq!(sel2, vec![sid.into()]);

        let rect3 = BBox::new(40.0, 40.0, 50.0, 50.0);
        let sel3 = select_rect(&mut doc, rect3);
        assert!(sel3.is_empty());
    }

    #[test]
    fn test_select_rect_transformed_stroke() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let s_pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 0.0)];
        let sid = make_stroke_in_doc(&mut doc, s_pts);

        if let Some(InkItem::Stroke(s)) = doc.get_item_mut(sid.into()) {
            s.xform = Xform2D::translate(100.0, 100.0);
            s.recompute_world_bbox();
        }
        doc.rebuild_runtime();

        let rect_orig = BBox::new(-5.0, -5.0, 15.0, 15.0);
        let sel_orig = select_rect(&mut doc, rect_orig);
        assert!(sel_orig.is_empty());

        let rect_new = BBox::new(95.0, 95.0, 115.0, 115.0);
        let sel_new = select_rect(&mut doc, rect_new);
        assert_eq!(sel_new, vec![sid.into()]);
    }

    #[test]
    fn test_select_rect_attached_stroke() {
        let mut doc = InkDoc::new(800.0, 600.0);

        let aid = AssetId::new();
        let img = InkImage {
            id: ItemId::new(),
            asset_id: aid,
            width: 100.0,
            height: 100.0,
            opacity: 1.0,
            xform: Xform2D::translate(50.0, 50.0),
            local_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            world_bbox: BBox::new(50.0, 50.0, 150.0, 150.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let img_id = img.id;
        doc.add_item(doc.active_layer_id, InkItem::Image(img));

        let stroke = InkStroke {
            id: StrokeId::new(),
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(10.0, 10.0)],
            pts: vec![make_ink_point(10.0, 10.0)],
            local_bbox: BBox::new(9.0, 9.0, 11.0, 11.0),
            world_bbox: BBox::new(9.0, 9.0, 11.0, 11.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let stroke_id = stroke.id;
        doc.add_item(doc.active_layer_id, InkItem::Stroke(stroke));
        doc.rebuild_runtime();

        let rect = BBox::new(55.0, 55.0, 65.0, 65.0);
        let sel = select_rect(&mut doc, rect);
        assert!(sel.contains(&stroke_id.into()));
    }

    #[test]
    fn test_select_rect_image_intersection() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let aid = AssetId::new();
        let img = InkImage {
            id: ItemId::new(),
            asset_id: aid,
            width: 100.0,
            height: 100.0,
            opacity: 1.0,
            xform: Xform2D::translate(100.0, 100.0),
            local_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            world_bbox: BBox::new(100.0, 100.0, 200.0, 200.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let img_id = img.id;
        doc.add_item(doc.active_layer_id, InkItem::Image(img));
        doc.rebuild_runtime();

        let rect1 = BBox::new(150.0, 150.0, 250.0, 250.0);
        let sel1 = select_rect(&mut doc, rect1);
        assert_eq!(sel1, vec![img_id]);

        let rect2 = BBox::new(300.0, 300.0, 400.0, 400.0);
        let sel2 = select_rect(&mut doc, rect2);
        assert!(sel2.is_empty());
    }

    #[test]
    fn test_select_rect_rotated_image() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let aid = AssetId::new();
        let xf = Xform2D::rotate_about(Point2::new(5.0, 5.0), std::f32::consts::FRAC_PI_4);
        let img = InkImage {
            id: ItemId::new(),
            asset_id: aid,
            width: 10.0,
            height: 10.0,
            opacity: 1.0,
            xform: xf,
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: xf.apply_bbox(BBox::new(0.0, 0.0, 10.0, 10.0)),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let img_id = img.id;
        doc.add_item(doc.active_layer_id, InkItem::Image(img));
        doc.rebuild_runtime();

        let rect = BBox::new(9.0, 5.0, 15.0, 15.0);
        let sel = select_rect(&mut doc, rect);
        assert_eq!(sel, vec![img_id]);
    }

    #[test]
    fn test_select_rect_broad_phase_refinement() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let s_pts = vec![make_ink_point(0.0, 0.0), make_ink_point(10.0, 10.0)];
        let _sid = make_stroke_in_doc(&mut doc, s_pts);

        let rect = BBox::new(0.0, 8.0, 2.0, 10.0);
        let sel = select_rect(&mut doc, rect);
        assert!(
            sel.is_empty(),
            "Exact intersection must filter out broad-phase false positive"
        );
    }

    #[test]
    fn test_select_rect_transform_roots() {
        let mut doc = InkDoc::new(800.0, 600.0);

        let aid = AssetId::new();
        let img = InkImage {
            id: ItemId::new(),
            asset_id: aid,
            width: 100.0,
            height: 100.0,
            opacity: 1.0,
            xform: Xform2D::translate(50.0, 50.0),
            local_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            world_bbox: BBox::new(50.0, 50.0, 150.0, 150.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let img_id = img.id;
        doc.add_item(doc.active_layer_id, InkItem::Image(img));

        let stroke = InkStroke {
            id: StrokeId::new(),
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![make_ink_point(10.0, 10.0)],
            pts: vec![make_ink_point(10.0, 10.0)],
            local_bbox: BBox::new(9.0, 9.0, 11.0, 11.0),
            world_bbox: BBox::new(9.0, 9.0, 11.0, 11.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        let stroke_id = stroke.id;
        doc.add_item(doc.active_layer_id, InkItem::Stroke(stroke));
        doc.rebuild_runtime();

        let rect_both = BBox::new(40.0, 40.0, 160.0, 160.0);
        select_rect(&mut doc, rect_both);
        let roots = doc.transform_roots();
        assert!(roots.contains(&img_id));
        assert!(!roots.contains(&stroke_id.into()));

        doc.clear_sel();
        doc.runtime.sel_items.insert(stroke_id.into());
        let roots_child = doc.transform_roots();
        assert!(!roots_child.contains(&img_id));
        assert!(roots_child.contains(&stroke_id.into()));
    }

    #[test]
    fn test_shortcuts_default_map_no_conflicts() {
        let sm = ShortcutMap::defaults();
        let mut seen = std::collections::HashSet::new();
        for (cmd, chords) in &sm.map {
            for chord in chords {
                assert!(
                    seen.insert(chord.clone()),
                    "Duplicate chord: {:?} for {:?}",
                    chord,
                    cmd
                );
            }
        }
    }

    #[test]
    fn test_shortcuts_resolutions() {
        let sm = ShortcutMap::defaults();

        assert_eq!(
            sm.command_for_chord(&KeyChord::primary("KeyZ")),
            Some(Command::Undo)
        );

        assert_eq!(
            sm.command_for_chord(&KeyChord::primary_shift("KeyZ")),
            Some(Command::Redo)
        );

        assert_eq!(
            sm.command_for_chord(&KeyChord::primary("KeyY")),
            Some(Command::Redo)
        );

        assert_eq!(
            sm.command_for_chord(&KeyChord::simple("Delete")),
            Some(Command::DeleteSelection)
        );
        assert_eq!(
            sm.command_for_chord(&KeyChord::simple("Backspace")),
            Some(Command::DeleteSelection)
        );
    }

    #[test]
    fn test_shortcuts_multiple_bindings_and_removal() {
        let mut sm = ShortcutMap::new();
        assert!(sm
            .add_binding(Command::Undo, KeyChord::simple("KeyU"))
            .is_ok());
        assert!(sm
            .add_binding(Command::Undo, KeyChord::simple("KeyZ"))
            .is_ok());

        let bindings = sm.bindings(Command::Undo);
        assert_eq!(bindings.len(), 2);

        assert!(sm.remove_binding(Command::Undo, &KeyChord::simple("KeyU")));
        let bindings_after = sm.bindings(Command::Undo);
        assert_eq!(bindings_after.len(), 1);
        assert_eq!(bindings_after[0].code, "KeyZ");
    }

    #[test]
    fn test_shortcuts_duplicate_conflict_detected() {
        let mut sm = ShortcutMap::new();
        assert!(sm
            .add_binding(Command::Undo, KeyChord::simple("KeyZ"))
            .is_ok());

        let res = sm.add_binding(Command::Redo, KeyChord::simple("KeyZ"));
        assert_eq!(res, Err(ConflictError::Conflict(Command::Undo)));
    }

    #[test]
    fn test_shortcuts_modifier_only_rejected() {
        let mut sm = ShortcutMap::new();
        let res = sm.add_binding(Command::Undo, KeyChord::simple("ControlLeft"));
        assert_eq!(res, Err(ConflictError::ModifierOnly));
    }

    #[test]
    fn test_shortcuts_settings_json_round_trip() {
        let mut settings = AppSettings::new();
        settings.shortcuts.map.clear();
        settings
            .shortcuts
            .add_binding(Command::Undo, KeyChord::simple("KeyX"))
            .unwrap();

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, settings);

        assert!(json.contains("\"undo\""));
        assert!(!json.contains("\"Undo\""));
    }

    #[test]
    fn test_shortcuts_validation_and_repair() {
        let mut settings = AppSettings::new();
        settings.shortcuts.map.insert(
            Command::Undo,
            vec![
                KeyChord::simple("KeyZ"),
                KeyChord::simple("ControlLeft"), // invalid modifier only
            ],
        );
        settings.shortcuts.map.insert(
            Command::Redo,
            vec![
                KeyChord::simple("KeyZ"), // conflicting with Undo
            ],
        );

        settings.validate_and_repair();

        let undo_chords = settings.shortcuts.bindings(Command::Undo);
        assert_eq!(undo_chords.len(), 1);
        assert_eq!(undo_chords[0], KeyChord::simple("KeyZ"));

        let redo_chords = settings.shortcuts.bindings(Command::Redo);
        assert!(
            redo_chords.is_empty(),
            "Redo conflict should have been cleaned"
        );
    }

    #[test]
    fn test_command_all_completeness() {
        use std::collections::HashSet;
        let all_set: HashSet<&str> = Command::ALL.iter().map(|c| c.to_id()).collect();
        let expected = [
            "tool_pen",
            "tool_pencil",
            "tool_highlighter",
            "tool_eraser",
            "tool_lasso",
            "tool_select",
            "tool_pan",
            "undo",
            "redo",
            "select_all",
            "delete_selection",
            "duplicate_selection",
            "clear_selection",
            "copy",
            "cut",
            "paste",
            "nudge_left",
            "nudge_right",
            "nudge_up",
            "nudge_down",
            "bring_forward",
            "send_backward",
            "bring_to_front",
            "send_to_back",
            "hold_pan",
        ];
        for id in &expected {
            assert!(all_set.contains(id), "Command::ALL missing: {}", id);
        }
        assert_eq!(Command::ALL.len(), expected.len());
    }

    #[test]
    fn test_shortcuts_stable_ordering() {
        assert_eq!(Command::ALL.len(), 25);
        assert_eq!(Command::ALL[0].to_id(), "tool_pen");
        assert_eq!(Command::ALL[24].to_id(), "hold_pan");
    }

    #[test]
    fn test_shortcuts_deterministic_repair() {
        let mut settings = AppSettings::new();
        settings
            .shortcuts
            .map
            .insert(Command::Redo, vec![KeyChord::simple("KeyZ")]);
        settings
            .shortcuts
            .map
            .insert(Command::Undo, vec![KeyChord::simple("KeyZ")]);

        settings.validate_and_repair();

        assert_eq!(settings.shortcuts.bindings(Command::Undo).len(), 1);
        assert_eq!(
            settings.shortcuts.bindings(Command::Undo)[0],
            KeyChord::simple("KeyZ")
        );
        assert!(settings.shortcuts.bindings(Command::Redo).is_empty());
    }

    #[test]
    fn test_settings_version_policy() {
        assert!(
            AppSettings::is_version_supported(1),
            "version 1 must be accepted"
        );
        assert!(
            !AppSettings::is_version_supported(0),
            "version 0 must be rejected"
        );
        assert!(
            !AppSettings::is_version_supported(2),
            "version 2 must be rejected"
        );
        assert!(
            !AppSettings::is_version_supported(999),
            "version 999 must be rejected"
        );
        let s = AppSettings::new();
        assert!(AppSettings::is_version_supported(s.version));
    }

    #[test]
    fn test_temp_pan_controller_transitions() {
        let mut ctrl = TempPanController::new();
        assert!(!ctrl.is_active());

        assert!(ctrl.handle_keydown("Space", true));
        assert!(ctrl.is_active());

        assert!(!ctrl.handle_keydown("Space", true));

        assert!(ctrl.handle_keyup("Space", false));
        assert!(!ctrl.is_active());

        assert!(!ctrl.handle_keydown("Space", false));
        assert!(!ctrl.is_active());

        assert!(ctrl.handle_keydown("Space", true));
        assert!(!ctrl.handle_keyup("Space", true));
        assert!(ctrl.is_active());
        assert!(ctrl.released);

        assert!(ctrl.handle_gesture_end());
        assert!(!ctrl.is_active());

        assert!(ctrl.handle_keydown("Space", true));
        ctrl.reset();
        assert!(!ctrl.is_active());
    }

    #[derive(Clone)]
    struct TestNudgeState {
        active_cmd: Command,
        before_xforms: std::collections::HashMap<ItemId, Xform2D>,
    }

    fn test_commit_nudge(session: &mut InkSession, nudge_state: &mut Option<TestNudgeState>) {
        if let Some(state) = nudge_state.take() {
            let mut item_ids = Vec::new();
            let mut before_xfs = Vec::new();
            let mut after_xfs = Vec::new();
            for (id, start_xf) in state.before_xforms {
                if let Some(item) = session.doc.get_item(id) {
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
                session.do_tx(tx);
            }
        }
    }

    #[test]
    fn test_editing_01_intent_replace() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let s1 = make_stroke_in_doc(&mut doc, vec![make_ink_point(1.0, 1.0)]);
        let s2 = make_stroke_in_doc(&mut doc, vec![make_ink_point(2.0, 2.0)]);
        doc.runtime.sel_items.insert(s1);
        apply_selection_hits(&mut doc, &[s2], SelectionIntent::Replace);
        assert!(!doc.runtime.sel_items.contains(&s1));
        assert!(doc.runtime.sel_items.contains(&s2));
    }

    #[test]
    fn test_editing_02_intent_add() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let s1 = make_stroke_in_doc(&mut doc, vec![make_ink_point(1.0, 1.0)]);
        let s2 = make_stroke_in_doc(&mut doc, vec![make_ink_point(2.0, 2.0)]);
        doc.runtime.sel_items.insert(s1);
        apply_selection_hits(&mut doc, &[s2], SelectionIntent::Add);
        assert!(doc.runtime.sel_items.contains(&s1));
        assert!(doc.runtime.sel_items.contains(&s2));
    }

    #[test]
    fn test_editing_03_intent_toggle() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let s1 = make_stroke_in_doc(&mut doc, vec![make_ink_point(1.0, 1.0)]);
        let s2 = make_stroke_in_doc(&mut doc, vec![make_ink_point(2.0, 2.0)]);
        doc.runtime.sel_items.insert(s1);
        apply_selection_hits(&mut doc, &[s1, s2], SelectionIntent::Toggle);
        assert!(!doc.runtime.sel_items.contains(&s1));
        assert!(doc.runtime.sel_items.contains(&s2));
    }

    #[test]
    fn test_editing_04_copy_empty() {
        let session = InkSession::new(800.0, 600.0);
        assert!(session.copy_sel().is_none());
    }

    #[test]
    fn test_editing_05_copy_layer_order() {
        let mut session = InkSession::new(800.0, 600.0);
        let new_layer = InkLayer::new("Layer 2");
        let new_layer_id = new_layer.id;
        session.doc.layers.push(new_layer);
        session.doc.rebuild_runtime();

        let s1 = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        session.doc.active_layer_id = new_layer_id;
        let s2 = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(2.0, 2.0)]);
        session.doc.runtime.sel_items.insert(s1);
        session.doc.runtime.sel_items.insert(s2);
        let bundle = session.copy_sel().unwrap();
        assert_eq!(bundle.records[0].item.id(), s1);
        assert_eq!(bundle.records[1].item.id(), s2);
        assert_eq!(bundle.records[0].source_layer_rank, 0);
        assert_eq!(bundle.records[1].source_layer_rank, 1);
    }

    #[test]
    fn test_editing_06_copy_attached_children() {
        let mut session = InkSession::new(800.0, 600.0);
        let img_id = ItemId::new();
        session.doc.layers[0].items.push(InkItem::Image(InkImage {
            id: img_id,
            asset_id: AssetId::new(),
            width: 10.0,
            height: 10.0,
            opacity: 1.0,
            xform: Xform2D::identity(),
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        let stroke_id = ItemId::new();
        session.doc.layers[0].items.push(InkItem::Stroke(InkStroke {
            id: stroke_id,
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.rebuild_runtime();
        session.doc.runtime.sel_items.insert(img_id);
        let bundle = session.copy_sel().unwrap();
        assert_eq!(bundle.records.len(), 2);
        assert_eq!(bundle.records[0].item.id(), img_id);
        assert_eq!(bundle.records[1].item.id(), stroke_id);
    }

    #[test]
    fn test_editing_07_copy_parent_child_explicit_non_duplication() {
        let mut session = InkSession::new(800.0, 600.0);
        let img_id = ItemId::new();
        session.doc.layers[0].items.push(InkItem::Image(InkImage {
            id: img_id,
            asset_id: AssetId::new(),
            width: 10.0,
            height: 10.0,
            opacity: 1.0,
            xform: Xform2D::identity(),
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        let stroke_id = ItemId::new();
        session.doc.layers[0].items.push(InkItem::Stroke(InkStroke {
            id: stroke_id,
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.rebuild_runtime();
        session.doc.runtime.sel_items.insert(img_id);
        session.doc.runtime.sel_items.insert(stroke_id);
        let bundle = session.copy_sel().unwrap();
        assert_eq!(bundle.records.len(), 2);
    }

    #[test]
    fn test_editing_08_copy_required_assets() {
        let mut session = InkSession::new(800.0, 600.0);
        let img_id = ItemId::new();
        let asset_id = AssetId::new();
        let asset = ImageAsset {
            id: asset_id,
            mime: "image/png".to_string(),
            width_px: 50,
            height_px: 50,
            bytes: vec![1, 2],
        };
        session.doc.assets.push(asset.clone());
        session.doc.layers[0].items.push(InkItem::Image(InkImage {
            id: img_id,
            asset_id,
            width: 10.0,
            height: 10.0,
            opacity: 1.0,
            xform: Xform2D::identity(),
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.rebuild_runtime();
        session.doc.runtime.sel_items.insert(img_id);
        let bundle = session.copy_sel().unwrap();
        assert_eq!(bundle.assets.len(), 1);
        assert_eq!(bundle.assets[0].id, asset_id);
    }

    #[test]
    fn test_editing_09_copy_source_origin_calculation() {
        let mut session = InkSession::new(800.0, 600.0);
        let s = InkStroke {
            id: ItemId::new(),
            parent_id: None,
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(50.0, 100.0, 60.0, 120.0),
            world_bbox: BBox::new(50.0, 100.0, 60.0, 120.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        };
        session.doc.layers[0].items.push(InkItem::Stroke(s));
        session.doc.rebuild_runtime();
        session
            .doc
            .runtime
            .sel_items
            .insert(session.doc.layers[0].items[0].id());
        let bundle = session.copy_sel().unwrap();
        assert_eq!(bundle.source_origin.x, 50.0);
        assert_eq!(bundle.source_origin.y, 100.0);
    }

    #[test]
    fn test_editing_10_copy_source_indices() {
        let mut session = InkSession::new(800.0, 600.0);
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        session.doc.layers[0].items.push(make_s(id1));
        session.doc.layers[0].items.push(make_s(id2));
        session.doc.rebuild_runtime();
        session.doc.runtime.sel_items.insert(id2);
        let bundle = session.copy_sel().unwrap();
        assert_eq!(bundle.records[0].source_idx, 1);
    }

    #[test]
    fn test_editing_11_standalone_bake_detaches_parent() {
        let mut session = InkSession::new(800.0, 600.0);
        let img_id = ItemId::new();
        let stroke_id = ItemId::new();
        session.doc.layers[0].items.push(InkItem::Image(InkImage {
            id: img_id,
            asset_id: AssetId::new(),
            width: 10.0,
            height: 10.0,
            opacity: 1.0,
            xform: Xform2D::translate(10.0, 20.0),
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: BBox::new(10.0, 20.0, 20.0, 30.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.layers[0].items.push(InkItem::Stroke(InkStroke {
            id: stroke_id,
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            world_bbox: BBox::new(10.0, 20.0, 11.0, 21.0),
            xform: Xform2D::translate(5.0, 5.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.rebuild_runtime();
        session.doc.runtime.sel_items.insert(stroke_id);
        let bundle = session.copy_sel().unwrap();
        if let InkItem::Stroke(s) = &bundle.records[0].item {
            assert_eq!(s.parent_id, None);
        } else {
            panic!();
        }
    }

    #[test]
    fn test_editing_12_standalone_bake_world_translation() {
        let mut session = InkSession::new(800.0, 600.0);
        let img_id = ItemId::new();
        let stroke_id = ItemId::new();
        session.doc.layers[0].items.push(InkItem::Image(InkImage {
            id: img_id,
            asset_id: AssetId::new(),
            width: 10.0,
            height: 10.0,
            opacity: 1.0,
            xform: Xform2D::translate(10.0, 20.0),
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: BBox::new(10.0, 20.0, 20.0, 30.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.layers[0].items.push(InkItem::Stroke(InkStroke {
            id: stroke_id,
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            world_bbox: BBox::new(10.0, 20.0, 11.0, 21.0),
            xform: Xform2D::translate(5.0, 5.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.rebuild_runtime();
        session.doc.runtime.sel_items.insert(stroke_id);
        let bundle = session.copy_sel().unwrap();
        if let InkItem::Stroke(s) = &bundle.records[0].item {
            assert_eq!(s.xform.tx, 15.0);
            assert_eq!(s.xform.ty, 25.0);
        } else {
            panic!();
        }
    }

    #[test]
    fn test_editing_13_standalone_bake_preserves_draw_order() {
        let mut session = InkSession::new(800.0, 600.0);
        let img_id = ItemId::new();
        let stroke_id = ItemId::new();
        session.doc.layers[0].items.push(InkItem::Image(InkImage {
            id: img_id,
            asset_id: AssetId::new(),
            width: 10.0,
            height: 10.0,
            opacity: 1.0,
            xform: Xform2D::identity(),
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.layers[0].items.push(InkItem::Stroke(InkStroke {
            id: stroke_id,
            parent_id: Some(img_id),
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.rebuild_runtime();
        session.doc.runtime.sel_items.insert(stroke_id);
        session.doc.runtime.sel_items.insert(img_id);
        let bundle = session.copy_sel().unwrap();
        assert_eq!(bundle.records[0].item.id(), img_id);
        assert_eq!(bundle.records[1].item.id(), stroke_id);
    }

    #[test]
    fn test_editing_14_standalone_bake_no_parent_reference() {
        let mut session = InkSession::new(800.0, 600.0);
        let stroke_id = ItemId::new();
        session.doc.layers[0].items.push(InkItem::Stroke(InkStroke {
            id: stroke_id,
            parent_id: None,
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.rebuild_runtime();
        session.doc.runtime.sel_items.insert(stroke_id);
        let bundle = session.copy_sel().unwrap();
        if let InkItem::Stroke(s) = &bundle.records[0].item {
            assert_eq!(s.parent_id, None);
        } else {
            panic!();
        }
    }

    #[test]
    fn test_editing_15_standalone_bake_relative_coordinates() {
        let mut session = InkSession::new(800.0, 600.0);
        let stroke_id = ItemId::new();
        session.doc.layers[0].items.push(InkItem::Stroke(InkStroke {
            id: stroke_id,
            parent_id: None,
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(10.0, 10.0, 20.0, 20.0),
            world_bbox: BBox::new(10.0, 10.0, 20.0, 20.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        }));
        session.doc.rebuild_runtime();
        session.doc.runtime.sel_items.insert(stroke_id);
        let bundle = session.copy_sel().unwrap();
        assert_eq!(bundle.source_origin.x, 10.0);
    }

    #[test]
    fn test_editing_16_paste_remaps_ids() {
        let mut session = InkSession::new(800.0, 600.0);
        let stroke_id = ItemId::new();
        let rec = ClipboardItemRecord {
            source_layer_id: session.doc.layers[0].id,
            source_layer_rank: 0,
            source_idx: 0,
            item: InkItem::Stroke(InkStroke {
                id: stroke_id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        };
        let bundle = ClipboardBundle {
            records: vec![rec],
            assets: vec![],
            source_origin: Point2::new(0.0, 0.0),
        };
        let pasted = session.paste_sel(&bundle, Xform2D::identity());
        assert_ne!(pasted[0], stroke_id);
    }

    #[test]
    fn test_editing_17_paste_target_layer_preservation() {
        let mut session = InkSession::new(800.0, 600.0);
        let rec = ClipboardItemRecord {
            source_layer_id: session.doc.layers[0].id,
            source_layer_rank: 0,
            source_idx: 0,
            item: InkItem::Stroke(InkStroke {
                id: ItemId::new(),
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        };
        let bundle = ClipboardBundle {
            records: vec![rec],
            assets: vec![],
            source_origin: Point2::new(0.0, 0.0),
        };
        session.paste_sel(&bundle, Xform2D::identity());
        assert_eq!(session.doc.layers[0].items.len(), 1);
    }

    #[test]
    fn test_editing_18_paste_target_layer_fallback() {
        let mut session = InkSession::new(800.0, 600.0);
        let rec = ClipboardItemRecord {
            source_layer_id: LayerId::new(), // non-existent
            source_layer_rank: 99,
            source_idx: 0,
            item: InkItem::Stroke(InkStroke {
                id: ItemId::new(),
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        };
        let bundle = ClipboardBundle {
            records: vec![rec],
            assets: vec![],
            source_origin: Point2::new(0.0, 0.0),
        };
        session.paste_sel(&bundle, Xform2D::identity());
        assert_eq!(session.doc.layers[0].items.len(), 1); // fallback to Layer 1
    }

    #[test]
    fn test_editing_19_paste_selection_to_roots() {
        let mut session = InkSession::new(800.0, 600.0);
        let img_id = ItemId::new();
        let stroke_id = ItemId::new();
        let r1 = ClipboardItemRecord {
            source_layer_id: session.doc.layers[0].id,
            source_layer_rank: 0,
            source_idx: 0,
            item: InkItem::Image(InkImage {
                id: img_id,
                asset_id: AssetId::new(),
                width: 10.0,
                height: 10.0,
                opacity: 1.0,
                xform: Xform2D::identity(),
                local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        };
        let r2 = ClipboardItemRecord {
            source_layer_id: session.doc.layers[0].id,
            source_layer_rank: 0,
            source_idx: 1,
            item: InkItem::Stroke(InkStroke {
                id: stroke_id,
                parent_id: Some(img_id),
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        };
        let bundle = ClipboardBundle {
            records: vec![r1, r2],
            assets: vec![],
            source_origin: Point2::new(0.0, 0.0),
        };
        session.paste_sel(&bundle, Xform2D::identity());
        assert_eq!(session.doc.runtime.sel_items.len(), 1); // Only image (root) selected
    }

    #[test]
    fn test_editing_20_paste_single_transaction_undo() {
        let mut session = InkSession::new(800.0, 600.0);
        let rec = ClipboardItemRecord {
            source_layer_id: session.doc.layers[0].id,
            source_layer_rank: 0,
            source_idx: 0,
            item: InkItem::Stroke(InkStroke {
                id: ItemId::new(),
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            }),
        };
        let bundle = ClipboardBundle {
            records: vec![rec],
            assets: vec![],
            source_origin: Point2::new(0.0, 0.0),
        };
        session.paste_sel(&bundle, Xform2D::identity());
        assert_eq!(session.doc.layers[0].items.len(), 1);
        session.undo();
        assert_eq!(session.doc.layers[0].items.len(), 0);
    }

    #[test]
    fn test_editing_21_asset_properties_match() {
        let mut session = InkSession::new(800.0, 600.0);
        let asset_id = AssetId::new();
        session.doc.assets.push(ImageAsset {
            id: asset_id,
            mime: "image/png".to_string(),
            width_px: 10,
            height_px: 10,
            bytes: vec![1, 2],
        });
        let bundle = ClipboardBundle {
            records: vec![],
            assets: vec![ImageAsset {
                id: asset_id,
                mime: "image/png".to_string(),
                width_px: 10,
                height_px: 10,
                bytes: vec![1, 2],
            }],
            source_origin: Point2::new(0.0, 0.0),
        };
        session.paste_sel(&bundle, Xform2D::identity());
        assert_eq!(session.doc.assets.len(), 1);
        assert!(session.doc.assets.iter().any(|a| a.id == asset_id));
    }

    #[test]
    fn test_editing_22_asset_mime_mismatch() {
        let mut session = InkSession::new(800.0, 600.0);
        let asset_id = AssetId::new();
        session.doc.assets.push(ImageAsset {
            id: asset_id,
            mime: "image/png".to_string(),
            width_px: 10,
            height_px: 10,
            bytes: vec![1, 2],
        });
        let bundle = ClipboardBundle {
            records: vec![],
            assets: vec![ImageAsset {
                id: asset_id,
                mime: "image/jpeg".to_string(), // Mismatch
                width_px: 10,
                height_px: 10,
                bytes: vec![1, 2],
            }],
            source_origin: Point2::new(0.0, 0.0),
        };
        session.paste_sel(&bundle, Xform2D::identity());
        assert_eq!(session.doc.assets.len(), 2);
    }

    #[test]
    fn test_editing_23_asset_dimensions_mismatch() {
        let mut session = InkSession::new(800.0, 600.0);
        let asset_id = AssetId::new();
        session.doc.assets.push(ImageAsset {
            id: asset_id,
            mime: "image/png".to_string(),
            width_px: 10,
            height_px: 10,
            bytes: vec![1, 2],
        });
        let bundle = ClipboardBundle {
            records: vec![],
            assets: vec![ImageAsset {
                id: asset_id,
                mime: "image/png".to_string(),
                width_px: 20, // Mismatch
                height_px: 10,
                bytes: vec![1, 2],
            }],
            source_origin: Point2::new(0.0, 0.0),
        };
        session.paste_sel(&bundle, Xform2D::identity());
        assert_eq!(session.doc.assets.len(), 2);
    }

    #[test]
    fn test_editing_24_asset_bytes_mismatch() {
        let mut session = InkSession::new(800.0, 600.0);
        let asset_id = AssetId::new();
        session.doc.assets.push(ImageAsset {
            id: asset_id,
            mime: "image/png".to_string(),
            width_px: 10,
            height_px: 10,
            bytes: vec![1, 2],
        });
        let bundle = ClipboardBundle {
            records: vec![],
            assets: vec![ImageAsset {
                id: asset_id,
                mime: "image/png".to_string(),
                width_px: 10,
                height_px: 10,
                bytes: vec![1, 3], // Mismatch
            }],
            source_origin: Point2::new(0.0, 0.0),
        };
        session.paste_sel(&bundle, Xform2D::identity());
        assert_eq!(session.doc.assets.len(), 2);
    }

    #[test]
    fn test_editing_25_asset_missing_adds_new() {
        let mut session = InkSession::new(800.0, 600.0);
        let asset_id = AssetId::new();
        let bundle = ClipboardBundle {
            records: vec![],
            assets: vec![ImageAsset {
                id: asset_id,
                mime: "image/png".to_string(),
                width_px: 10,
                height_px: 10,
                bytes: vec![1, 2],
            }],
            source_origin: Point2::new(0.0, 0.0),
        };
        session.paste_sel(&bundle, Xform2D::identity());
        assert_eq!(session.doc.assets.len(), 1);
        assert!(session.doc.assets.iter().any(|a| a.id == asset_id));
    }

    #[test]
    fn test_editing_26_cut_returns_copied_bundle() {
        let mut session = InkSession::new(800.0, 600.0);
        let s = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        session.doc.runtime.sel_items.insert(s);
        let bundle = session.cut_sel().unwrap();
        assert_eq!(bundle.records.len(), 1);
        assert_eq!(bundle.records[0].item.id(), s);
    }

    #[test]
    fn test_editing_27_cut_removes_from_doc() {
        let mut session = InkSession::new(800.0, 600.0);
        let s = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        session.doc.runtime.sel_items.insert(s);
        session.cut_sel();
        assert_eq!(session.doc.layers[0].items.len(), 0);
    }

    #[test]
    fn test_editing_28_cut_clears_selection() {
        let mut session = InkSession::new(800.0, 600.0);
        let s = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        session.doc.runtime.sel_items.insert(s);
        session.cut_sel();
        assert!(session.doc.runtime.sel_items.is_empty());
    }

    #[test]
    fn test_editing_29_cut_leaves_unselected_intact() {
        let mut session = InkSession::new(800.0, 600.0);
        let s1 = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        let s2 = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(2.0, 2.0)]);
        session.doc.runtime.sel_items.insert(s1);
        session.cut_sel();
        assert_eq!(session.doc.layers[0].items.len(), 1);
        assert_eq!(session.doc.layers[0].items[0].id(), s2);
    }

    #[test]
    fn test_editing_30_nudge_creates_no_tx_initially() {
        let mut session = InkSession::new(800.0, 600.0);
        let s = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        session.doc.runtime.sel_items.insert(s);
        assert_eq!(session.undo_redo.undo_stack.len(), 0);
    }

    #[test]
    fn test_editing_31_nudge_preserves_intermediate_state() {
        let mut session = InkSession::new(800.0, 600.0);
        let s = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        session.doc.runtime.sel_items.insert(s);
        let item = session.doc.get_item_mut(s).unwrap();
        item.set_xform(Xform2D::translate(5.0, 0.0));
        assert_eq!(session.doc.get_item(s).unwrap().xform().tx, 5.0);
    }

    #[test]
    fn test_editing_32_nudge_undo_restores_original() {
        let mut session = InkSession::new(800.0, 600.0);
        let s = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        session.doc.runtime.sel_items.insert(s);

        let mut before_xforms = std::collections::HashMap::new();
        before_xforms.insert(s, Xform2D::identity());
        let nudge_state = TestNudgeState {
            active_cmd: Command::NudgeRight,
            before_xforms,
        };

        session
            .doc
            .get_item_mut(s)
            .unwrap()
            .set_xform(Xform2D::translate(1.0, 0.0));

        let mut state_opt = Some(nudge_state);
        test_commit_nudge(&mut session, &mut state_opt);

        assert_eq!(session.doc.get_item(s).unwrap().xform().tx, 1.0);
        session.undo();
        assert_eq!(session.doc.get_item(s).unwrap().xform().tx, 0.0);
    }

    #[test]
    fn test_editing_33_nudge_redo_applies_final() {
        let mut session = InkSession::new(800.0, 600.0);
        let s = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        session.doc.runtime.sel_items.insert(s);

        let mut before_xforms = std::collections::HashMap::new();
        before_xforms.insert(s, Xform2D::identity());
        let nudge_state = TestNudgeState {
            active_cmd: Command::NudgeRight,
            before_xforms,
        };

        session
            .doc
            .get_item_mut(s)
            .unwrap()
            .set_xform(Xform2D::translate(2.0, 0.0));

        let mut state_opt = Some(nudge_state);
        test_commit_nudge(&mut session, &mut state_opt);

        session.undo();
        session.redo();
        assert_eq!(session.doc.get_item(s).unwrap().xform().tx, 2.0);
    }

    #[test]
    fn test_editing_34_nudge_no_tx_on_zero_diff() {
        let mut session = InkSession::new(800.0, 600.0);
        let s = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(1.0, 1.0)]);
        session.doc.runtime.sel_items.insert(s);

        let mut before_xforms = std::collections::HashMap::new();
        before_xforms.insert(s, Xform2D::identity());
        let nudge_state = TestNudgeState {
            active_cmd: Command::NudgeRight,
            before_xforms,
        };

        let mut state_opt = Some(nudge_state);
        test_commit_nudge(&mut session, &mut state_opt); // No actual transform was applied

        assert_eq!(session.undo_redo.undo_stack.len(), 0);
    }

    #[test]
    fn test_editing_35_z_order_bring_forward() {
        let mut session = InkSession::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let id3 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        session.doc.layers[0].items.push(make_s(id1));
        session.doc.layers[0].items.push(make_s(id2));
        session.doc.layers[0].items.push(make_s(id3));

        session.doc.runtime.sel_items.insert(id2);
        session.z_order_sel(ZOrderCmd::BringForward);
        let ids: Vec<ItemId> = session.doc.layers[0]
            .items
            .iter()
            .map(|it| it.id())
            .collect();
        assert_eq!(ids, vec![id1, id3, id2]);
    }

    #[test]
    fn test_editing_36_z_order_send_backward() {
        let mut session = InkSession::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let id3 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        session.doc.layers[0].items.push(make_s(id1));
        session.doc.layers[0].items.push(make_s(id2));
        session.doc.layers[0].items.push(make_s(id3));

        session.doc.runtime.sel_items.insert(id2);
        session.z_order_sel(ZOrderCmd::SendBackward);
        let ids: Vec<ItemId> = session.doc.layers[0]
            .items
            .iter()
            .map(|it| it.id())
            .collect();
        assert_eq!(ids, vec![id2, id1, id3]);
    }

    #[test]
    fn test_editing_37_z_order_bring_to_front() {
        let mut session = InkSession::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let id3 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        session.doc.layers[0].items.push(make_s(id1));
        session.doc.layers[0].items.push(make_s(id2));
        session.doc.layers[0].items.push(make_s(id3));

        session.doc.runtime.sel_items.insert(id1);
        session.z_order_sel(ZOrderCmd::BringToFront);
        let ids: Vec<ItemId> = session.doc.layers[0]
            .items
            .iter()
            .map(|it| it.id())
            .collect();
        assert_eq!(ids, vec![id2, id3, id1]);
    }

    #[test]
    fn test_editing_38_z_order_send_to_back() {
        let mut session = InkSession::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let id3 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        session.doc.layers[0].items.push(make_s(id1));
        session.doc.layers[0].items.push(make_s(id2));
        session.doc.layers[0].items.push(make_s(id3));

        session.doc.runtime.sel_items.insert(id3);
        session.z_order_sel(ZOrderCmd::SendToBack);
        let ids: Vec<ItemId> = session.doc.layers[0]
            .items
            .iter()
            .map(|it| it.id())
            .collect();
        assert_eq!(ids, vec![id3, id1, id2]);
    }

    #[test]
    fn test_editing_39_z_order_undo_redo() {
        let mut session = InkSession::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        session.doc.layers[0].items.push(make_s(id1));
        session.doc.layers[0].items.push(make_s(id2));

        session.doc.runtime.sel_items.insert(id1);
        session.z_order_sel(ZOrderCmd::BringForward);
        let ids: Vec<ItemId> = session.doc.layers[0]
            .items
            .iter()
            .map(|it| it.id())
            .collect();
        assert_eq!(ids, vec![id2, id1]);

        session.undo();
        let ids_undo: Vec<ItemId> = session.doc.layers[0]
            .items
            .iter()
            .map(|it| it.id())
            .collect();
        assert_eq!(ids_undo, vec![id1, id2]);
    }

    #[test]
    fn test_editing_40_z_order_no_change_when_already_at_boundary() {
        let mut session = InkSession::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        session.doc.layers[0].items.push(make_s(id1));
        session.doc.layers[0].items.push(make_s(id2));

        session.doc.runtime.sel_items.insert(id2);
        session.z_order_sel(ZOrderCmd::BringForward); // Already at front
        assert_eq!(session.undo_redo.undo_stack.len(), 0);
    }

    #[test]
    fn test_editing_41_z_order_multiple_selected_preserves_relative_order() {
        let mut session = InkSession::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let id3 = ItemId::new();
        let id4 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        session.doc.layers[0].items.push(make_s(id1));
        session.doc.layers[0].items.push(make_s(id2));
        session.doc.layers[0].items.push(make_s(id3));
        session.doc.layers[0].items.push(make_s(id4));

        session.doc.runtime.sel_items.insert(id1);
        session.doc.runtime.sel_items.insert(id3);
        session.z_order_sel(ZOrderCmd::BringToFront);

        let ids: Vec<ItemId> = session.doc.layers[0]
            .items
            .iter()
            .map(|it| it.id())
            .collect();
        assert_eq!(ids, vec![id2, id4, id1, id3]); // id1 and id3 moved to front, keeping relative order
    }

    #[test]
    fn test_editing_42_reorder_rejects_missing_element() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        let lid = doc.active_layer_id;
        doc.layers[0].items.push(make_s(id1));
        doc.layers[0].items.push(make_s(id2));

        let invalid = vec![id1]; // id2 missing!
        assert!(doc.reorder_items_in_layer(lid, &invalid).is_err());
    }

    #[test]
    fn test_editing_43_reorder_rejects_duplicate_element() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        let lid = doc.active_layer_id;
        doc.layers[0].items.push(make_s(id1));
        doc.layers[0].items.push(make_s(id2));

        let invalid = vec![id1, id1]; // duplicate id1!
        assert!(doc.reorder_items_in_layer(lid, &invalid).is_err());
    }

    #[test]
    fn test_editing_44_reorder_rejects_spurious_element() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let id1 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        let lid = doc.active_layer_id;
        doc.layers[0].items.push(make_s(id1));

        let invalid = vec![id1, ItemId::new()]; // spurious new ID!
        assert!(doc.reorder_items_in_layer(lid, &invalid).is_err());
    }

    #[test]
    fn test_editing_45_reorder_accepts_valid() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let id1 = ItemId::new();
        let id2 = ItemId::new();
        let make_s = |id| {
            InkItem::Stroke(InkStroke {
                id,
                parent_id: None,
                brush: Brush::default_pen(),
                raw_pts: vec![],
                pts: vec![],
                local_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                world_bbox: BBox::new(0.0, 0.0, 1.0, 1.0),
                xform: Xform2D::identity(),
                created_at_ms: 0,
                updated_at_ms: 0,
                geom_rev: 0,
            })
        };
        let lid = doc.active_layer_id;
        doc.layers[0].items.push(make_s(id1));
        doc.layers[0].items.push(make_s(id2));

        let valid = vec![id2, id1];
        assert!(doc.reorder_items_in_layer(lid, &valid).is_ok());
    }

    #[test]
    fn test_editing_46_reorder_rejects_invalid_layer() {
        let mut doc = InkDoc::new(800.0, 600.0);
        assert!(doc.reorder_items_in_layer(LayerId::new(), &[]).is_err());
    }

    #[test]
    fn test_editing_47_z_order_reorder_empty_layer_is_ok() {
        let mut doc = InkDoc::new(800.0, 600.0);
        let lid = doc.active_layer_id;
        assert!(doc.reorder_items_in_layer(lid, &[]).is_ok());
    }

    #[test]
    fn test_editing_48_duplicate_selection_empty_does_nothing() {
        let mut session = InkSession::new(800.0, 600.0);
        session.duplicate_sel();
        assert_eq!(session.undo_redo.undo_stack.len(), 0);
    }

    #[test]
    fn test_editing_49_delete_selection_empty_does_nothing() {
        let mut session = InkSession::new(800.0, 600.0);
        session.delete_sel();
        assert_eq!(session.undo_redo.undo_stack.len(), 0);
    }

    #[test]
    fn test_editing_50_set_stroke_brushes_transaction() {
        let mut session = InkSession::new(800.0, 600.0);
        let id1 = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(10.0, 20.0)]);
        let id2 = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(30.0, 40.0)]);

        let brush1 = session.doc.get_stroke(id1).unwrap().brush.clone();
        let brush2 = session.doc.get_stroke(id2).unwrap().brush.clone();

        let mut new_brush1 = brush1.clone();
        new_brush1.base_w = 12.5;
        let mut new_brush2 = brush2.clone();
        new_brush2.color.r = 255;

        let tx = InkTx::new("style change").push(InkOp::SetStrokeBrushes {
            stroke_ids: vec![id1, id2],
            before: vec![brush1.clone(), brush2.clone()],
            after: vec![new_brush1.clone(), new_brush2.clone()],
        });
        session.do_tx(tx);

        assert_eq!(session.doc.get_stroke(id1).unwrap().brush.base_w, 12.5);
        assert_eq!(session.doc.get_stroke(id2).unwrap().brush.color.r, 255);

        session.undo();
        assert_eq!(
            session.doc.get_stroke(id1).unwrap().brush.base_w,
            brush1.base_w
        );
        assert_eq!(
            session.doc.get_stroke(id2).unwrap().brush.color.r,
            brush2.color.r
        );

        session.redo();
        assert_eq!(session.doc.get_stroke(id1).unwrap().brush.base_w, 12.5);
        assert_eq!(session.doc.get_stroke(id2).unwrap().brush.color.r, 255);
    }

    #[test]
    fn test_editing_51_single_selected_image_root() {
        let mut doc = InkDoc::new(800.0, 600.0);

        let img_id1 = ItemId::new();
        let img1 = InkItem::Image(InkImage {
            id: img_id1,
            asset_id: AssetId::new(),
            width: 100.0,
            height: 100.0,
            opacity: 1.0,
            xform: Xform2D::identity(),
            local_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            world_bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        });
        doc.add_item(doc.active_layer_id, img1);

        let stroke_id1 = StrokeId::new();
        let stroke1 = InkItem::Stroke(InkStroke {
            id: stroke_id1,
            parent_id: Some(img_id1),
            brush: Brush::default_pen(),
            raw_pts: vec![],
            pts: vec![],
            local_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            world_bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
            xform: Xform2D::identity(),
            created_at_ms: 0,
            updated_at_ms: 0,
            geom_rev: 0,
        });
        doc.add_item(doc.active_layer_id, stroke1);

        assert_eq!(doc.single_selected_image_root(), None);

        doc.runtime.sel_items.insert(stroke_id1);
        assert_eq!(doc.single_selected_image_root(), None);

        doc.runtime.sel_items.clear();
        doc.runtime.sel_items.insert(img_id1);
        assert_eq!(doc.single_selected_image_root(), Some(img_id1));

        doc.runtime.sel_items.insert(stroke_id1);
        assert_eq!(doc.single_selected_image_root(), Some(img_id1));
    }

    #[test]
    fn test_editing_52_is_z_order_enabled_check() {
        let mut session = InkSession::new(800.0, 600.0);
        let id1 = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(10.0, 20.0)]);
        let id2 = make_stroke_in_doc(&mut session.doc, vec![make_ink_point(30.0, 40.0)]);

        assert!(!session.is_z_order_enabled(ZOrderCmd::BringForward));

        session.doc.runtime.sel_items.insert(id1);
        assert!(session.is_z_order_enabled(ZOrderCmd::BringForward));
        assert!(session.is_z_order_enabled(ZOrderCmd::BringToFront));
        assert!(!session.is_z_order_enabled(ZOrderCmd::SendBackward));
        assert!(!session.is_z_order_enabled(ZOrderCmd::SendToBack));

        session.doc.runtime.sel_items.clear();
        session.doc.runtime.sel_items.insert(id2);
        assert!(!session.is_z_order_enabled(ZOrderCmd::BringForward));
        assert!(!session.is_z_order_enabled(ZOrderCmd::BringToFront));
        assert!(session.is_z_order_enabled(ZOrderCmd::SendBackward));
        assert!(session.is_z_order_enabled(ZOrderCmd::SendToBack));
    }
}
