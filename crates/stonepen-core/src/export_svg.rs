use crate::doc::InkDoc;
use crate::session::InkError;
use crate::stroke::InkStroke;
use crate::xform::Xform2D;

pub fn export_svg(doc: &InkDoc) -> Result<String, InkError> {
    let mut out = String::new();
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
        doc.width, doc.height, doc.width, doc.height
    ));
    out.push('\n');
    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        out.push_str(&format!(
            r#"  <g id="layer-{}" opacity="{:.3}">"#,
            layer.id.0, layer.opacity
        ));
        out.push('\n');
        for stroke in &layer.strokes {
            let path = stroke_to_svg_path(stroke);
            let color = &stroke.brush.color;
            let opacity = stroke.brush.opacity;
            let stroke_w = stroke.brush.base_w;
            let (r, g, b, a) = (color.r, color.g, color.b, color.a as f32 / 255.0);
            let xf = stroke.xform;
            let transform = xform_to_svg(xf);
            out.push_str(&format!(
                r#"    <path d="{}" stroke="rgba({},{},{},{:.3})" stroke-width="{}" fill="none" stroke-linecap="round" stroke-linejoin="round" opacity="{:.3}" transform="{}" />"#,
                path, r, g, b, a, stroke_w, opacity, transform
            ));
            out.push('\n');
        }
        out.push_str("  </g>\n");
    }
    out.push_str("</svg>\n");
    Ok(out)
}

fn stroke_to_svg_path(stroke: &InkStroke) -> String {
    let pts = &stroke.pts;
    if pts.is_empty() {
        return String::new();
    }
    let mut d = format!("M {:.3} {:.3}", pts[0].x, pts[0].y);
    for p in pts.iter().skip(1) {
        d.push_str(&format!(" L {:.3} {:.3}", p.x, p.y));
    }
    d
}

fn xform_to_svg(xf: Xform2D) -> String {
    format!(
        "matrix({:.6},{:.6},{:.6},{:.6},{:.6},{:.6})",
        xf.a, xf.b, xf.c, xf.d, xf.tx, xf.ty
    )
}
