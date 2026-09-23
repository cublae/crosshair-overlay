use cairo::{Context, LineCap};

use crate::{Profile, Rgba, Shape};

/// Draws `profile` centred on (`cx`, `cy`) in logical pixels.
///
/// The shape is stroked twice — outline first, then the crosshair colour on
/// top — so the outline reads as a border instead of covering the shape.
pub fn draw(cr: &Context, profile: &Profile, cx: f64, cy: f64) {
    let _ = cr.save();
    cr.translate(cx + profile.offset_x, cy + profile.offset_y);
    if profile.rotation != 0.0 {
        cr.rotate(profile.rotation.to_radians());
    }
    cr.set_line_cap(LineCap::Butt);

    if profile.outline.enabled && profile.outline.width > 0.0 {
        let width = profile.thickness + profile.outline.width * 2.0;
        stroke_shape(cr, profile, width, profile.outline.color);
    }
    stroke_shape(cr, profile, profile.thickness, profile.color);
    draw_dot(cr, profile);

    let _ = cr.restore();
}

fn set_color(cr: &Context, color: Rgba, opacity: f64) {
    let c = color.faded(opacity);
    cr.set_source_rgba(c.r, c.g, c.b, c.a);
}

fn stroke_shape(cr: &Context, profile: &Profile, width: f64, color: Rgba) {
    if width <= 0.0 || profile.length <= 0.0 {
        return;
    }
    set_color(cr, color, profile.opacity);
    cr.set_line_width(width);
    path_shape(cr, profile);
    let _ = cr.stroke();
}

fn path_shape(cr: &Context, p: &Profile) {
    let (gap, len) = (p.gap, p.length);
    match p.shape {
        Shape::Cross => {
            for (dx, dy) in [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
                arm(cr, dx, dy, gap, len);
            }
        }
        Shape::T => {
            for (dx, dy) in [(0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
                arm(cr, dx, dy, gap, len);
            }
        }
        Shape::X => {
            let d = std::f64::consts::FRAC_1_SQRT_2;
            for (dx, dy) in [(-d, -d), (d, -d), (-d, d), (d, d)] {
                arm(cr, dx, dy, gap, len);
            }
        }
        Shape::Circle => {
            cr.new_sub_path();
            cr.arc(0.0, 0.0, len, 0.0, std::f64::consts::TAU);
        }
        Shape::Square => {
            cr.rectangle(-len, -len, len * 2.0, len * 2.0);
        }
        Shape::Dot => {}
    }
}

/// One arm pointing along the unit vector (`dx`, `dy`).
fn arm(cr: &Context, dx: f64, dy: f64, gap: f64, len: f64) {
    cr.move_to(dx * gap, dy * gap);
    cr.line_to(dx * (gap + len), dy * (gap + len));
}

fn draw_dot(cr: &Context, p: &Profile) {
    if !p.dot.enabled || p.dot.size <= 0.0 {
        return;
    }
    if p.outline.enabled && p.outline.width > 0.0 {
        set_color(cr, p.outline.color, p.opacity);
        cr.arc(
            0.0,
            0.0,
            p.dot.size + p.outline.width,
            0.0,
            std::f64::consts::TAU,
        );
        let _ = cr.fill();
    }
    set_color(cr, p.dot.color.unwrap_or(p.color), p.opacity);
    cr.arc(0.0, 0.0, p.dot.size, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();
}
